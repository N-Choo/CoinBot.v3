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

use crate::strategy::config::Config;
use crate::strategy::types::{StrategyState, TrendState};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BosKind { Bullish, Bearish }

/// Detect a BOS on the just-closed candle using the §6.3 eligibility table.
/// `atr_before` must be the ATR through the *previous* candle (no look-ahead).
/// Returns the broken kind and the absolute index of the broken level.
pub fn detect_bos(state: &StrategyState, atr_before: Decimal, cfg: &Config) -> Option<(BosKind, u64)> {
    let close = state.window.back()?.close;
    let dist = cfg.bos_min_distance_atr * atr_before;
    let recent_high = state.swing_highs.iter().filter(|p| !p.broken).max_by_key(|p| p.index);
    let recent_low = state.swing_lows.iter().filter(|p| !p.broken).max_by_key(|p| p.index);
    let high_price = |idx: u64| state.swing_highs.iter().find(|p| p.index == idx).map(|p| p.price);
    let low_price = |idx: u64| state.swing_lows.iter().find(|p| p.index == idx).map(|p| p.price);

    // Bullish candidate: in Bearish only valid_high (reversal); otherwise the most recent unbroken high.
    let bull = match state.trend {
        TrendState::Bearish => state.valid_high_idx.and_then(|i| high_price(i).map(|p| (p, i))),
        _ => recent_high.map(|h| (h.price, h.index)),
    };
    if let Some((price, idx)) = bull {
        if close >= price + dist { return Some((BosKind::Bullish, idx)); }
    }
    // Bearish candidate: in Bullish only valid_low (reversal); otherwise the most recent unbroken low.
    let bear = match state.trend {
        TrendState::Bullish => state.valid_low_idx.and_then(|i| low_price(i).map(|p| (p, i))),
        _ => recent_low.map(|l| (l.price, l.index)),
    };
    if let Some((price, idx)) = bear {
        if close <= price - dist { return Some((BosKind::Bearish, idx)); }
    }
    None
}

#[cfg(test)]
mod bos_tests {
    use super::*;
    use crate::strategy::types::{Candle, Pivot, StrategyState, TrendState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    fn state_with(close: &str, highs: Vec<(u64, &str)>, lows: Vec<(u64, &str)>, trend: TrendState, vl: Option<u64>, vh: Option<u64>) -> StrategyState {
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.window.push_back(Candle { open_time_ms: 0, open: dec(close), high: dec(close), low: dec(close), close: dec(close) });
        s.swing_highs = highs.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: dec(p), broken: false }).collect();
        s.swing_lows = lows.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: dec(p), broken: false }).collect();
        s.trend = trend; s.valid_low_idx = vl; s.valid_high_idx = vh;
        s
    }

    #[test]
    fn bullish_bos_reports_most_recent_cleared_high() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = state_with("2.6", vec![(0, "2.0"), (1, "2.3")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), Some((BosKind::Bullish, 1)));
    }

    #[test]
    fn bos_equality_boundary_fires() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // close == price + dist exactly (2.0 + 0.1)
        let s = state_with("2.1", vec![(0, "2.0")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), Some((BosKind::Bullish, 0)));
    }

    #[test]
    fn bullish_trend_does_not_reverse_on_arbitrary_low() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // In Bullish, only valid_low triggers a bearish reversal; an unbroken low must NOT fire.
        let s = state_with("1.0", vec![], vec![(0, "0.5")], TrendState::Bullish, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), None);
    }

    #[test]
    fn no_bos_when_distance_not_met() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = state_with("2.0", vec![(0, "2.0")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), None);
    }
}
