use crate::strategy::config::Config;
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum ConfigEnvError {
    #[error("invalid {0}={1}")]
    Invalid(&'static str, String),
    #[error("invalid strategy config: {0}")]
    Strategy(String),
}

pub fn load_dotenv() {
    let candidates = [
        std::env::current_dir().ok().map(|dir| dir.join(".env")),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env")),
    ];
    for path in candidates.into_iter().flatten() {
        if path.is_file() {
            let _ = dotenvy::from_path(path);
            return;
        }
    }
}

pub fn env_string(key: &'static str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn strategy_from_env() -> Result<Config, ConfigEnvError> {
    let mut cfg = Config::default();
    cfg.rsi_period = parse("RSI_PERIOD", cfg.rsi_period)?;
    cfg.rsi_order = parse("RSI_DIVERGENCE_ORDER", cfg.rsi_order)?;
    cfg.price_diff_threshold = parse_decimal("PRICE_DIFF_THRESHOLD", cfg.price_diff_threshold)?;
    cfg.sma_enabled = parse_bool("SMA_ENABLED", cfg.sma_enabled)?;
    cfg.sma_period = parse("SMA_PERIOD", cfg.sma_period)?;
    cfg.atr_period = parse("ATR_PERIOD", cfg.atr_period)?;
    cfg.atr_sl_multiplier = parse_decimal("ATR_SL_MULTIPLIER", cfg.atr_sl_multiplier)?;
    cfg.atr_tp_multiplier = parse_decimal("ATR_TP_MULTIPLIER", cfg.atr_tp_multiplier)?;
    cfg.atr_pct_high = parse_decimal("ATR_PCT_HIGH", cfg.atr_pct_high)?;
    cfg.atr_high_adjust = parse_decimal("ATR_HIGH_ADJUST", cfg.atr_high_adjust)?;
    cfg.atr_sl_max_pct = parse_decimal("ATR_SL_MAX_PCT", cfg.atr_sl_max_pct)?;
    cfg.window = parse("ANALYZER_WINDOW_CANDLES", cfg.window)?;
    cfg.validate()
        .map_err(|error| ConfigEnvError::Strategy(error.to_string()))?;
    Ok(cfg)
}

pub fn parse_bool_str(key: &'static str, value: &str) -> Result<bool, ConfigEnvError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigEnvError::Invalid(key, value.to_string())),
    }
}

pub fn parse_decimal_str(key: &'static str, value: &str) -> Result<Decimal, ConfigEnvError> {
    Decimal::from_str(value.trim()).map_err(|_| ConfigEnvError::Invalid(key, value.to_string()))
}

pub fn parse_optional_usize_str(
    key: &'static str,
    value: &str,
) -> Result<Option<usize>, ConfigEnvError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse()
        .map(Some)
        .map_err(|_| ConfigEnvError::Invalid(key, value.to_string()))
}

fn parse<T: FromStr>(key: &'static str, default: T) -> Result<T, ConfigEnvError> {
    match env_string(key) {
        Some(value) => value
            .parse()
            .map_err(|_| ConfigEnvError::Invalid(key, value)),
        None => Ok(default),
    }
}

fn parse_decimal(key: &'static str, default: Decimal) -> Result<Decimal, ConfigEnvError> {
    match env_string(key) {
        Some(value) => parse_decimal_str(key, &value),
        None => Ok(default),
    }
}

fn parse_bool(key: &'static str, default: bool) -> Result<bool, ConfigEnvError> {
    match env_string(key) {
        Some(value) => parse_bool_str(key, &value),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_values_are_trimmed() {
        assert_eq!(
            parse_decimal_str("X", " 1.5 ").unwrap(),
            Decimal::new(15, 1)
        );
    }

    #[test]
    fn booleans_are_case_and_space_insensitive() {
        assert!(parse_bool_str("X", " ON ").unwrap());
        assert!(!parse_bool_str("X", " Off ").unwrap());
    }

    #[test]
    fn empty_optional_is_none() {
        assert_eq!(parse_optional_usize_str("X", "  ").unwrap(), None);
        assert_eq!(parse_optional_usize_str("X", " 7 ").unwrap(), Some(7));
    }

    #[test]
    fn env_string_trims_and_treats_empty_as_unset() {
        std::env::set_var("ANALYZER_TEST_TRIMMED", " 42 ");
        std::env::set_var("ANALYZER_TEST_EMPTY", "   ");
        assert_eq!(env_string("ANALYZER_TEST_TRIMMED"), Some("42".into()));
        assert_eq!(env_string("ANALYZER_TEST_EMPTY"), None);
        std::env::remove_var("ANALYZER_TEST_TRIMMED");
        std::env::remove_var("ANALYZER_TEST_EMPTY");
    }

    #[test]
    fn parse_uses_trimmed_env_and_empty_falls_back() {
        std::env::set_var("ANALYZER_TEST_INT", " 14 ");
        std::env::set_var("ANALYZER_TEST_INT_EMPTY", " ");
        assert_eq!(parse("ANALYZER_TEST_INT", 8usize).unwrap(), 14);
        assert_eq!(parse("ANALYZER_TEST_INT_EMPTY", 8usize).unwrap(), 8);
        std::env::remove_var("ANALYZER_TEST_INT");
        std::env::remove_var("ANALYZER_TEST_INT_EMPTY");
    }

    #[test]
    fn decimal_and_bool_env_are_trimmed() {
        std::env::set_var("ANALYZER_TEST_DEC", " 1.5 ");
        std::env::set_var("ANALYZER_TEST_BOOL", " ON ");
        assert_eq!(
            parse_decimal("ANALYZER_TEST_DEC", Decimal::ZERO).unwrap(),
            Decimal::new(15, 1)
        );
        assert!(parse_bool("ANALYZER_TEST_BOOL", false).unwrap());
        std::env::remove_var("ANALYZER_TEST_DEC");
        std::env::remove_var("ANALYZER_TEST_BOOL");
    }
}
