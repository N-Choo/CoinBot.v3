use crate::strategy::types::{Candle, Pivot};

/// Strict on the right, `>=` on the left: a plateau resolves to its rightmost member,
/// and a pivot is available only after `right` candles close.
pub fn is_swing_high(c: &[Candle], i: usize, left: usize, right: usize) -> bool {
    if i < left || i + right >= c.len() { return false; }
    let h = c[i].high;
    (1..=right).all(|k| h > c[i + k].high) && (1..=left).all(|k| h >= c[i - k].high)
}

pub fn is_swing_low(c: &[Candle], i: usize, left: usize, right: usize) -> bool {
    if i < left || i + right >= c.len() { return false; }
    let l = c[i].low;
    (1..=right).all(|k| l < c[i + k].low) && (1..=left).all(|k| l <= c[i - k].low)
}

/// Pivot newly confirmed by appending the candle at the last index.
pub fn newly_confirmed_high(c: &[Candle], left: usize, right: usize) -> Option<Pivot> {
    if c.len() <= right { return None; }
    let i = c.len() - 1 - right;
    if is_swing_high(c, i, left, right) {
        Some(Pivot { index: i as u64, time_ms: c[i].open_time_ms, price: c[i].high, broken: false })
    } else { None }
}

pub fn newly_confirmed_low(c: &[Candle], left: usize, right: usize) -> Option<Pivot> {
    if c.len() <= right { return None; }
    let i = c.len() - 1 - right;
    if is_swing_low(c, i, left, right) {
        Some(Pivot { index: i as u64, time_ms: c[i].open_time_ms, price: c[i].low, broken: false })
    } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn c(high: &str, low: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(low), high: dec(high), low: dec(low), close: dec(low) }
    }

    #[test]
    fn swing_high_confirmed_only_after_right_candles_close() {
        // highs: 1, 0.5, 2, 0.5, 0.4 -> index 2 is the swing high
        let candles = vec![c("1","0.5"), c("0.5","0.2"), c("2","1.5"), c("0.5","0.2"), c("0.4","0.1")];
        assert!(newly_confirmed_high(&candles[..4], 2, 2).is_none());
        let p = newly_confirmed_high(&candles[..5], 2, 2).unwrap();
        assert_eq!(p.index, 2);
        assert_eq!(p.price, dec("2"));
    }

    #[test]
    fn plateau_resolves_to_rightmost_member() {
        // highs: 2,2,2,1,0.5 -> index 2 is the only eligible swing high (strict right)
        let candles = vec![c("2","1"), c("2","1"), c("2","1"), c("1","0.5"), c("0.5","0.2")];
        let p = newly_confirmed_high(&candles, 2, 2).unwrap();
        assert_eq!(p.index, 2);
    }

    #[test]
    fn finds_swing_low() {
        // lows: 1.0, 1.2, 0.3, 1.1, 1.0 -> index 2
        let candles = vec![c("1.5","1.0"), c("1.5","1.2"), c("1.5","0.3"), c("1.5","1.1"), c("1.5","1.0")];
        let p = newly_confirmed_low(&candles, 2, 2).unwrap();
        assert_eq!(p.index, 2);
        assert_eq!(p.price, dec("0.3"));
    }
}
