use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rsi_period: usize,
    pub rsi_long_threshold: Decimal,
    pub rsi_short_threshold: Decimal,
    pub atr_enabled: bool,
    pub atr_period: usize,
    pub atr_sl_multiplier: Decimal,
    pub atr_tp_multiplier: Decimal,
    pub atr_pct_high: Decimal,
    pub atr_high_adjust: Decimal,
    pub atr_sl_max_pct: Decimal,
    pub window: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rsi_period: 21,
            rsi_long_threshold: Decimal::from(60),
            rsi_short_threshold: Decimal::from(50),
            atr_enabled: false,
            atr_period: 14,
            atr_sl_multiplier: Decimal::new(20, 1),
            atr_tp_multiplier: Decimal::new(30, 1),
            atr_pct_high: Decimal::new(50, 1),
            atr_high_adjust: Decimal::new(15, 1),
            atr_sl_max_pct: Decimal::ZERO,
            window: 100,
        }
    }
}

impl Config {
    pub fn min_window(&self) -> usize {
        let atr = if self.atr_enabled { self.atr_period } else { 0 };
        self.rsi_period.max(atr).saturating_add(2)
    }

    pub fn validate(&self) -> Result<()> {
        let fail = |msg: &str| Err(Error::ConfigInvalid(msg.to_string()));
        if self.rsi_period == 0 {
            return fail("rsi_period must be >= 1");
        }
        if self.rsi_short_threshold < Decimal::ZERO {
            return fail("rsi_short_threshold must be >= 0");
        }
        if self.rsi_long_threshold > Decimal::ONE_HUNDRED {
            return fail("rsi_long_threshold must be <= 100");
        }
        if self.rsi_short_threshold >= self.rsi_long_threshold {
            return fail("rsi_short_threshold must be < rsi_long_threshold");
        }
        if self.atr_period == 0 {
            return fail("atr_period must be >= 1");
        }
        if self.atr_sl_multiplier <= Decimal::ZERO {
            return fail("atr_sl_multiplier must be > 0");
        }
        if self.atr_tp_multiplier <= Decimal::ZERO {
            return fail("atr_tp_multiplier must be > 0");
        }
        if self.atr_pct_high < Decimal::ZERO {
            return fail("atr_pct_high must be >= 0");
        }
        if self.atr_high_adjust <= Decimal::ZERO {
            return fail("atr_high_adjust must be > 0");
        }
        if self.atr_sl_max_pct < Decimal::ZERO || self.atr_sl_max_pct >= Decimal::ONE {
            return fail("atr_sl_max_pct must be in [0, 1)");
        }
        if self.window < 8 {
            return Err(Error::WindowTooShort {
                need: 8,
                got: self.window,
            });
        }
        if self.window < self.min_window() {
            return Err(Error::WindowTooShort {
                need: self.min_window(),
                got: self.window,
            });
        }
        Ok(())
    }
}

pub fn decimal_from_str(s: &str) -> Result<Decimal> {
    Decimal::from_str(s).map_err(|_| Error::ConfigInvalid(format!("invalid decimal {s:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn rejects_zero_periods() {
        let cases = [
            (
                Config {
                    rsi_period: 0,
                    ..Config::default()
                },
                "rsi_period",
            ),
            (
                Config {
                    atr_period: 0,
                    ..Config::default()
                },
                "atr_period",
            ),
        ];
        for (cfg, field) in cases {
            assert!(
                matches!(cfg.validate(), Err(Error::ConfigInvalid(message)) if message.contains(field)),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn rejects_inverted_or_out_of_range_thresholds() {
        let cases = [
            (
                Config {
                    rsi_short_threshold: Decimal::new(-1, 0),
                    ..Config::default()
                },
                "rsi_short_threshold",
            ),
            (
                Config {
                    rsi_long_threshold: Decimal::from(101),
                    ..Config::default()
                },
                "rsi_long_threshold",
            ),
            (
                Config {
                    rsi_short_threshold: Decimal::from(60),
                    rsi_long_threshold: Decimal::from(60),
                    ..Config::default()
                },
                "rsi_short_threshold",
            ),
        ];
        for (cfg, field) in cases {
            assert!(
                matches!(cfg.validate(), Err(Error::ConfigInvalid(message)) if message.contains(field)),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn rejects_invalid_atr_settings() {
        let cases = [
            (
                Config {
                    atr_sl_multiplier: Decimal::ZERO,
                    ..Config::default()
                },
                "atr_sl_multiplier",
            ),
            (
                Config {
                    atr_tp_multiplier: Decimal::ZERO,
                    ..Config::default()
                },
                "atr_tp_multiplier",
            ),
            (
                Config {
                    atr_pct_high: Decimal::new(-1, 0),
                    ..Config::default()
                },
                "atr_pct_high",
            ),
            (
                Config {
                    atr_high_adjust: Decimal::ZERO,
                    ..Config::default()
                },
                "atr_high_adjust",
            ),
            (
                Config {
                    atr_sl_max_pct: Decimal::ONE,
                    ..Config::default()
                },
                "atr_sl_max_pct",
            ),
        ];
        for (cfg, field) in cases {
            assert!(
                matches!(cfg.validate(), Err(Error::ConfigInvalid(message)) if message.contains(field)),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn min_window_boundaries_are_exact() {
        let off = Config {
            atr_enabled: false,
            atr_period: 50,
            rsi_period: 21,
            window: 23,
            ..Config::default()
        };
        assert_eq!(off.min_window(), 23);
        assert!(off.validate().is_ok());
        let below = Config { window: 22, ..off };
        assert!(matches!(
            below.validate(),
            Err(Error::WindowTooShort { need: 23, got: 22 })
        ));

        let on = Config {
            atr_enabled: true,
            atr_period: 50,
            rsi_period: 21,
            window: 52,
            ..Config::default()
        };
        assert_eq!(on.min_window(), 52);
        assert!(on.validate().is_ok());
        let on_below = Config { window: 51, ..on };
        assert!(matches!(
            on_below.validate(),
            Err(Error::WindowTooShort { need: 52, got: 51 })
        ));
    }

    #[test]
    fn rejects_window_below_hard_floor() {
        let cfg = Config {
            window: 7,
            rsi_period: 1,
            atr_period: 1,
            ..Config::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(Error::WindowTooShort { need: 8, got: 7 })
        ));
    }
}
