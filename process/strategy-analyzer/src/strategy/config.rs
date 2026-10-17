use crate::strategy::error::StrategyError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode {
    FixedR,
    Structural,
}

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
    /// Per-symbol price **increment** (an actual price step, e.g. KuCoin `0.0005`). A dp
    /// `price_scale` cannot express a step-5/step-25 grid, so when a symbol's increment is present
    /// the core snaps stop/TP/reference levels to it (spec §7.4/§7.6): stop **away from** entry,
    /// take-profit **toward** entry, so published prices are valid multiples the engine can
    /// re-validate. **Instance data**, like `price_scale`: excluded from `config_version`. A symbol
    /// with no entry falls back to the Phase 0 `round_price(price_scale)` rounding.
    pub price_increment: BTreeMap<String, Decimal>,
}

impl Config {
    pub fn default_for(symbols: &[&str]) -> Self {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        Config {
            atr_period: 14,
            pivot_left: 2,
            pivot_right: 2,
            bos_min_distance_atr: d("0.1"),
            zone_search_lookback: 10,
            impulse_body_threshold_atr: d("1.5"),
            zone_expiry_candles: 30,
            stop_buffer_atr: d("0.1"),
            min_zone_height_atr: d("0.5"),
            max_zone_height_atr: d("3.0"),
            mitigation_grace_candles: 1,
            target_mode: TargetMode::FixedR,
            target_r: d("3.0"),
            min_gross_rr: d("3.0"),
            structural_buffer_atr: d("0.1"),
            allow_no_obstacle: true,
            engine_net_floor: d("2.5"),
            working_scale: 12,
            ratio_serialization_scale: 4,
            price_scale: symbols.iter().map(|s| (s.to_string(), 4)).collect(),
            // Empty by default: without an explicit increment the core uses `round_price(price_scale)`
            // (the Phase 0 path), so existing fixtures/goldens are unchanged. Set per symbol to snap.
            price_increment: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), StrategyError> {
        let bad = |m: &str| StrategyError::ConfigInvalid(m.to_string());
        if self.atr_period == 0 {
            return Err(bad("atr_period must be >= 1"));
        }
        if self.pivot_left == 0 || self.pivot_right == 0 {
            return Err(bad("pivot windows must be >= 1"));
        }
        if self.min_zone_height_atr > self.max_zone_height_atr {
            return Err(bad("min_zone_height > max_zone_height"));
        }
        if self.bos_min_distance_atr <= Decimal::ZERO {
            return Err(bad("bos_min_distance must be positive"));
        }
        if self.impulse_body_threshold_atr <= Decimal::ZERO {
            return Err(bad("impulse threshold must be positive"));
        }
        if self.target_r <= self.engine_net_floor {
            return Err(bad("target_r must exceed engine net floor (cushion)"));
        }
        if self.min_gross_rr <= self.engine_net_floor {
            return Err(bad("min_gross_rr must exceed engine net floor (cushion)"));
        }
        if self.price_scale.is_empty() {
            return Err(bad("price_scale required per symbol"));
        }
        Ok(())
    }

    /// The explicit price increment configured for `symbol`, if any. `None` means the core falls
    /// back to dp `price_scale` rounding for that symbol (see the `price_increment` field).
    pub fn price_increment_for(&self, symbol: &str) -> Option<Decimal> {
        self.price_increment.get(symbol).copied()
    }

    /// Deterministic canonical JSON of the **strategy parameters**, used for `config_version`.
    ///
    /// `price_scale` and `price_increment` are deliberately omitted: they are per-symbol **instance
    /// data** (the symbol's price scale/increment, discovered at activation), not strategy
    /// parameters. Hashing them would change the version — and invalidate the freeze lock — every
    /// time a ticker is added or a symbol's scale/increment is refined. The core still reads them
    /// from `Config` for level rounding/snapping; only the hash input excludes them. Decimal fields
    /// are normalized (trailing zeros removed) before serialization, so `"3"` and `"3.0"` hash
    /// identically: the strategy places no meaning on a Decimal's scale, only on its value.
    pub fn canonical_params_json(&self) -> String {
        let mut c = self.clone();
        c.bos_min_distance_atr = c.bos_min_distance_atr.normalize();
        c.impulse_body_threshold_atr = c.impulse_body_threshold_atr.normalize();
        c.stop_buffer_atr = c.stop_buffer_atr.normalize();
        c.min_zone_height_atr = c.min_zone_height_atr.normalize();
        c.max_zone_height_atr = c.max_zone_height_atr.normalize();
        c.target_r = c.target_r.normalize();
        c.min_gross_rr = c.min_gross_rr.normalize();
        c.structural_buffer_atr = c.structural_buffer_atr.normalize();
        c.engine_net_floor = c.engine_net_floor.normalize();
        let mut value = serde_json::to_value(&c).expect("config is serializable");
        if let serde_json::Value::Object(map) = &mut value {
            map.remove("price_scale");
            map.remove("price_increment");
        }
        serde_json::to_string(&value).expect("config is serializable")
    }
}

/// `config_version` = sha256 hex of the canonical strategy parameters. Hashing lives at the edge.
pub fn config_version(canonical_params_json: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(canonical_params_json.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

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
        let j = c.canonical_params_json();
        assert_eq!(config_version(&j), config_version(&j));
        assert_ne!(config_version(&j), config_version("{}"));
    }

    #[test]
    fn config_version_normalizes_decimal_scale() {
        // `"3"` and `"3.0"` are the same value (Decimal Eq ignores scale) and must hash identically.
        let a = Config::default_for(&["XRP-USDT"]);
        let mut b = a.clone();
        b.target_r = dec("3");
        assert_eq!(a.target_r, b.target_r);
        assert_eq!(
            config_version(&a.canonical_params_json()),
            config_version(&b.canonical_params_json())
        );
    }

    #[test]
    fn config_version_excludes_price_scale() {
        // `price_scale` is per-symbol instance data (the symbol's price increment), not a strategy
        // parameter: adding a ticker or changing a symbol's scale must NOT move the version — that
        // would invalidate the freeze lock every time a ticker is added.
        let a = Config::default_for(&["XRP-USDT"]);
        let b = Config::default_for(&["XRP-USDT", "BTC-USDT"]); // extra ticker
        let mut c = a.clone();
        c.price_scale.insert("XRP-USDT".to_string(), 8); // changed scale
        assert_eq!(
            a.canonical_params_json(),
            b.canonical_params_json(),
            "adding a ticker must not change the hashed params"
        );
        assert_eq!(
            a.canonical_params_json(),
            c.canonical_params_json(),
            "changing a symbol's price scale must not change the hashed params"
        );
    }

    #[test]
    fn config_version_excludes_price_increment() {
        // `price_increment` is per-symbol instance data (like `price_scale`): setting or refining it
        // must NOT move `config_version` — that would invalidate the freeze lock per ticker.
        let a = Config::default_for(&["XRP-USDT"]);
        let mut b = a.clone();
        b.price_increment
            .insert("XRP-USDT".to_string(), dec("0.0005"));
        assert_eq!(
            a.canonical_params_json(),
            b.canonical_params_json(),
            "setting a symbol's price increment must not change the hashed params"
        );
        assert_eq!(
            config_version(&a.canonical_params_json()),
            config_version(&b.canonical_params_json())
        );
    }

    #[test]
    fn price_increment_for_reports_only_configured_symbols() {
        let mut c = Config::default_for(&["XRP-USDT"]);
        assert_eq!(c.price_increment_for("XRP-USDT"), None);
        c.price_increment
            .insert("XRP-USDT".to_string(), dec("0.0005"));
        assert_eq!(c.price_increment_for("XRP-USDT"), Some(dec("0.0005")));
        assert_eq!(c.price_increment_for("BTC-USDT"), None);
    }
}
