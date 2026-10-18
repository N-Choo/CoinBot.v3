//! `EngineService` gRPC server (live spec §6).
//!
//! The engine is the server; the analyzer is the client. The analyzer polls
//! [`EngineService::get_active_symbols`] for activation and pushes each signal via
//! [`EngineService::publish`]. Dedup is in-process only for now — a durable table is a
//! follow-up — but it must exist so a retried `publish` cannot double-execute a trade.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use kucoin::client::rest::KuCoinClient;
use share::db::contracts::{ContractFilter, Status as ContractStatus};
use share::engine_service_server::{EngineService, EngineServiceServer};
use share::models::signals::TradeSignal;
use share::{Ack, Empty, Signal as ProtoSignal, Symbols};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct EngineServer {
    db: Pool<Postgres>,
    kcc: KuCoinClient,
    seen: Arc<Mutex<HashSet<String>>>,
}

impl EngineServer {
    pub fn new(db: Pool<Postgres>, kcc: KuCoinClient) -> Self {
        Self {
            db,
            kcc,
            seen: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

#[tonic::async_trait]
impl EngineService for EngineServer {
    async fn get_active_symbols(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Symbols>, Status> {

        let tickers = ContractFilter::new()
            .with_status(ContractStatus::Active)
            .execute_tickers(&self.db)
            .await
            .map_err(|e| Status::internal(format!("active-ticker query failed: {e}")))?;

        Ok(Response::new(Symbols { tickers }))
    }

    async fn publish(&self, request: Request<ProtoSignal>) -> Result<Response<Ack>, Status> {
        let signal = request.into_inner();
        log::info!(
            "EngineService.Publish: {} {} seq={} id={}",
            signal.action,
            signal.ticker,
            signal.seq,
            signal.signal_id
        );

        if let Some(reason) = reject_reason(&signal, chrono::Utc::now().timestamp_millis()) {
            log::warn!("rejecting signal {}: {reason}", signal.signal_id);
            return Ok(Response::new(Ack {
                accepted: false,
                reason,
            }));
        }

        // Dedup Guard.
        if !self.seen.lock().unwrap().insert(signal.signal_id.clone()) {
            return Ok(Response::new(Ack {
                accepted: false,
                reason: "duplicate".to_string(),
            }));
        }

        let trade = to_trade_signal(&signal);
        if let Err(e) =
            crate::signals::signal_handler(&trade, self.db.clone(), self.kcc.clone()).await
        {
            // Unmark so a retry can re-attempt after a transient failure.
            self.seen.lock().unwrap().remove(&signal.signal_id);
            return Err(Status::internal(format!("signal handling failed: {e}")));
        }

        Ok(Response::new(Ack {
            accepted: true,
            reason: "handled".to_string(),
        }))
    }
}

/// Serve `EngineService` on `grpc_addr` until the process exits. A malformed address or a bind
/// failure is logged and returns rather than killing the rest of the engine.
pub async fn serve(server: EngineServer, grpc_addr: String) {
    let addr = match grpc_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            log::error!("invalid GRPC_ADDR {grpc_addr:?}: {e}");
            return;
        }
    };

    log::info!("EngineService listening gRPC on {addr}");
    if let Err(e) = tonic::transport::Server::builder()
        .add_service(EngineServiceServer::new(server))
        .serve(addr)
        .await
    {
        log::error!("EngineService server error: {e}");
    }
}

/// Reject a signal that must not be acted on; `None` means it may proceed. `expires_at_ms == 0`
/// means "no expiry".
fn reject_reason(signal: &ProtoSignal, now_ms: i64) -> Option<String> {
    if signal.expires_at_ms != 0 && signal.expires_at_ms < now_ms {
        return Some("expired".to_string());
    }
    None
}

/// Map the wire signal to the engine's internal `TradeSignal`. The proto dropped `confidence`, so
/// the engine's model gets the neutral `1.0` (the engine does not gate on confidence).
fn to_trade_signal(s: &ProtoSignal) -> TradeSignal {
    TradeSignal {
        ticker: s.ticker.clone(),
        action: s.action.clone(),
        confidence: 1.0,
        entry_price: s.entry_price,
        sl_price: s.sl_price,
        tp_price: s.tp_price,
        reason: s.reason.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProtoSignal {
        ProtoSignal {
            ticker: "XRP-USDT".into(),
            action: "BUY".into(),
            entry_price: 1.23,
            schema_version: 1,
            signal_id: "z1:0".into(),
            seq: 42,
            expires_at_ms: 1_000,
            trigger_time_ms: 900,
            sl_price: 1.20,
            tp_price: 1.30,
            zone_id: "z1".into(),
            reason: "rr=3".into(),
        }
    }

    #[test]
    fn maps_to_trade_signal_with_neutral_confidence() {
        let t = to_trade_signal(&sample());
        assert_eq!(t.ticker, "XRP-USDT");
        assert_eq!(t.action, "BUY");
        assert_eq!(t.confidence, 1.0);
        assert_eq!(t.entry_price, 1.23);
        assert_eq!(t.sl_price, 1.20);
        assert_eq!(t.tp_price, 1.30);
        assert_eq!(t.reason, "rr=3");
    }

    #[test]
    fn expired_signal_is_rejected_and_zero_means_no_expiry() {
        assert_eq!(reject_reason(&sample(), 1_001).as_deref(), Some("expired"));
        assert_eq!(reject_reason(&sample(), 1_000), None);
        let mut s = sample();
        s.expires_at_ms = 0;
        assert_eq!(reject_reason(&s, i64::MAX), None);
    }
}
