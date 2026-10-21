use crate::exchange::timeframes::Timeframe;
use crate::strategy::config::Config;
use rust_decimal::Decimal;
use std::collections::BTreeSet;
use std::str::FromStr;

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
            Some(value) => parse_optional_usize("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES", &value)?,
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

/// Load `.env` from the current directory first, then from the crate directory (for `cargo run`
/// from a parent directory). Existing process environment variables are not overwritten.
pub fn load_dotenv() {
    let candidates = [
        std::env::current_dir().ok().map(|dir| dir.join(".env")),
        Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env")),
    ];
    for path in candidates.into_iter().flatten() {
        if path.is_file() {
            let _ = dotenvy::from_path(path);
            return;
        }
    }
}

/// Build the strategy config from the Python-compatible environment names, falling back to
/// `Config::default()` for anything that is unset.
pub fn strategy_from_env() -> Result<Config, LiveConfigError> {
    let default = Config::default();
    let cfg = Config {
        rsi_period: env_parse("RSI_PERIOD", default.rsi_period)?,
        rsi_order: env_parse("RSI_DIVERGENCE_ORDER", default.rsi_order)?,
        price_diff_threshold: env_decimal("PRICE_DIFF_THRESHOLD", default.price_diff_threshold)?,
        sma_enabled: env_bool("SMA_ENABLED", default.sma_enabled)?,
        sma_period: env_parse("SMA_PERIOD", default.sma_period)?,
        atr_period: env_parse("ATR_PERIOD", default.atr_period)?,
        atr_sl_multiplier: env_decimal("ATR_SL_MULTIPLIER", default.atr_sl_multiplier)?,
        atr_tp_multiplier: env_decimal("ATR_TP_MULTIPLIER", default.atr_tp_multiplier)?,
        atr_pct_high: env_decimal("ATR_PCT_HIGH", default.atr_pct_high)?,
        atr_high_adjust: env_decimal("ATR_HIGH_ADJUST", default.atr_high_adjust)?,
        atr_sl_max_pct: env_decimal("ATR_SL_MAX_PCT", default.atr_sl_max_pct)?,
        window: env_parse("ANALYZER_WINDOW_CANDLES", default.window)?,
    };
    cfg.validate()
        .map_err(|error| LiveConfigError::Invalid("strategy", error.to_string()))?;
    Ok(cfg)
}

fn env_string(key: &'static str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_bool_str(key: &'static str, value: &str) -> Result<bool, LiveConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(LiveConfigError::Invalid(key, value.to_string())),
    }
}

fn parse_decimal_str(key: &'static str, value: &str) -> Result<Decimal, LiveConfigError> {
    Decimal::from_str(value.trim()).map_err(|_| LiveConfigError::Invalid(key, value.to_string()))
}

fn parse_optional_usize(key: &'static str, value: &str) -> Result<Option<usize>, LiveConfigError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| LiveConfigError::Invalid(key, value.to_string()))
}

fn env_parse<T: FromStr>(key: &'static str, default: T) -> Result<T, LiveConfigError> {
    match env_string(key) {
        Some(value) => value
            .parse()
            .map_err(|_| LiveConfigError::Invalid(key, value)),
        None => Ok(default),
    }
}

fn env_bool(key: &'static str, default: bool) -> Result<bool, LiveConfigError> {
    match env_string(key) {
        Some(value) => parse_bool_str(key, &value),
        None => Ok(default),
    }
}

fn env_decimal(key: &'static str, default: Decimal) -> Result<Decimal, LiveConfigError> {
    match env_string(key) {
        Some(value) => parse_decimal_str(key, &value),
        None => Ok(default),
    }
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
    use super::*;

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
            Err(LiveConfigError::Invalid("TEST", _))
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
            Err(LiveConfigError::Invalid("TEST", _))
        ));
    }

    #[test]
    fn optional_age_cap_treats_empty_as_none() {
        assert_eq!(parse_optional_usize("TEST", "").unwrap(), None);
        assert_eq!(parse_optional_usize("TEST", "   ").unwrap(), None);
        assert_eq!(parse_optional_usize("TEST", "5").unwrap(), Some(5));
        assert!(matches!(
            parse_optional_usize("TEST", "-1"),
            Err(LiveConfigError::Invalid("TEST", _))
        ));
    }
}
