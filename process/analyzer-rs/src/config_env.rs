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
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|_| ConfigEnvError::Invalid(key, value)),
        Err(_) => Ok(default),
    }
}

fn parse_decimal(key: &'static str, default: Decimal) -> Result<Decimal, ConfigEnvError> {
    match std::env::var(key) {
        Ok(value) => parse_decimal_str(key, &value),
        Err(_) => Ok(default),
    }
}

fn parse_bool(key: &'static str, default: bool) -> Result<bool, ConfigEnvError> {
    match std::env::var(key) {
        Ok(value) => parse_bool_str(key, &value),
        Err(_) => Ok(default),
    }
}
