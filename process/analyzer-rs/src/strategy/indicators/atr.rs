use crate::strategy::decimal::{abs, sub};
use crate::strategy::error::{Error, Result};
use crate::strategy::indicators::{ewm, Series};
use crate::strategy::types::Candle;

/// Average True Range.
///
/// True range is `max(high - low, |high - prev_close|, |low - prev_close|)`. The first bar has no
/// previous close, so its true range is simply `high - low`. True range is then smoothed with the
/// same `ewm(alpha = 1/period, adjust = False)` used by the Python oracle (Wilder-style RMA),
/// seeded at the first bar.
pub fn atr(candles: &[Candle], period: usize) -> Result<Series> {
    if period == 0 {
        return Err(Error::ConfigInvalid("atr period must be >= 1".into()));
    }
    let mut true_range = vec![None; candles.len()];
    for (i, candle) in candles.iter().enumerate() {
        let high_low = sub(candle.high, candle.low)?;
        if i == 0 {
            true_range[i] = Some(high_low);
        } else {
            let prev = candles[i - 1].close;
            let high_close = abs(sub(candle.high, prev)?)?;
            let low_close = abs(sub(candle.low, prev)?)?;
            let mut max = high_low;
            if high_close > max {
                max = high_close;
            }
            if low_close > max {
                max = low_close;
            }
            true_range[i] = Some(max);
        }
    }
    ewm(&true_range, period)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Candle;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn candle(high: &str, low: &str, close: &str) -> Candle {
        Candle {
            open_time_ms: 0,
            open: d(close),
            high: d(high),
            low: d(low),
            close: d(close),
            volume: Decimal::ONE,
        }
    }

    #[test]
    fn first_true_range_is_high_minus_low() {
        let candles = vec![candle("11", "9", "10")];
        let out = atr(&candles, 3).unwrap();
        assert_eq!(out[0], Some(Decimal::from(2)));
    }

    #[test]
    fn matches_pandas_on_a_known_series() {
        let closes = ["10", "11", "10.5", "11.5", "12", "11", "12.5", "13"];
        let highs = ["11", "12", "11.5", "12.5", "13", "12", "13.5", "14"];
        let lows = ["9", "10", "10", "10.5", "11", "10.5", "11.5", "12"];
        let candles: Vec<Candle> = (0..closes.len())
            .map(|i| candle(highs[i], lows[i], closes[i]))
            .collect();
        let out = atr(&candles, 3).unwrap();
        let expected = [
            "2.0",
            "2.0",
            "1.8333333333333335",
            "1.888888888888889",
            "1.925925925925926",
            "1.7839506172839505",
            "2.022633744855967",
            "2.0150891632373114",
        ];
        for (i, e) in expected.iter().enumerate() {
            let got = out[i].unwrap();
            let want = d(e);
            let diff = (got - want).abs();
            assert!(
                diff <= Decimal::from_str("0.00000001").unwrap(),
                "i={i} got={got} want={want}"
            );
        }
    }

    #[test]
    fn zero_period_is_rejected() {
        assert!(atr(&[candle("2", "1", "1.5")], 0).is_err());
    }

    #[test]
    fn empty_input_is_empty() {
        let out = atr(&[], 3).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn single_candle_is_high_minus_low_regardless_of_period() {
        // The first bar has no previous close, so TR is just high - low and the EWM seeds with it.
        for period in [1, 3, 50] {
            let out = atr(&[candle("11", "9", "10")], period).unwrap();
            assert_eq!(out, vec![Some(Decimal::from(2))], "period={period}");
        }
    }

    #[test]
    fn period_longer_than_series_still_uses_all_bars() {
        // EWM seeds at the first bar regardless of period, so no bars are suppressed.
        let candles = vec![candle("11", "9", "10"), candle("13", "11", "12")];
        let out = atr(&candles, 10).unwrap();
        // TR is [2, 3]; alpha = 1/10 gives 0.1 * 3 + 0.9 * 2 = 2.1.
        assert_eq!(out, vec![Some(Decimal::from(2)), Some(d("2.1"))]);
    }

    #[test]
    fn flat_series_is_zero() {
        let candles = vec![
            candle("5", "5", "5"),
            candle("5", "5", "5"),
            candle("5", "5", "5"),
        ];
        let out = atr(&candles, 3).unwrap();
        for v in out.iter().flatten() {
            assert_eq!(*v, Decimal::ZERO);
        }
    }

    #[test]
    fn period_one_uses_the_raw_true_range() {
        // alpha = 1 makes the EWM the raw true range.
        let candles = vec![
            candle("11", "9", "10"),
            candle("13", "10", "11"),
            candle("12", "11", "11.5"),
        ];
        let out = atr(&candles, 1).unwrap();
        assert_eq!(
            out,
            vec![
                Some(Decimal::from(2)),
                Some(Decimal::from(3)),
                Some(Decimal::from(1))
            ]
        );
    }
}
