use crate::harness::sweep::execution::{EntryMode, ExecModel};
use crate::strategy::config::{decimal_from_str, Config};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

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
