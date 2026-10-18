//! gRPC client to the engine's `EngineService` (live spec §6).
//!
//! The engine is the **server**; the analyzer is the **client**. This polls `GetActiveSymbols` for
//! activation and pushes each emitted signal via `Publish`, retrying until the engine acks. `seq`
//! and the pending/retry buffer are in-memory only for now (persistence is a follow-up).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use tonic::transport::Channel;

use crate::harness::publish::price_f64;
use crate::live::{to_contract, to_kucoin, SymbolState};
use crate::strategy::types::{Direction, Signal};

#[allow(clippy::result_large_err)]
mod pb {
    tonic::include_proto!("analyzer");
}
pub use pb::{
    engine_service_client::EngineServiceClient, Ack, Empty, Signal as ProtoSignal, Symbols,
};

/// Signal time-to-live: an unacked signal older than this is dropped (live spec §7).
const EXPIRY_MS: i64 = 60_000;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Thin wrapper over the generated `EngineServiceClient`. Cheap to clone (the channel is
/// multiplexed and reconnects on demand).
#[derive(Clone)]
pub struct EngineClient {
    client: EngineServiceClient<Channel>,
}

impl EngineClient {
    /// Build a lazy client. Only a malformed endpoint is an error; the connection itself is
    /// established (and re-established) on the first call.
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let channel = Channel::from_shared(endpoint.to_string())
            .map_err(|e| format!("invalid ENGINE_ENDPOINT {endpoint:?}: {e}"))?
            .connect_lazy();
        Ok(Self {
            client: EngineServiceClient::new(channel),
        })
    }

    /// The engine's active ticker set (activation).
    #[allow(clippy::result_large_err)]
    pub async fn active_symbols(&mut self) -> Result<Vec<String>, tonic::Status> {
        Ok(self
            .client
            .get_active_symbols(Empty {})
            .await?
            .into_inner()
            .tickers)
    }

    /// Push one signal; returns the engine's ack.
    #[allow(clippy::result_large_err)]
    pub async fn publish(&mut self, signal: ProtoSignal) -> Result<Ack, tonic::Status> {
        Ok(self.client.publish(signal).await?.into_inner())
    }
}

/// Map a core signal to the wire message. `None` when a price cannot be represented as `f64`
/// (the caller must skip publishing rather than substitute a default — live spec §7).
pub fn to_proto_signal(s: &Signal, seq: u64, now_ms: i64) -> Option<ProtoSignal> {
    Some(ProtoSignal {
        ticker: to_contract(&s.symbol),
        action: match s.direction {
            Direction::Long => "BUY",
            Direction::Short => "SELL",
        }
        .to_string(),
        entry_price: price_f64(s.reference_entry_price)?,
        schema_version: s.schema_version,
        signal_id: s.signal_id.clone(),
        seq,
        expires_at_ms: now_ms + EXPIRY_MS,
        trigger_time_ms: s.triggered_at_ms,
        sl_price: price_f64(s.stop_loss)?,
        tp_price: price_f64(s.take_profit)?,
        zone_id: s.zone_id.clone(),
        reason: format!("supply_demand_4h rr={} mode={}", s.gross_rr, s.target_mode),
    })
}

/// Drain emitted signals, assign a monotonic `seq`, and push each to the engine until it is acked
/// (any `Ack` clears it) or it expires. A transport error retries with a fixed backoff.
pub async fn run_publisher(mut client: EngineClient, mut rx: mpsc::Receiver<Signal>) {
    let mut seq: u64 = 1;
    while let Some(sig) = rx.recv().await {
        let Some(proto) = to_proto_signal(&sig, seq, now_ms()) else {
            log::warn!("signal {} not representable as f64; dropped", sig.signal_id);
            continue;
        };
        seq += 1;
        loop {
            if proto.expires_at_ms != 0 && now_ms() > proto.expires_at_ms {
                log::warn!("signal {} expired before ack; dropped", proto.signal_id);
                break;
            }
            match client.publish(proto.clone()).await {
                Ok(ack) => {
                    log::info!(
                        "publish {} accepted={} ({})",
                        proto.signal_id,
                        ack.accepted,
                        ack.reason
                    );
                    break;
                }
                Err(e) => {
                    log::warn!("publish {} failed ({e}); retrying", proto.signal_id);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

/// Poll the engine's active symbol set and publish it to shared state (activation).
pub async fn run_activation(mut client: EngineClient, active: Arc<SymbolState>) {
    loop {
        match client.active_symbols().await {
            Ok(symbols) => {
                log::info!("engine active symbols: {symbols:?}");
                active.replace(symbols.into_iter().map(|s| to_kucoin(&s)));
            }
            Err(e) => log::warn!("GetActiveSymbols failed: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::TrendState;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn core_signal(direction: Direction) -> Signal {
        Signal {
            schema_version: 1,
            event_type: "strategy.entry_signal".into(),
            signal_id: "z:0".into(),
            strategy: "supply_demand_4h".into(),
            strategy_version: "0.1.0".into(),
            config_version: "x".into(),
            symbol: "XRP-USDT".into(),
            market: "margin".into(),
            direction,
            timeframe: "1min".into(),
            trend_state: TrendState::Bullish,
            zone_id: "z".into(),
            zone_low: dec("1.20"),
            zone_high: dec("1.30"),
            zone_height_atr: dec("0.4"),
            atr: dec("0.1"),
            trigger_price: dec("1.23"),
            reference_entry_price: dec("1.23"),
            stop_loss: dec("1.20"),
            take_profit: dec("1.30"),
            target_mode: "fixed_r".into(),
            target_r: dec("3.0"),
            min_gross_rr: dec("3.0"),
            gross_rr: dec("3.0"),
            structure_obstacle_price: None,
            obstacle_rr: None,
            triggered_at_ms: 900,
            candle_timestamp_ms: 0,
        }
    }

    #[test]
    fn maps_core_signal_to_wire_message() {
        let p = to_proto_signal(&core_signal(Direction::Long), 42, 1_000).unwrap();
        assert_eq!(p.ticker, "XRP/USDT");
        assert_eq!(p.action, "BUY");
        assert_eq!(p.entry_price, 1.23);
        assert_eq!(p.sl_price, 1.20);
        assert_eq!(p.tp_price, 1.30);
        assert_eq!(p.signal_id, "z:0");
        assert_eq!(p.zone_id, "z");
        assert_eq!(p.seq, 42);
        assert_eq!(p.trigger_time_ms, 900);
        assert_eq!(p.expires_at_ms, 1_000 + EXPIRY_MS);
    }

    #[test]
    fn short_maps_to_sell() {
        let p = to_proto_signal(&core_signal(Direction::Short), 1, 0).unwrap();
        assert_eq!(p.action, "SELL");
    }
}
