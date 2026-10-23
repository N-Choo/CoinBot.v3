use crate::strategy::decimal::{add, div, mul, sub};
use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;

pub mod atr;
pub mod rsi;

pub type Series = Vec<Option<Decimal>>;

/// pandas-style `ewm(alpha = 1/period, adjust = False)`.
///
/// For NaN-free input (the RSI/ATR use) this matches
/// `Series.ewm(alpha=1/period, adjust=False).mean()` exactly.
///
/// `None` positions emit `None` and do **not** advance the recursion (equivalent to pandas
/// `ignore_na=True`): a gap does not decay the previous weight by its length. The Python oracle
/// uses `ignore_na=False`, but its RSI/ATR inputs contain no NaN, so the two agree on every input
/// this crate passes. Callers feeding sparse series must not assume default-pandas gap decay.
pub fn ewm(values: &[Option<Decimal>], period: usize) -> Result<Series> {
    if period == 0 {
        return Err(Error::ConfigInvalid("ewm period must be >= 1".into()));
    }
    let alpha = div(Decimal::ONE, Decimal::from(period))?;
    let one_minus = sub(Decimal::ONE, alpha)?;
    let mut out = vec![None; values.len()];
    let mut prev: Option<Decimal> = None;
    for (i, value) in values.iter().enumerate() {
        match (value, prev) {
            (Some(x), Some(p)) => {
                let next = add(mul(alpha, *x)?, mul(one_minus, p)?)?;
                out[i] = Some(next);
                prev = Some(next);
            }
            (Some(x), None) => {
                out[i] = Some(*x);
                prev = Some(*x);
            }
            (None, _) => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    fn assert_series_close(actual: &[Option<Decimal>], expected: &[Option<Decimal>]) {
        assert_eq!(actual.len(), expected.len());
        let tolerance = Decimal::new(1, 9);
        for (got, want) in actual.iter().zip(expected) {
            match (got, want) {
                (Some(g), Some(w)) => {
                    assert!((*g - *w).abs() <= tolerance, "expected {w}, got {g}")
                }
                (None, None) => {}
                _ => panic!("expected {want:?}, got {got:?}"),
            }
        }
    }

    #[test]
    fn ewm_simple_recursion() {
        let values = vec![Some(d(1)), Some(d(2)), Some(d(3))];
        let out = ewm(&values, 2).unwrap();
        assert_eq!(
            out,
            vec![
                Some(d(1)),
                Some(Decimal::new(15, 1)),
                Some(Decimal::new(225, 2))
            ]
        );
    }

    #[test]
    fn ewm_zero_period_is_rejected() {
        assert!(ewm(&[Some(d(1))], 0).is_err());
    }

    #[test]
    fn ewm_leading_none_seeds_at_first_valid() {
        let values = vec![None, Some(d(2)), Some(d(3))];
        let out = ewm(&values, 2).unwrap();
        assert_eq!(out, vec![None, Some(d(2)), Some(Decimal::new(25, 1))]);
    }

    #[test]
    fn ewm_trailing_none_stays_none() {
        let values = vec![Some(d(1)), Some(d(2)), None];
        let out = ewm(&values, 2).unwrap();
        assert_eq!(out, vec![Some(d(1)), Some(Decimal::new(15, 1)), None]);
    }

    #[test]
    fn ewm_all_none_is_all_none() {
        let values = vec![None, None];
        let out = ewm(&values, 2).unwrap();
        assert_eq!(out, vec![None, None]);
    }

    #[test]
    fn ewm_internal_none_uses_gap_independent_decay() {
        let values = vec![Some(d(1)), Some(d(2)), None, None, Some(d(4))];
        let out = ewm(&values, 3).unwrap();
        let expected = vec![
            Some(d(1)),
            Some(Decimal::from(4) / Decimal::from(3)),
            None,
            None,
            Some(Decimal::from(20) / Decimal::from(9)),
        ];
        assert_series_close(&out, &expected);
    }

    #[test]
    fn ewm_period_one_is_identity() {
        let values = vec![Some(d(1)), None, Some(d(3))];
        let out = ewm(&values, 1).unwrap();
        assert_eq!(out, vec![Some(d(1)), None, Some(d(3))]);
    }
}
