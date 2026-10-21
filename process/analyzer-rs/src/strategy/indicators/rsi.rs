use crate::strategy::decimal::{add, div, neg, sub};
use crate::strategy::error::{Error, Result};
use crate::strategy::indicators::{ewm, Series};
use rust_decimal::Decimal;

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
    for (i, d) in delta.iter().enumerate() {
        if let Some(d) = d {
            if *d > Decimal::ZERO {
                gain[i] = Some(*d);
            } else if *d < Decimal::ZERO {
                loss[i] = Some(neg(*d)?);
            }
        }
    }

    let avg_gain = ewm(&gain, period)?;
    let avg_loss = ewm(&loss, period)?;
    let epsilon = Decimal::new(1, 10);
    let hundred = Decimal::from(100);
    let mut out = vec![None; closes.len()];
    for i in 0..closes.len() {
        if let (Some(g), Some(l)) = (avg_gain[i], avg_loss[i]) {
            let rs = div(g, add(l, epsilon)?)?;
            let ratio = div(hundred, add(Decimal::ONE, rs)?)?;
            out[i] = Some(sub(hundred, ratio)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn close(v: &[&str]) -> Vec<Option<Decimal>> {
        v.iter().map(|x| Some(d(x))).collect()
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
}
