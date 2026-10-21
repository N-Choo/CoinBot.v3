use crate::strategy::decimal::{abs, sub};
use crate::strategy::error::{Error, Result};
use crate::strategy::indicators::{ewm, Series};
use crate::strategy::types::Candle;

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
}
