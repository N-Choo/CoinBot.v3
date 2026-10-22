use crate::exchange::timeframes::Timeframe;
use crate::strategy::config::Config;
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub enum LiveConfigError {
    #[error("unsupported KUCOIN_TIMEFRAME {0:?} (use 15min, 4hour, or 1day)")]
    UnsupportedTimeframe(String),
    #[error("invalid {0}={1}")]
    Invalid(&'static str, String),
    #[error("window must be >= 8")]
    WindowTooSmall,
    #[error("window {window} is below the strategy minimum {need}")]
    WindowBelowStrategy { window: usize, need: usize },
    #[error("poll interval must be > 0")]
    PollIntervalZero,
    #[error("activation poll interval must be > 0")]
    ActivationPollIntervalZero,
    #[error("candle settle must be >= 0")]
    NegativeSettle,
}

#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub symbols: Vec<String>,
    pub timeframe: Timeframe,
    pub window: usize,
    pub settle_ms: i64,
    pub poll_ms: u64,
    pub activation_poll_ms: u64,
    pub endpoint: String,
    pub base_url: String,
    pub max_divergence_age_candles: Option<usize>,
}

impl LiveConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbols: Vec<String>,
        timeframe: &str,
        window: usize,
        settle_ms: i64,
        poll_ms: u64,
        activation_poll_ms: u64,
        endpoint: String,
        base_url: String,
    ) -> Result<Self, LiveConfigError> {
        let timeframe = Timeframe::from_kucoin(timeframe)
            .ok_or_else(|| LiveConfigError::UnsupportedTimeframe(timeframe.to_string()))?;
        if window < 8 {
            return Err(LiveConfigError::WindowTooSmall);
        }
        if settle_ms < 0 {
            return Err(LiveConfigError::NegativeSettle);
        }
        if poll_ms == 0 {
            return Err(LiveConfigError::PollIntervalZero);
        }
        if activation_poll_ms == 0 {
            return Err(LiveConfigError::ActivationPollIntervalZero);
        }

        let mut symbols: Vec<String> = symbols
            .into_iter()
            .map(|symbol| symbol.trim().replace('/', "-"))
            .filter(|symbol| !symbol.is_empty())
            .collect();
        let mut seen = BTreeSet::new();
        symbols.retain(|symbol| seen.insert(symbol.clone()));

        Ok(LiveConfig {
            symbols,
            timeframe,
            window,
            settle_ms,
            poll_ms,
            activation_poll_ms,
            endpoint,
            base_url,
            max_divergence_age_candles: None,
        })
    }

    pub fn from_env() -> Result<Self, LiveConfigError> {
        let symbols = std::env::var("LIVE_SYMBOLS")
            .unwrap_or_default()
            .split(',')
            .map(str::to_string)
            .collect();
        let timeframe = env_string("KUCOIN_TIMEFRAME").unwrap_or_else(|| "15min".into());
        let window = parse("ANALYZER_WINDOW_CANDLES", 100)?;
        let settle_ms = parse("ANALYZER_CANDLE_SETTLE_MS", 5_000)?;
        let poll_ms = parse("ANALYZER_POLL_INTERVAL_MS", 1_000)?;
        let activation_poll_ms = parse("ANALYZER_ACTIVATION_POLL_MS", 30_000)?;
        let endpoint = env_string("GRPC_TRADE_ENGINE_ENDPOINT")
            .unwrap_or_else(|| "http://127.0.0.1:50051".into());
        let base_url =
            env_string("KUCOIN_BASE_URL").unwrap_or_else(|| "https://api.kucoin.com".into());

        let mut cfg = Self::new(
            symbols,
            &timeframe,
            window,
            settle_ms,
            poll_ms,
            activation_poll_ms,
            endpoint,
            base_url,
        )?;
        cfg.max_divergence_age_candles = match env_string("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES") {
            Some(value) => crate::config_env::parse_optional_usize_str(
                "ANALYZER_MAX_DIVERGENCE_AGE_CANDLES",
                &value,
            )
            .map_err(|_| LiveConfigError::Invalid("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES", value))?,
            None => None,
        };
        Ok(cfg)
    }

    /// Reject a live window that cannot satisfy the strategy's indicator warm-up.
    pub fn validate_strategy(&self, strategy: &Config) -> Result<(), LiveConfigError> {
        let need = strategy.min_window();
        if self.window < need {
            return Err(LiveConfigError::WindowBelowStrategy {
                window: self.window,
                need,
            });
        }
        Ok(())
    }
}

pub use crate::config_env::load_dotenv;

/// Build the strategy config from the Python-compatible environment names, falling back to
/// `Config::default()` for anything that is unset.
pub fn strategy_from_env() -> Result<Config, LiveConfigError> {
    crate::config_env::strategy_from_env()
        .map_err(|error| LiveConfigError::Invalid("strategy", error.to_string()))
}

fn env_string(key: &'static str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, LiveConfigError> {
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|_| LiveConfigError::Invalid(key, value)),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use crate::config_env::{
        parse_bool_str, parse_decimal_str, parse_optional_usize_str, ConfigEnvError,
    };
    use rust_decimal::Decimal;

    #[test]
    fn boolean_forms_are_accepted() {
        for value in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(parse_bool_str("TEST", value).unwrap());
        }
        for value in ["0", "false", "FALSE", " no ", "off"] {
            assert!(!parse_bool_str("TEST", value).unwrap());
        }
        assert!(matches!(
            parse_bool_str("TEST", "maybe"),
            Err(ConfigEnvError::Invalid("TEST", _))
        ));
    }

    #[test]
    fn decimal_forms_are_accepted() {
        assert_eq!(
            parse_decimal_str("TEST", "0.05").unwrap(),
            Decimal::new(5, 2)
        );
        assert_eq!(parse_decimal_str("TEST", "2").unwrap(), Decimal::new(2, 0));
        assert_eq!(
            parse_decimal_str("TEST", " 3.5 ").unwrap(),
            Decimal::new(35, 1)
        );
        assert!(matches!(
            parse_decimal_str("TEST", "nope"),
            Err(ConfigEnvError::Invalid("TEST", _))
        ));
    }

    #[test]
    fn optional_age_cap_treats_empty_as_none() {
        assert_eq!(parse_optional_usize_str("TEST", "").unwrap(), None);
        assert_eq!(parse_optional_usize_str("TEST", "   ").unwrap(), None);
        assert_eq!(parse_optional_usize_str("TEST", "5").unwrap(), Some(5));
        assert!(matches!(
            parse_optional_usize_str("TEST", "-1"),
            Err(ConfigEnvError::Invalid("TEST", _))
        ));
    }
}
