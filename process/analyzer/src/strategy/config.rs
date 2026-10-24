use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rsi_period: usize,
    pub rsi_long_threshold: Decimal,
    pub rsi_short_threshold: Decimal,
    pub window: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rsi_period: 21,
            rsi_long_threshold: Decimal::from(60),
            rsi_short_threshold: Decimal::from(50),
            window: 100,
        }
    }
}

impl Config {
    pub fn min_window(&self) -> usize {
        self.rsi_period.saturating_add(2)
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
    fn rejects_zero_period() {
        let cfg = Config {
            rsi_period: 0,
            ..Config::default()
        };
        assert!(
            matches!(cfg.validate(), Err(Error::ConfigInvalid(message)) if message.contains("rsi_period")),
            "expected ConfigInvalid mentioning rsi_period, got {:?}",
            cfg.validate()
        );
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
    fn min_window_boundaries_are_exact() {
        let cfg = Config {
            rsi_period: 21,
            window: 23,
            ..Config::default()
        };
        assert_eq!(cfg.min_window(), 23);
        assert!(cfg.validate().is_ok());
        let below = Config { window: 22, ..cfg };
        assert!(matches!(
            below.validate(),
            Err(Error::WindowTooShort { need: 23, got: 22 })
        ));
    }

    #[test]
    fn rejects_window_below_hard_floor() {
        let cfg = Config {
            window: 7,
            rsi_period: 1,
            ..Config::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(Error::WindowTooShort { need: 8, got: 7 })
        ));
    }
}
