use crate::exchange::timeframes::Timeframe;
use crate::identity::signal_id;
use crate::strategy::types::{Direction, Signal};
use rust_decimal::prelude::ToPrimitive;
use std::time::Duration;
use tokio::sync::mpsc;
use tonic::transport::{Channel, Endpoint};

pub use crate::identity::{config_version, now_ms};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RPC_TIMEOUT: Duration = Duration::from_secs(10);

#[allow(clippy::result_large_err)]
mod pb {
    tonic::include_proto!("analyzer");
}
pub use pb::{
    engine_service_client::EngineServiceClient, Ack, Empty, Signal as ProtoSignal, Symbols,
};

#[derive(Clone)]
pub struct EngineClient {
    client: EngineServiceClient<Channel>,
}

impl EngineClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let channel = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| format!("invalid engine endpoint {endpoint:?}: {e}"))?
            .timeout(RPC_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .connect_lazy();
        Ok(Self {
            client: EngineServiceClient::new(channel),
        })
    }

    #[allow(clippy::result_large_err)]
    pub async fn active_symbols(&mut self) -> Result<Vec<String>, tonic::Status> {
        let response = tokio::time::timeout(RPC_TIMEOUT, self.client.get_active_symbols(Empty {}))
            .await
            .map_err(|_| {
                tonic::Status::deadline_exceeded(format!(
                    "GetActiveSymbols timed out after {RPC_TIMEOUT:?}"
                ))
            })??;
        Ok(response.into_inner().tickers)
    }

    #[allow(clippy::result_large_err)]
    pub async fn publish(&mut self, signal: ProtoSignal) -> Result<Ack, tonic::Status> {
        let response = tokio::time::timeout(RPC_TIMEOUT, self.client.publish(signal))
            .await
            .map_err(|_| {
                tonic::Status::deadline_exceeded(format!("Publish timed out after {RPC_TIMEOUT:?}"))
            })??;
        Ok(response.into_inner())
    }
}

/// Delivery-queue TTL for a signal after it is dequeued. This is not signal freshness; the
/// underlying candle signal may be older.
pub fn signal_ttl_ms(timeframe: Timeframe) -> i64 {
    (timeframe.millis() / 10).clamp(60_000, 3_600_000)
}

fn price_f64(value: rust_decimal::Decimal) -> Option<f64> {
    value.to_f64()
}

/// Map a core signal to the wire message.
///
/// `now_ms` is the dequeue time: `expires_at_ms` is a delivery-queue TTL stamped when the
/// publisher picks the signal up, not a claim about how fresh the underlying candle signal is.
pub fn to_proto_signal(
    signal: &Signal,
    seq: u64,
    now_ms: i64,
    ttl_ms: i64,
    config_version: &str,
    timeframe: Timeframe,
) -> Option<ProtoSignal> {
    Some(ProtoSignal {
        ticker: signal.ticker.replace('-', "/"),
        action: match signal.action {
            Direction::Long => "BUY",
            Direction::Short => "SELL",
        }
        .to_string(),
        entry_price: price_f64(signal.entry_price)?,
        schema_version: 1,
        signal_id: signal_id(signal, config_version),
        seq,
        expires_at_ms: now_ms + ttl_ms,
        trigger_time_ms: signal.signal_candle_open_time_ms + timeframe.millis(),
        zone_id: String::new(),
        reason: signal.reason.clone(),
    })
}

pub async fn run_publisher(
    mut client: EngineClient,
    mut rx: mpsc::Receiver<Signal>,
    ttl_ms: i64,
    config_version: String,
    timeframe: Timeframe,
) {
    let mut seq = 1u64;
    while let Some(signal) = rx.recv().await {
        let Some(proto) =
            to_proto_signal(&signal, seq, now_ms(), ttl_ms, &config_version, timeframe)
        else {
            log::warn!("signal {} not representable as f64; dropped", signal.reason);
            continue;
        };
        seq += 1;
        match client.publish(proto.clone()).await {
            Ok(ack) => log::info!(
                "publish symbol={} action={} accepted={} reason={}",
                proto.ticker,
                proto.action,
                ack.accepted,
                ack.reason
            ),
            Err(error) => log::error!("publish failed symbol={}: {error} (dropped)", proto.ticker),
        }
    }
}
