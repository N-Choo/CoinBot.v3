use crate::strategy::decimal::work;
use rust_decimal::Decimal;

/// True range; the first candle uses `high - low` (no previous close).
pub fn true_range(prev_close: Option<Decimal>, high: Decimal, low: Decimal) -> Decimal {
    match prev_close {
        None => high - low,
        Some(pc) => (high - low).max((high - pc).abs()).max((low - pc).abs()),
    }
}

/// Wilder ATR seeded with the simple mean of the first `period` true ranges.
/// Returns `None` until warm-up is complete.
/// Incremental Wilder ATR. State stores its fields directly (`atr`, `atr_seed_sum`, `atr_seen`).
pub struct AtrState {
    pub value: Option<Decimal>,
    pub seed_sum: Decimal,
    pub seen: u32,
    pub period: u32,
}

impl AtrState {
    pub fn new(period: u32) -> Self {
        Self {
            value: None,
            seed_sum: Decimal::ZERO,
            seen: 0,
            period,
        }
    }

    /// Feed one candle's TR inputs; returns the running ATR once warm-up completes.
    pub fn update(
        &mut self,
        prev_close: Option<Decimal>,
        high: Decimal,
        low: Decimal,
    ) -> Option<Decimal> {
        let tr = true_range(prev_close, high, low);
        self.seen += 1;
        match self.value {
            None => {
                self.seed_sum += tr;
                if self.seen == self.period {
                    self.value = Some(work(self.seed_sum / Decimal::from(self.period as u64)));
                }
            }
            Some(prev) => {
                self.value = Some(work(
                    (prev * Decimal::from(self.period as u64 - 1) + tr)
                        / Decimal::from(self.period as u64),
                ));
            }
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn first_candle_tr_is_high_minus_low() {
        assert_eq!(true_range(None, dec("10"), dec("8")), dec("2"));
    }

    #[test]
    fn warmup_returns_none() {
        let mut a = AtrState::new(14);
        for _ in 0..13 {
            assert_eq!(a.update(None, dec("2"), dec("0.5")), None);
        }
        assert!(a.update(None, dec("2"), dec("0.5")).is_some());
    }

    #[test]
    fn flat_series_yields_zero_atr_not_error() {
        let mut a = AtrState::new(14);
        let mut v = None;
        for _ in 0..20 {
            v = a.update(Some(dec("1")), dec("1"), dec("1"));
        }
        assert_eq!(v, Some(Decimal::ZERO));
    }

    #[test]
    fn constant_range_seeds_to_that_range() {
        let mut a = AtrState::new(14);
        let mut v = None;
        for _ in 0..20 {
            v = a.update(Some(dec("1")), dec("2"), dec("0.5"));
        }
        assert_eq!(v, Some(dec("1.5")));
    }
}
