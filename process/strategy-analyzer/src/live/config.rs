//! Phase 1 live (shadow) configuration (live spec §3c, §5).
//!
//! Phase 1 has no `Analyze` call, so the symbol set is an **explicit input** (a config file's
//! `symbols` list or the `LIVE_SYMBOLS` env var) rather than an RPC. Every value has a documented
//! default; env vars override the config file, which overrides the defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Default `allTickers` poll cadence (live spec §5, proposed 5 s).
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 5_000;
/// Default price staleness (live spec §5, proposed 30 s).
pub const DEFAULT_PRICE_STALENESS_MS: i64 = 30_000;
/// Default candle settle (live spec §3a, proposed 5 s).
pub const DEFAULT_CANDLE_SETTLE_MS: i64 = 5_000;
/// Default activation TTL (live spec §3c, proposed 5 min). Phase 1 has no RPC to refresh it.
pub const DEFAULT_ACTIVATION_TTL_MS: i64 = 300_000;
/// Default quarantine cooldown (live spec §8, proposed 24 h).
pub const DEFAULT_QUARANTINE_COOLDOWN_MS: i64 = 86_400_000;
/// Default state directory (a mounted Docker volume in compose).
pub const DEFAULT_STATE_DIR: &str = "data/live/state";
/// Default recorded-input-stream directory.
pub const DEFAULT_STREAM_DIR: &str = "data/live/stream";
/// Default exchange base URL.
pub const DEFAULT_EXCHANGE_BASE_URL: &str = "https://api.kucoin.com";
/// Default candle interval (`4hour`).
pub const DEFAULT_CANDLE_INTERVAL: &str = "4hour";

/// The 4H timeframe in milliseconds. Mirrors `strategy::step::TIMEFRAME_MS`, which is `pub(crate)`
/// to the core; the adapter keeps its own copy rather than widen the core's surface.
pub const TIMEFRAME_MS: i64 = 14_400_000;

fn default_poll_interval_ms() -> u64 {
    DEFAULT_POLL_INTERVAL_MS
}
fn default_price_staleness_ms() -> i64 {
    DEFAULT_PRICE_STALENESS_MS
}
fn default_candle_settle_ms() -> i64 {
    DEFAULT_CANDLE_SETTLE_MS
}
fn default_activation_ttl_ms() -> i64 {
    DEFAULT_ACTIVATION_TTL_MS
}
fn default_quarantine_cooldown_ms() -> i64 {
    DEFAULT_QUARANTINE_COOLDOWN_MS
}
fn default_state_dir() -> String {
    DEFAULT_STATE_DIR.to_string()
}
fn default_stream_dir() -> String {
    DEFAULT_STREAM_DIR.to_string()
}
fn default_exchange_base_url() -> String {
    DEFAULT_EXCHANGE_BASE_URL.to_string()
}
fn default_candle_interval() -> String {
    DEFAULT_CANDLE_INTERVAL.to_string()
}

/// Errors from loading or validating [`LiveConfig`].
#[derive(Debug, thiserror::Error)]
pub enum LiveConfigError {
    #[error("no symbols configured: set `LIVE_SYMBOLS` or a config file's `symbols` list")]
    NoSymbols,
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid config JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid value for {key}: {value:?}")]
    Invalid { key: String, value: String },
}

/// Phase 1 live configuration. Serde defaults make a config file minimal (only `symbols` is
/// required in practice; every other field falls back to its documented default).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveConfig {
    /// The explicit symbol list (Phase 1: no `Analyze` call, live spec §3c).
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_price_staleness_ms")]
    pub price_staleness_ms: i64,
    #[serde(default = "default_candle_settle_ms")]
    pub candle_settle_ms: i64,
    #[serde(default = "default_activation_ttl_ms")]
    pub activation_ttl_ms: i64,
    #[serde(default = "default_quarantine_cooldown_ms")]
    pub quarantine_cooldown_ms: i64,
    #[serde(default = "default_state_dir")]
    pub state_dir: String,
    #[serde(default = "default_stream_dir")]
    pub stream_dir: String,
    #[serde(default = "default_exchange_base_url")]
    pub exchange_base_url: String,
    #[serde(default = "default_candle_interval")]
    pub candle_interval: String,
}

impl Default for LiveConfig {
    fn default() -> Self {
        LiveConfig {
            symbols: Vec::new(),
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            price_staleness_ms: DEFAULT_PRICE_STALENESS_MS,
            candle_settle_ms: DEFAULT_CANDLE_SETTLE_MS,
            activation_ttl_ms: DEFAULT_ACTIVATION_TTL_MS,
            quarantine_cooldown_ms: DEFAULT_QUARANTINE_COOLDOWN_MS,
            state_dir: DEFAULT_STATE_DIR.to_string(),
            stream_dir: DEFAULT_STREAM_DIR.to_string(),
            exchange_base_url: DEFAULT_EXCHANGE_BASE_URL.to_string(),
            candle_interval: DEFAULT_CANDLE_INTERVAL.to_string(),
        }
    }
}

impl LiveConfig {
    /// Load from `LIVE_CONFIG` (a JSON file, optional) then apply `LIVE_*` env overrides.
    pub fn load() -> Result<Self, LiveConfigError> {
        let path = std::env::var("LIVE_CONFIG").ok();
        Self::load_with_file(path.as_deref())
    }

    /// Load from an explicit config file (optional) then apply `LIVE_*` env overrides, then validate.
    pub fn load_with_file(path: Option<&str>) -> Result<Self, LiveConfigError> {
        let mut cfg = match path {
            Some(p) => Self::from_json_file(p)?,
            None => Self::default(),
        };
        cfg.apply_env()?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Parse a config document from a JSON string.
    pub fn from_json_str(s: &str) -> Result<Self, LiveConfigError> {
        Ok(serde_json::from_str(s)?)
    }

    /// Parse a config document from a JSON file.
    pub fn from_json_file(path: &str) -> Result<Self, LiveConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| LiveConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        Self::from_json_str(&text)
    }

    /// Apply the `LIVE_*` env overrides on top of the current values.
    fn apply_env(&mut self) -> Result<(), LiveConfigError> {
        if let Ok(syms) = std::env::var("LIVE_SYMBOLS") {
            self.symbols = parse_symbol_list(&syms);
        }
        if let Ok(v) = std::env::var("LIVE_POLL_INTERVAL_MS") {
            self.poll_interval_ms = parse_u64("LIVE_POLL_INTERVAL_MS", &v)?;
        }
        if let Ok(v) = std::env::var("LIVE_PRICE_STALENESS_MS") {
            self.price_staleness_ms = parse_i64("LIVE_PRICE_STALENESS_MS", &v)?;
        }
        if let Ok(v) = std::env::var("LIVE_CANDLE_SETTLE_MS") {
            self.candle_settle_ms = parse_i64("LIVE_CANDLE_SETTLE_MS", &v)?;
        }
        if let Ok(v) = std::env::var("LIVE_ACTIVATION_TTL_MS") {
            self.activation_ttl_ms = parse_i64("LIVE_ACTIVATION_TTL_MS", &v)?;
        }
        if let Ok(v) = std::env::var("LIVE_QUARANTINE_COOLDOWN_MS") {
            self.quarantine_cooldown_ms = parse_i64("LIVE_QUARANTINE_COOLDOWN_MS", &v)?;
        }
        if let Ok(v) = std::env::var("LIVE_STATE_DIR") {
            self.state_dir = v;
        }
        if let Ok(v) = std::env::var("LIVE_STREAM_DIR") {
            self.stream_dir = v;
        }
        if let Ok(v) = std::env::var("LIVE_EXCHANGE_BASE_URL") {
            self.exchange_base_url = v;
        }
        if let Ok(v) = std::env::var("LIVE_CANDLE_INTERVAL") {
            self.candle_interval = v;
        }
        Ok(())
    }

    /// Validate the parts the loops depend on: at least one symbol and positive intervals.
    pub fn validate(&self) -> Result<(), LiveConfigError> {
        if self.symbols.is_empty() {
            return Err(LiveConfigError::NoSymbols);
        }
        if self.poll_interval_ms == 0 {
            return Err(LiveConfigError::Invalid {
                key: "poll_interval_ms".to_string(),
                value: self.poll_interval_ms.to_string(),
            });
        }
        for (key, value) in [
            ("price_staleness_ms", self.price_staleness_ms),
            ("candle_settle_ms", self.candle_settle_ms),
            ("activation_ttl_ms", self.activation_ttl_ms),
            ("quarantine_cooldown_ms", self.quarantine_cooldown_ms),
        ] {
            if value < 0 {
                return Err(LiveConfigError::Invalid {
                    key: key.to_string(),
                    value: value.to_string(),
                });
            }
        }
        Ok(())
    }

    /// The `allTickers` poll cadence.
    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms)
    }

    /// The state directory as a path.
    pub fn state_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.state_dir)
    }

    /// The recorded-input-stream directory as a path.
    pub fn stream_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.stream_dir)
    }

    /// The candle-close helper's interval in ms (the 4H timeframe).
    pub fn timeframe_ms(&self) -> i64 {
        TIMEFRAME_MS
    }
}

/// Split a comma-separated symbol list, trimming and dropping empties.
pub fn parse_symbol_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_u64(key: &str, raw: &str) -> Result<u64, LiveConfigError> {
    raw.trim().parse().map_err(|_| LiveConfigError::Invalid {
        key: key.to_string(),
        value: raw.to_string(),
    })
}

fn parse_i64(key: &str, raw: &str) -> Result<i64, LiveConfigError> {
    raw.trim().parse().map_err(|_| LiveConfigError::Invalid {
        key: key.to_string(),
        value: raw.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_proposed_spec_values() {
        let c = LiveConfig::default();
        assert_eq!(c.poll_interval_ms, 5_000);
        assert_eq!(c.price_staleness_ms, 30_000);
        assert_eq!(c.candle_settle_ms, 5_000);
        assert_eq!(c.activation_ttl_ms, 300_000);
        assert_eq!(c.quarantine_cooldown_ms, 86_400_000);
        assert_eq!(c.candle_interval, "4hour");
    }

    #[test]
    fn json_file_supplies_symbols_and_overrides_defaults() {
        let cfg = LiveConfig::from_json_str(
            r#"{ "symbols": ["XRP-USDT", "BTC-USDT"], "poll_interval_ms": 1000,
                 "state_dir": "/tmp/x", "candle_settle_ms": 1 }"#,
        )
        .unwrap();
        assert_eq!(cfg.symbols, vec!["XRP-USDT", "BTC-USDT"]);
        assert_eq!(cfg.poll_interval_ms, 1000);
        assert_eq!(cfg.state_dir, "/tmp/x");
        assert_eq!(cfg.candle_settle_ms, 1);
        // Untouched fields keep their defaults.
        assert_eq!(cfg.price_staleness_ms, DEFAULT_PRICE_STALENESS_MS);
    }

    #[test]
    fn missing_symbols_is_rejected() {
        let cfg = LiveConfig::from_json_str(r#"{ "poll_interval_ms": 1000 }"#).unwrap();
        assert!(matches!(cfg.validate(), Err(LiveConfigError::NoSymbols)));
    }

    #[test]
    fn zero_poll_interval_is_rejected() {
        let mut cfg = LiveConfig::default();
        cfg.symbols = vec!["XRP-USDT".into()];
        cfg.poll_interval_ms = 0;
        assert!(matches!(
            cfg.validate(),
            Err(LiveConfigError::Invalid { .. })
        ));
    }

    #[test]
    fn negative_staleness_is_rejected() {
        let mut cfg = LiveConfig::default();
        cfg.symbols = vec!["XRP-USDT".into()];
        cfg.price_staleness_ms = -1;
        assert!(matches!(
            cfg.validate(),
            Err(LiveConfigError::Invalid { .. })
        ));
    }

    #[test]
    fn parse_symbol_list_trims_and_drops_empties() {
        assert_eq!(parse_symbol_list(" A ,, B ,"), vec!["A", "B"]);
    }

    #[test]
    fn valid_config_round_trips_json() {
        let mut cfg = LiveConfig::default();
        cfg.symbols = vec!["XRP-USDT".into()];
        let s = serde_json::to_string(&cfg).unwrap();
        assert_eq!(LiveConfig::from_json_str(&s).unwrap(), cfg);
    }
}
