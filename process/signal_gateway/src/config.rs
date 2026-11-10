//! Environment-backed configuration for the gateway.

use std::path::PathBuf;
use std::time::Duration;

use chrono::TimeDelta;
use share::ProcessError;

/// Whether entry prices are checked against the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceCheck {
    Strict,
    Off,
}

/// Runtime configuration for the signal gateway, loaded from environment variables.
#[derive(Clone, PartialEq)]
pub struct Config {
    pub opencode_base_url: String,
    pub opencode_username: String,
    /// OpenCode password; redacted in debug output.
    pub opencode_password: Option<String>,
    pub opencode_agent: String,
    pub opencode_model: Option<String>,
    pub opencode_timeout: Duration,
    pub grpc_endpoint: String,
    pub grpc_timeout: Duration,
    /// Shared token required on internal gRPC calls.
    pub grpc_auth_token: String,
    pub state_path: PathBuf,
    pub audit_path: PathBuf,
    pub min_confidence: f64,
    pub price_check: PriceCheck,
    /// Maximum allowed entry-price drift, as a percentage.
    pub max_entry_drift_pct: f64,
    /// Delay between signal-generation cycles.
    pub cycle_delay: TimeDelta,
    /// Time-to-live for generated signals.
    pub signal_ttl: TimeDelta,
    pub run_on_start: bool,
    pub delete_sessions: bool,
    pub reset_state: bool,
    pub kucoin_base_url: String,
}

/// Debug impl that redacts the opencode password.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opencode_password = self.opencode_password.as_ref().map(|_| "<redacted>");
        f.debug_struct("Config")
            .field("opencode_base_url", &self.opencode_base_url)
            .field("opencode_username", &self.opencode_username)
            .field("opencode_password", &opencode_password)
            .field("opencode_agent", &self.opencode_agent)
            .field("opencode_model", &self.opencode_model)
            .field("opencode_timeout", &self.opencode_timeout)
            .field("grpc_endpoint", &self.grpc_endpoint)
            .field("grpc_timeout", &self.grpc_timeout)
            .field("state_path", &self.state_path)
            .field("audit_path", &self.audit_path)
            .field("min_confidence", &self.min_confidence)
            .field("price_check", &self.price_check)
            .field("max_entry_drift_pct", &self.max_entry_drift_pct)
            .field("cycle_delay", &self.cycle_delay)
            .field("signal_ttl", &self.signal_ttl)
            .field("run_on_start", &self.run_on_start)
            .field("delete_sessions", &self.delete_sessions)
            .field("reset_state", &self.reset_state)
            .field("kucoin_base_url", &self.kucoin_base_url)
            .finish()
    }
}

impl Config {
    /// Load configuration from process environment variables.
    pub fn from_env() -> Result<Self, ProcessError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Load configuration using a custom variable lookup.
    pub fn from_lookup<F>(lookup: F) -> Result<Self, ProcessError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let value = |key: &str| lookup(key).filter(|v| !v.is_empty());
        let value_or = |key: &str, default: &str| value(key).unwrap_or_else(|| default.to_string());
        let parse_u64 = |key: &str, default: u64| -> Result<u64, ProcessError> {
            value_or(key, &default.to_string())
                .parse::<u64>()
                .map_err(|_| ProcessError::InvalidConfig(format!("{key} must be an integer")))
        };
        let parse_f64 = |key: &str, default: f64| -> Result<f64, ProcessError> {
            value_or(key, &default.to_string())
                .parse::<f64>()
                .map_err(|_| ProcessError::InvalidConfig(format!("{key} must be a number")))
        };
        let parse_duration = |key: &str, default: u64| -> Result<TimeDelta, ProcessError> {
            let secs = parse_u64(key, default)?;
            let secs = i64::try_from(secs)
                .map_err(|_| ProcessError::InvalidConfig(format!("{key} is out of range")))?;
            TimeDelta::try_seconds(secs)
                .ok_or_else(|| ProcessError::InvalidConfig(format!("{key} is out of range")))
        };
        let parse_bool = |key: &str, default: bool| -> Result<bool, ProcessError> {
            match value(key) {
                None => Ok(default),
                Some(raw) => match raw.to_ascii_lowercase().as_str() {
                    "true" | "1" => Ok(true),
                    "false" | "0" => Ok(false),
                    _ => Err(ProcessError::InvalidConfig(format!(
                        "{key} must be true, false, 1, or 0, got {raw:?}"
                    ))),
                },
            }
        };

        let min_confidence = parse_f64("SIGNAL_GATEWAY_MIN_CONFIDENCE", 0.55)?;
        if !(0.0..=1.0).contains(&min_confidence) {
            return Err(ProcessError::InvalidConfig(
                "SIGNAL_GATEWAY_MIN_CONFIDENCE must be in [0, 1]".into(),
            ));
        }

        let max_entry_drift_pct = parse_f64("SIGNAL_GATEWAY_MAX_ENTRY_DRIFT_PCT", 5.0)?;
        if !max_entry_drift_pct.is_finite() || max_entry_drift_pct <= 0.0 {
            return Err(ProcessError::InvalidConfig(
                "SIGNAL_GATEWAY_MAX_ENTRY_DRIFT_PCT must be > 0".into(),
            ));
        }

        let price_check = match value("SIGNAL_GATEWAY_PRICE_CHECK").as_deref() {
            None | Some("strict") => PriceCheck::Strict,
            Some("off") => PriceCheck::Off,
            Some(other) => {
                return Err(ProcessError::InvalidConfig(format!(
                    "SIGNAL_GATEWAY_PRICE_CHECK must be strict or off, got {other:?}"
                )));
            }
        };

        let cycle_delay = parse_duration("SIGNAL_GATEWAY_CYCLE_DELAY_SECS", 90)?;
        let signal_ttl = parse_duration("SIGNAL_GATEWAY_SIGNAL_TTL_SECS", 14_400)?;
        if signal_ttl <= TimeDelta::zero() {
            return Err(ProcessError::InvalidConfig(
                "SIGNAL_GATEWAY_SIGNAL_TTL_SECS must be > 0".into(),
            ));
        }

        Ok(Self {
            opencode_base_url: value_or("OPENCODE_BASE_URL", "http://127.0.0.1:4096"),
            opencode_username: value_or("OPENCODE_USERNAME", "opencode"),
            opencode_password: value("OPENCODE_PASSWORD"),
            opencode_agent: value_or("OPENCODE_AGENT", "trade-agent"),
            opencode_model: value("OPENCODE_MODEL"),
            opencode_timeout: Duration::from_secs(parse_u64("OPENCODE_TIMEOUT_SECS", 180)?),
            grpc_endpoint: value_or("GRPC_TRADE_ENGINE_ENDPOINT", "http://127.0.0.1:50051"),
            grpc_timeout: Duration::from_secs(parse_u64("GRPC_TIMEOUT_SECS", 10)?),
            grpc_auth_token: value("GRPC_AUTH_TOKEN")
                .ok_or_else(|| ProcessError::MissingEnv("GRPC_AUTH_TOKEN".into()))?,
            state_path: PathBuf::from(value_or(
                "SIGNAL_GATEWAY_STATE_PATH",
                "./state/signal_gateway.json",
            )),
            audit_path: PathBuf::from(value_or("SIGNAL_GATEWAY_AUDIT_PATH", "./state/audit.jsonl")),
            min_confidence,
            price_check,
            max_entry_drift_pct,
            cycle_delay,
            signal_ttl,
            run_on_start: parse_bool("SIGNAL_GATEWAY_RUN_ON_START", false)?,
            delete_sessions: parse_bool("SIGNAL_GATEWAY_DELETE_SESSIONS", true)?,
            reset_state: parse_bool("SIGNAL_GATEWAY_RESET_STATE", false)?,
            kucoin_base_url: value_or("KUCOIN_BASE_URL", "https://api.kucoin.com"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(map: &HashMap<String, String>) -> Result<Config, ProcessError> {
        let mut map = map.clone();
        map.entry("GRPC_AUTH_TOKEN".to_string())
            .or_insert_with(|| "test-token".to_string());
        Config::from_lookup(|key| map.get(key).cloned())
    }

    fn config(vars: &[(&str, &str)]) -> Config {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        lookup(&map).unwrap()
    }

    #[test]
    fn defaults_match_the_spec() {
        let cfg = config(&[]);
        assert_eq!(cfg.opencode_base_url, "http://127.0.0.1:4096");
        assert_eq!(cfg.opencode_agent, "trade-agent");
        assert_eq!(cfg.grpc_endpoint, "http://127.0.0.1:50051");
        assert_eq!(cfg.min_confidence, 0.55);
        assert_eq!(cfg.price_check, PriceCheck::Strict);
        assert_eq!(cfg.max_entry_drift_pct, 5.0);
        assert_eq!(cfg.cycle_delay, TimeDelta::seconds(90));
        assert_eq!(cfg.signal_ttl, TimeDelta::seconds(14_400));
        assert!(cfg.delete_sessions);
        assert!(!cfg.run_on_start);
        assert!(!cfg.reset_state);
    }

    #[test]
    fn overrides_are_read() {
        let cfg = config(&[
            ("OPENCODE_BASE_URL", "http://example:1234"),
            ("OPENCODE_PASSWORD", "secret"),
            ("OPENCODE_MODEL", "opencode/gpt-5.5"),
            ("SIGNAL_GATEWAY_MIN_CONFIDENCE", "0.7"),
            ("SIGNAL_GATEWAY_PRICE_CHECK", "off"),
            ("SIGNAL_GATEWAY_RUN_ON_START", "true"),
            ("SIGNAL_GATEWAY_SIGNAL_TTL_SECS", "3600"),
        ]);
        assert_eq!(cfg.opencode_base_url, "http://example:1234");
        assert_eq!(cfg.opencode_password.as_deref(), Some("secret"));
        assert_eq!(cfg.opencode_model.as_deref(), Some("opencode/gpt-5.5"));
        assert_eq!(cfg.min_confidence, 0.7);
        assert_eq!(cfg.price_check, PriceCheck::Off);
        assert!(cfg.run_on_start);
        assert_eq!(cfg.signal_ttl, TimeDelta::seconds(3600));
    }

    #[test]
    fn rejects_out_of_range_confidence() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_MIN_CONFIDENCE".to_string(),
            "1.5".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("MIN_CONFIDENCE"));
    }

    #[test]
    fn rejects_unknown_price_check() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_PRICE_CHECK".to_string(),
            "sometimes".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("PRICE_CHECK"));
    }

    #[test]
    fn rejects_zero_entry_drift() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_MAX_ENTRY_DRIFT_PCT".to_string(),
            "0".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("MAX_ENTRY_DRIFT_PCT"));
    }

    #[test]
    fn rejects_zero_signal_ttl() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_SIGNAL_TTL_SECS".to_string(),
            "0".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("SIGNAL_TTL"));
    }

    #[test]
    fn rejects_invalid_boolean() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_DELETE_SESSIONS".to_string(),
            "yes".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("DELETE_SESSIONS"));
    }

    #[test]
    fn rejects_non_numeric_cycle_delay() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_CYCLE_DELAY_SECS".to_string(),
            "soon".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("CYCLE_DELAY"));
    }

    #[test]
    fn rejects_out_of_range_cycle_delay() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_CYCLE_DELAY_SECS".to_string(),
            "9300000000000000".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("SIGNAL_GATEWAY_CYCLE_DELAY_SECS"));
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn rejects_out_of_range_signal_ttl() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_SIGNAL_TTL_SECS".to_string(),
            "9300000000000000".to_string(),
        )]);
        let err = lookup(&map).unwrap_err();
        assert!(err.to_string().contains("SIGNAL_GATEWAY_SIGNAL_TTL_SECS"));
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn debug_redacts_password() {
        let cfg = config(&[("OPENCODE_PASSWORD", "super-secret")]);
        let debug = format!("{cfg:?}");
        assert!(debug.contains("Some(\"<redacted>\")"));
        assert!(!debug.contains("super-secret"));
    }
}
