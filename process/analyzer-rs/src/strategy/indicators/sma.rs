use crate::strategy::decimal::{add, div};
use crate::strategy::error::{Error, Result};
use crate::strategy::indicators::Series;
use rust_decimal::Decimal;

pub fn sma(values: &[Option<Decimal>], period: usize) -> Result<Series> {
    if period == 0 {
        return Err(Error::ConfigInvalid("sma period must be >= 1".into()));
    }
    let mut out = vec![None; values.len()];
    if values.len() < period {
        return Ok(out);
    }
    for i in (period - 1)..values.len() {
        let mut sum = Decimal::ZERO;
        let mut valid = true;
        for value in &values[i + 1 - period..=i] {
            match value {
                Some(v) => sum = add(sum, *v)?,
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            out[i] = Some(div(sum, Decimal::from(period))?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    #[test]
    fn simple_moving_average_has_pandas_warmup() {
        let values = vec![Some(d(1)), Some(d(2)), Some(d(3)), Some(d(4))];
        let out = sma(&values, 2).unwrap();
        assert_eq!(
            out,
            vec![
                None,
                Some(Decimal::new(15, 1)),
                Some(Decimal::new(25, 1)),
                Some(Decimal::new(35, 1))
            ]
        );
    }

    #[test]
    fn a_none_in_the_window_makes_the_average_none() {
        let values = vec![Some(d(1)), None, Some(d(3))];
        let out = sma(&values, 2).unwrap();
        assert_eq!(out, vec![None, None, None]);
    }

    #[test]
    fn zero_period_is_rejected() {
        assert!(sma(&[Some(d(1))], 0).is_err());
    }

    #[test]
    fn a_none_leaving_the_window_restores_the_average() {
        let values = vec![Some(d(1)), None, Some(d(3)), Some(d(4))];
        let out = sma(&values, 2).unwrap();
        assert_eq!(out, vec![None, None, None, Some(Decimal::new(35, 1))]);
    }

    #[test]
    fn shorter_than_period_is_all_none() {
        let values = vec![Some(d(1)), Some(d(2))];
        let out = sma(&values, 3).unwrap();
        assert_eq!(out, vec![None, None]);
    }
}
