use crate::harness::sweep::execution::{EntryMode, ExecModel};
use crate::strategy::config::{decimal_from_str, Config};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Independent digest of the committed lock file itself.
///
/// This makes the lock's config/model/dataset/decision rule tamper-evident even if the lock's own
/// `signal_generator_sha256` is edited to match altered frozen files.
pub const EXPECTED_LOCK_SHA256: &str =
    "b4c6e24d0e0c298b3c713d93e937af5dbc869bc6dc964af0611ef8083c849063";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrozenConfig {
    pub rsi_period: usize,
    pub rsi_order: usize,
    pub max_age: Option<usize>,
    pub sma_enabled: bool,
    pub sma_period: usize,
    pub atr_period: usize,
    pub atr_sl_multiplier: String,
    pub atr_tp_multiplier: String,
    pub price_diff_threshold: String,
    pub atr_pct_high: String,
    pub atr_high_adjust: String,
    pub atr_sl_max_pct: String,
    pub window: usize,
}

impl FrozenConfig {
    /// Resolves the lock's string decimals into the strategy configuration.
    pub fn to_config(&self) -> Result<Config, String> {
        let decimal = |field: &str, value: &str| {
            decimal_from_str(value).map_err(|error| format!("config.{field}: {error}"))
        };
        let config = Config {
            rsi_period: self.rsi_period,
            rsi_order: self.rsi_order,
            price_diff_threshold: decimal("price_diff_threshold", &self.price_diff_threshold)?,
            sma_enabled: self.sma_enabled,
            sma_period: self.sma_period,
            atr_period: self.atr_period,
            atr_sl_multiplier: decimal("atr_sl_multiplier", &self.atr_sl_multiplier)?,
            atr_tp_multiplier: decimal("atr_tp_multiplier", &self.atr_tp_multiplier)?,
            atr_pct_high: decimal("atr_pct_high", &self.atr_pct_high)?,
            atr_high_adjust: decimal("atr_high_adjust", &self.atr_high_adjust)?,
            atr_sl_max_pct: decimal("atr_sl_max_pct", &self.atr_sl_max_pct)?,
            window: self.window,
        };
        config.validate().map_err(|error| error.to_string())?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrozenModel {
    pub name: String,
    pub entry: String,
    pub offset_bps: String,
    pub valid_candles: usize,
    pub buffer_bps: String,
    pub maker_bps: String,
    pub taker_bps: String,
    pub slip_bps: String,
}

impl FrozenModel {
    /// Resolves the lock's execution model, rejecting anything outside the frozen pre-registration.
    pub fn to_exec_model(&self) -> Result<ExecModel, String> {
        if self.name != "retest_2c_b2" {
            return Err(format!(
                "unsupported frozen model {:?}; expected \"retest_2c_b2\"",
                self.name
            ));
        }
        if self.entry != "limit" {
            return Err(format!(
                "unsupported frozen entry {:?}; expected \"limit\"",
                self.entry
            ));
        }
        let decimal = |field: &str, value: &str| {
            decimal_from_str(value).map_err(|error| format!("model.{field}: {error}"))
        };
        Ok(ExecModel {
            label: self.name.clone(),
            entry: EntryMode::Limit {
                valid_candles: self.valid_candles,
                buffer_bps: decimal("buffer_bps", &self.buffer_bps)?,
                offset_bps: decimal("offset_bps", &self.offset_bps)?,
            },
            maker_bps: decimal("maker_bps", &self.maker_bps)?,
            taker_bps: decimal("taker_bps", &self.taker_bps)?,
            slip_bps: decimal("slip_bps", &self.slip_bps)?,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrozenDataset {
    pub symbols: Vec<String>,
    pub timeframe: String,
    pub as_of_ms: i64,
    pub months: u32,
    pub start_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecisionRule {
    pub min_pooled_trades: usize,
    pub require_positive_net_expectancy: bool,
    pub require_profit_factor_above: f64,
    pub min_positive_symbols: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrozenLock {
    pub created_at: String,
    pub git_commit: String,
    pub signal_generator_sha256: String,
    pub frozen_files: Vec<String>,
    pub config: FrozenConfig,
    pub model: FrozenModel,
    pub dataset: FrozenDataset,
    pub decision_rule: DecisionRule,
}

pub fn load(path: &Path) -> Result<FrozenLock, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Hashes the frozen files in sorted relative-path order.
///
/// Each entry contributes its relative path, a NUL separator, the file bytes, and another NUL.
/// Sorting the lock's path strings (rather than the joined paths) keeps the digest independent of
/// the repository location and path separator.
pub fn compute_hash(repo_root: &Path, files: &[String]) -> Result<String, String> {
    let mut relative: Vec<&str> = files.iter().map(String::as_str).collect();
    relative.sort_unstable();

    let mut hasher = Sha256::new();
    for path in relative {
        let absolute = repo_root.join(path);
        let bytes =
            std::fs::read(&absolute).map_err(|error| format!("{}: {error}", absolute.display()))?;
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify(repo_root: &Path, lock: &FrozenLock) -> Result<(), String> {
    let actual = compute_hash(repo_root, &lock.frozen_files)?;
    if actual != lock.signal_generator_sha256 {
        return Err(format!(
            "frozen-file hash mismatch: expected {}, got {}",
            lock.signal_generator_sha256, actual
        ));
    }
    Ok(())
}

/// Verifies the lock bytes against the independently committed digest.
///
/// Any edit to the lock file (including its config, model, dataset, decision rule, or stored
/// frozen-file hash) changes the digest and aborts the run.
pub fn verify_lock_digest(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != EXPECTED_LOCK_SHA256 {
        return Err(format!(
            "lock digest mismatch: expected {}, got {}",
            EXPECTED_LOCK_SHA256, actual
        ));
    }
    Ok(())
}

fn expect_value<T, U>(field: &str, actual: &T, expected: U) -> Result<(), String>
where
    T: PartialEq<U> + std::fmt::Debug,
    U: std::fmt::Debug,
{
    if *actual != expected {
        return Err(format!(
            "locked {field} is {actual:?}; expected {expected:?}"
        ));
    }
    Ok(())
}

/// Rejects a lock whose frozen values differ from the pre-registered confirmation.
pub fn validate_lock(lock: &FrozenLock) -> Result<(), String> {
    let config = &lock.config;
    expect_value("config.rsi_period", &config.rsi_period, 14)?;
    expect_value("config.rsi_order", &config.rsi_order, 5)?;
    expect_value("config.max_age", &config.max_age, Some(4))?;
    expect_value("config.sma_enabled", &config.sma_enabled, true)?;
    expect_value("config.sma_period", &config.sma_period, 20)?;
    expect_value("config.atr_period", &config.atr_period, 14)?;
    expect_value("config.atr_sl_multiplier", &config.atr_sl_multiplier, "1.5")?;
    expect_value("config.atr_tp_multiplier", &config.atr_tp_multiplier, "2.0")?;
    expect_value(
        "config.price_diff_threshold",
        &config.price_diff_threshold,
        "0.05",
    )?;
    expect_value("config.atr_pct_high", &config.atr_pct_high, "5.0")?;
    expect_value("config.atr_high_adjust", &config.atr_high_adjust, "1.5")?;
    expect_value("config.atr_sl_max_pct", &config.atr_sl_max_pct, "0.0")?;
    expect_value("config.window", &config.window, 100)?;

    let model = &lock.model;
    expect_value("model.name", &model.name, "retest_2c_b2")?;
    expect_value("model.entry", &model.entry, "limit")?;
    expect_value("model.offset_bps", &model.offset_bps, "25")?;
    expect_value("model.valid_candles", &model.valid_candles, 2)?;
    expect_value("model.buffer_bps", &model.buffer_bps, "2")?;
    expect_value("model.maker_bps", &model.maker_bps, "2")?;
    expect_value("model.taker_bps", &model.taker_bps, "10")?;
    expect_value("model.slip_bps", &model.slip_bps, "5")?;

    let dataset = &lock.dataset;
    expect_value(
        "dataset.symbols",
        &dataset.symbols,
        vec![
            "BTC-USDT".to_string(),
            "ETH-USDT".to_string(),
            "SOL-USDT".to_string(),
        ],
    )?;
    expect_value("dataset.timeframe", &dataset.timeframe, "4hour")?;
    expect_value("dataset.as_of_ms", &dataset.as_of_ms, 1_729_555_200_000i64)?;
    expect_value("dataset.months", &dataset.months, 24u32)?;
    expect_value("dataset.start_ms", &dataset.start_ms, 1_667_347_200_000i64)?;

    let rule = &lock.decision_rule;
    expect_value(
        "decision_rule.min_pooled_trades",
        &rule.min_pooled_trades,
        50,
    )?;
    expect_value(
        "decision_rule.require_positive_net_expectancy",
        &rule.require_positive_net_expectancy,
        true,
    )?;
    expect_value(
        "decision_rule.require_profit_factor_above",
        &rule.require_profit_factor_above,
        1.0,
    )?;
    expect_value(
        "decision_rule.min_positive_symbols",
        &rule.min_positive_symbols,
        2,
    )?;
    Ok(())
}
