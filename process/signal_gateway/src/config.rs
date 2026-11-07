use std::path::PathBuf;
use std::time::Duration;

use chrono::TimeDelta;
use share::ProcessError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceCheck {
    Strict,
    Off,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub opencode_base_url: String,
    pub opencode_username: String,
    pub opencode_password: Option<String>,
    pub opencode_agent: String,
    pub opencode_model: Option<String>,
    pub opencode_timeout: Duration,
    pub grpc_endpoint: String,
    pub grpc_timeout: Duration,
    pub state_path: PathBuf,
    pub audit_path: PathBuf,
    pub min_confidence: f64,
    pub price_check: PriceCheck,
    pub max_entry_drift_pct: f64,
    pub cycle_delay: TimeDelta,
    pub signal_ttl: TimeDelta,
    pub run_on_start: bool,
    pub delete_sessions: bool,
    pub reset_state: bool,
    pub kucoin_base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ProcessError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, ProcessError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let value = |key: &str| lookup(key).filter(|v| !v.is_empty());
        let value_or = |key: &str, default: &str| {
            value(key).unwrap_or_else(|| default.to_string())
        };
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
        let parse_bool = |key: &str, default: bool| -> bool {
            match value(key).as_deref() {
                Some("true" | "1") => true,
                Some("false" | "0") => false,
                Some(other) => {
                    log::warn!("{key}={other:?} is not a bool; using {default}");
                    default
                }
                None => default,
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

        let cycle_delay = TimeDelta::seconds(parse_u64("SIGNAL_GATEWAY_CYCLE_DELAY_SECS", 90)? as i64);
        let signal_ttl = TimeDelta::seconds(parse_u64("SIGNAL_GATEWAY_SIGNAL_TTL_SECS", 14_400)? as i64);
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
            state_path: PathBuf::from(value_or(
                "SIGNAL_GATEWAY_STATE_PATH",
                "./state/signal_gateway.json",
            )),
            audit_path: PathBuf::from(value_or(
                "SIGNAL_GATEWAY_AUDIT_PATH",
                "./state/audit.jsonl",
            )),
            min_confidence,
            price_check,
            max_entry_drift_pct,
            cycle_delay,
            signal_ttl,
            run_on_start: parse_bool("SIGNAL_GATEWAY_RUN_ON_START", false),
            delete_sessions: parse_bool("SIGNAL_GATEWAY_DELETE_SESSIONS", true),
            reset_state: parse_bool("SIGNAL_GATEWAY_RESET_STATE", false),
            kucoin_base_url: value_or("KUCOIN_BASE_URL", "https://api.kucoin.com"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Config {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|key| map.get(key).cloned()).unwrap()
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
        let err = Config::from_lookup(|key| map.get(key).cloned()).unwrap_err();
        assert!(err.to_string().contains("MIN_CONFIDENCE"));
    }

    #[test]
    fn rejects_unknown_price_check() {
        let map = HashMap::from([(
            "SIGNAL_GATEWAY_PRICE_CHECK".to_string(),
            "sometimes".to_string(),
        )]);
        let err = Config::from_lookup(|key| map.get(key).cloned()).unwrap_err();
        assert!(err.to_string().contains("PRICE_CHECK"));
    }
}
