use crate::strategy::error::StrategyError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode { FixedR, Structural }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub atr_period: u32,
    pub pivot_left: u32,
    pub pivot_right: u32,
    pub bos_min_distance_atr: Decimal,
    pub zone_search_lookback: u32,
    pub impulse_body_threshold_atr: Decimal,
    pub zone_expiry_candles: u32,
    pub stop_buffer_atr: Decimal,
    pub min_zone_height_atr: Decimal,
    pub max_zone_height_atr: Decimal,
    pub mitigation_grace_candles: u32,
    pub target_mode: TargetMode,
    pub target_r: Decimal,
    pub min_gross_rr: Decimal,
    pub structural_buffer_atr: Decimal,
    pub allow_no_obstacle: bool,
    pub engine_net_floor: Decimal,
    pub working_scale: u32,
    pub ratio_serialization_scale: u32,
    /// Per-symbol price scale (dp).
    pub price_scale: BTreeMap<String, u32>,
}

impl Config {
    pub fn default_for(symbols: &[&str]) -> Self {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        Config {
            atr_period: 14, pivot_left: 2, pivot_right: 2,
            bos_min_distance_atr: d("0.1"), zone_search_lookback: 10,
            impulse_body_threshold_atr: d("1.5"), zone_expiry_candles: 30,
            stop_buffer_atr: d("0.1"), min_zone_height_atr: d("0.5"),
            max_zone_height_atr: d("3.0"), mitigation_grace_candles: 1,
            target_mode: TargetMode::FixedR, target_r: d("3.0"),
            min_gross_rr: d("3.0"), structural_buffer_atr: d("0.1"),
            allow_no_obstacle: true, engine_net_floor: d("2.5"),
            working_scale: 12, ratio_serialization_scale: 4,
            price_scale: symbols.iter().map(|s| (s.to_string(), 4)).collect(),
        }
    }

    pub fn validate(&self) -> Result<(), StrategyError> {
        let bad = |m: &str| StrategyError::ConfigInvalid(m.to_string());
        if self.atr_period == 0 { return Err(bad("atr_period must be >= 1")); }
        if self.pivot_left == 0 || self.pivot_right == 0 { return Err(bad("pivot windows must be >= 1")); }
        if self.min_zone_height_atr > self.max_zone_height_atr { return Err(bad("min_zone_height > max_zone_height")); }
        if self.bos_min_distance_atr <= Decimal::ZERO { return Err(bad("bos_min_distance must be positive")); }
        if self.impulse_body_threshold_atr <= Decimal::ZERO { return Err(bad("impulse threshold must be positive")); }
        if self.target_r <= self.engine_net_floor { return Err(bad("target_r must exceed engine net floor (cushion)")); }
        if self.min_gross_rr <= self.engine_net_floor { return Err(bad("min_gross_rr must exceed engine net floor (cushion)")); }
        if self.price_scale.is_empty() { return Err(bad("price_scale required per symbol")); }
        Ok(())
    }

    /// Deterministic canonical JSON used for `config_version`.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("config is serializable")
    }
}

/// `config_version` = sha256 hex of the canonical config. Hashing lives at the edge.
pub fn config_version(canonical_json: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(canonical_json.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default_for(&["XRP-USDT"]).validate().is_ok());
    }

    #[test]
    fn target_r_below_floor_rejected() {
        let mut c = Config::default_for(&["XRP-USDT"]);
        c.target_r = dec("2.0");
        assert!(matches!(c.validate(), Err(StrategyError::ConfigInvalid(_))));
    }

    #[test]
    fn min_gross_rr_below_floor_rejected() {
        let mut c = Config::default_for(&["XRP-USDT"]);
        c.min_gross_rr = dec("2.5");
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_version_hashes_json_stably() {
        let c = Config::default_for(&["XRP-USDT"]);
        let j = c.canonical_json();
        assert_eq!(config_version(&j), config_version(&j));
        assert_ne!(config_version(&j), config_version("{}"));
    }
}
