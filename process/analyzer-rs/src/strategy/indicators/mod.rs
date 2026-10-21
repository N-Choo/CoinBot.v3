use crate::strategy::decimal::{add, div, mul, sub};
use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;

pub mod atr;
pub mod peaks;
pub mod rsi;
pub mod sma;

pub type Series = Vec<Option<Decimal>>;

/// pandas `ewm(alpha=1/period, adjust=False)` with `skipna=True`.
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
