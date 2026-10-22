use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rsi_period: usize,
    pub rsi_order: usize,
    pub price_diff_threshold: Decimal,
    pub sma_enabled: bool,
    pub sma_period: usize,
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
            rsi_period: 14,
            rsi_order: 5,
            price_diff_threshold: Decimal::new(5, 2),
            sma_enabled: false,
            sma_period: 20,
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
        let sma = if self.sma_enabled { self.sma_period } else { 0 };
        self.rsi_period
            .max(self.atr_period)
            .max(sma)
            .saturating_add(self.rsi_order)
            .saturating_add(2)
    }

    pub fn validate(&self) -> Result<()> {
        let fail = |msg: &str| Err(Error::ConfigInvalid(msg.to_string()));
        if self.rsi_period == 0 {
            return fail("rsi_period must be >= 1");
        }
        if self.rsi_order == 0 {
            return fail("rsi_order must be >= 1");
        }
        if self.sma_period == 0 {
            return fail("sma_period must be >= 1");
        }
        if self.atr_period == 0 {
            return fail("atr_period must be >= 1");
        }
        if self.price_diff_threshold < Decimal::ZERO {
            return fail("price_diff_threshold must be >= 0");
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
                    rsi_order: 0,
                    ..Config::default()
                },
                "rsi_order",
            ),
            (
                Config {
                    sma_period: 0,
                    ..Config::default()
                },
                "sma_period",
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
                matches!(
                    cfg.validate(),
                    Err(Error::ConfigInvalid(message)) if message.contains(field)
                ),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn rejects_invalid_threshold_and_atr_settings() {
        let cases = [
            (
                Config {
                    price_diff_threshold: Decimal::new(-1, 0),
                    ..Config::default()
                },
                "price_diff_threshold",
            ),
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
                matches!(
                    cfg.validate(),
                    Err(Error::ConfigInvalid(message)) if message.contains(field)
                ),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn accepts_window_at_indicator_minimum() {
        let need = Config::default().min_window();
        let cfg = Config {
            window: need,
            ..Config::default()
        };
        assert_eq!(cfg.validate(), Ok(()));
    }

    #[test]
    fn rejects_window_below_indicator_minimum() {
        let need = Config::default().min_window();
        let cfg = Config {
            window: need - 1,
            ..Config::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(Error::WindowTooShort {
                need: expected_need,
                got: expected_got,
            }) if expected_need == need && expected_got == need - 1
        ));
    }

    #[test]
    fn rejects_window_below_hard_floor() {
        let cfg = Config {
            rsi_period: 1,
            atr_period: 1,
            rsi_order: 1,
            window: 7,
            ..Config::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(Error::WindowTooShort { need: 8, got: 7 })
        ));
    }
}
