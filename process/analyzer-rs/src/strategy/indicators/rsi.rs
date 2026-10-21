use crate::strategy::decimal::{add, div, neg, sub};
use crate::strategy::error::{Error, Result};
use crate::strategy::indicators::{ewm, Series};
use rust_decimal::Decimal;

/// Wilder's Relative Strength Index.
///
/// The first bar is `Some(0)` by oracle construction; a `None` close contributes a zero delta
/// (mirrors pandas `where`); the `1e-10` epsilon mirrors the Python guard; warm-up bars are
/// discarded downstream.
pub fn rsi(closes: &[Option<Decimal>], period: usize) -> Result<Series> {
    if period == 0 {
        return Err(Error::ConfigInvalid("rsi period must be >= 1".into()));
    }
    let mut delta = vec![None; closes.len()];
    for i in 1..closes.len() {
        if let (Some(now), Some(prev)) = (closes[i], closes[i - 1]) {
            delta[i] = Some(sub(now, prev)?);
        }
    }

    let mut gain = vec![Some(Decimal::ZERO); closes.len()];
    let mut loss = vec![Some(Decimal::ZERO); closes.len()];
    for (i, delta_step) in delta.iter().enumerate() {
        if let Some(step) = delta_step {
            if *step > Decimal::ZERO {
                gain[i] = Some(*step);
            } else if *step < Decimal::ZERO {
                loss[i] = Some(neg(*step)?);
            }
        }
    }

    let avg_gain = ewm(&gain, period)?;
    let avg_loss = ewm(&loss, period)?;
    let epsilon = Decimal::new(1, 10);
    let hundred = Decimal::ONE_HUNDRED;
    let mut out = vec![None; closes.len()];
    for i in 0..closes.len() {
        if let (Some(g), Some(l)) = (avg_gain[i], avg_loss[i]) {
            let rs = div(g, add(l, epsilon)?)?;
            let scaled_inverse = div(hundred, add(Decimal::ONE, rs)?)?;
            out[i] = Some(sub(hundred, scaled_inverse)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn close(v: &[&str]) -> Vec<Option<Decimal>> {
        v.iter().map(|x| Some(d(x))).collect()
    }

    fn assert_close(actual: &[Option<Decimal>], expected: &[&str]) {
        assert_eq!(actual.len(), expected.len());
        let tolerance = Decimal::from_str("0.00000001").unwrap();
        for (i, e) in expected.iter().enumerate() {
            let got = actual[i].unwrap();
            let want = d(e);
            let diff = (got - want).abs();
            assert!(diff <= tolerance, "i={i} got={got} want={want}");
        }
    }

    #[test]
    fn first_value_is_zero_like_pandas_where() {
        let values = close(&["10.0", "11.0"]);
        let out = rsi(&values, 3).unwrap();
        assert_eq!(out[0], Some(Decimal::ZERO));
    }

    #[test]
    fn matches_pandas_on_a_known_series() {
        let values = close(&[
            "10.0", "11.0", "10.5", "11.5", "12.0", "11.0", "12.5", "13.0",
        ]);
        let out = rsi(&values, 3).unwrap();
        let expected = [
            "0.0",
            "99.99999997",
            "57.142857128163264",
            "81.24999998628905",
            "86.81318679773216",
            "45.93023255165056",
            "73.74735355627921",
            "79.11872016991116",
        ];
        assert_close(&out, &expected);
    }

    #[test]
    fn none_closes_contribute_zero_delta_like_pandas_where() {
        let values = vec![
            None,
            Some(d("10.0")),
            Some(d("11.0")),
            None,
            Some(d("11.5")),
            Some(d("12.0")),
        ];
        let out = rsi(&values, 3).unwrap();
        let expected = [
            "0.0",
            "0.0",
            "99.99999997",
            "99.999999955",
            "99.9999999325",
            "99.99999996232557",
        ];
        assert_close(&out, &expected);
    }

    #[test]
    fn flat_series_is_zero() {
        let values = close(&["5.0", "5.0", "5.0", "5.0"]);
        let out = rsi(&values, 3).unwrap();
        for v in out.iter().flatten() {
            assert_eq!(*v, Decimal::ZERO);
        }
    }

    #[test]
    fn zero_period_is_rejected() {
        assert!(rsi(&close(&["1.0", "2.0"]), 0).is_err());
    }

    #[test]
    fn period_one_uses_the_raw_delta() {
        // alpha = 1 makes the EWM the raw gain/loss: up bars are ~100, down/flat bars are 0,
        // and the first bar has no delta.
        let values = close(&["10.0", "11.0", "10.0", "9.0", "9.0"]);
        let out = rsi(&values, 1).unwrap();
        let expected = ["0.0", "99.99999999", "0.0", "0.0", "0.0"];
        assert_close(&out, &expected);
    }

    #[test]
    fn period_longer_than_series_still_uses_all_bars() {
        // EWM seeds at the first bar regardless of period, so no bars are suppressed.
        let out = rsi(&close(&["10.0", "11.0"]), 3).unwrap();
        let expected = ["0.0", "99.99999997"];
        assert_close(&out, &expected);
    }

    #[test]
    fn single_element_is_zero() {
        // The only bar has no delta, so the oracle returns zero.
        let out = rsi(&close(&["10.0"]), 3).unwrap();
        assert_close(&out, &["0.0"]);
    }

    #[test]
    fn empty_input_is_empty() {
        let out: Series = rsi(&[], 3).unwrap();
        assert!(out.is_empty());
    }
}
