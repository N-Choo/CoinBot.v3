use crate::strategy::config::Config;
use crate::strategy::types::{Direction, Signal};
use sha2::{Digest, Sha256};

pub fn config_version(cfg: &Config) -> String {
    format!(
        "rsi-mom-v1|w={}|rsi={}:{}:{}",
        cfg.window,
        cfg.rsi_period,
        cfg.rsi_long_threshold.normalize(),
        cfg.rsi_short_threshold.normalize(),
    )
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn signal_id(signal: &Signal, config_version: &str) -> String {
    const SEP: u8 = 0;
    let mut hasher = Sha256::new();
    hasher.update(b"analyzer-signal-v2");
    hasher.update([SEP]);
    hasher.update(signal.ticker.as_bytes());
    hasher.update([SEP]);
    hasher.update([match signal.action {
        Direction::Long => b'B',
        Direction::Short => b'S',
    }]);
    hasher.update([SEP]);
    hasher.update(signal.signal_candle_open_time_ms.to_be_bytes());
    hasher.update([SEP]);
    hasher.update(config_version.as_bytes());
    let digest = hasher.finalize();
    let mut id = String::from("v2:");
    for byte in digest {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}
