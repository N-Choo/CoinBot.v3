use crate::exchange::timeframes::Timeframe;
use crate::strategy::config::Config;
use crate::strategy::types::{Direction, Signal};
use rust_decimal::prelude::ToPrimitive;
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tonic::transport::Channel;

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
        let channel = Channel::from_shared(endpoint.to_string())
            .map_err(|e| format!("invalid engine endpoint {endpoint:?}: {e}"))?
            .connect_lazy();
        Ok(Self {
            client: EngineServiceClient::new(channel),
        })
    }

    #[allow(clippy::result_large_err)]
    pub async fn active_symbols(&mut self) -> Result<Vec<String>, tonic::Status> {
        Ok(self
            .client
            .get_active_symbols(Empty {})
            .await?
            .into_inner()
            .tickers)
    }

    #[allow(clippy::result_large_err)]
    pub async fn publish(&mut self, signal: ProtoSignal) -> Result<Ack, tonic::Status> {
        Ok(self.client.publish(signal).await?.into_inner())
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn signal_ttl_ms(timeframe: Timeframe) -> i64 {
    (timeframe.millis() / 10).clamp(60_000, 3_600_000)
}

pub fn config_version(cfg: &Config) -> String {
    let window = cfg.window;
    format!("rsi-v1-w{window}")
}

fn price_f64(value: rust_decimal::Decimal) -> Option<f64> {
    value.to_f64()
}

fn signal_id(signal: &Signal, config_version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"analyzer-signal-v1\0");
    hasher.update(signal.ticker.as_bytes());
    hasher.update([match signal.action {
        Direction::Long => b'B',
        Direction::Short => b'S',
    }]);
    hasher.update([signal.divergence_kind.as_str().as_bytes()[0]]);
    hasher.update(signal.divergence_open_time_ms.to_be_bytes());
    hasher.update(config_version.as_bytes());
    let digest = hasher.finalize();
    let mut id = String::from("v1:");
    for byte in digest {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

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
        sl_price: price_f64(signal.sl_price)?,
        tp_price: price_f64(signal.tp_price)?,
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
            eprintln!("signal {} not representable as f64; dropped", signal.reason);
            continue;
        };
        seq += 1;
        match client.publish(proto.clone()).await {
            Ok(ack) => println!(
                "publish symbol={} action={} accepted={} reason={}",
                proto.ticker, proto.action, ack.accepted, ack.reason
            ),
            Err(error) => eprintln!("publish failed symbol={}: {error} (dropped)", proto.ticker),
        }
    }
}

pub fn ttl_duration(_ttl_ms: i64) -> Duration {
    Duration::from_millis(0)
}
