use crate::strategy::error::{Error, Result};
use rust_decimal::Decimal;

pub fn add(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_add(b).ok_or(Error::Overflow)
}

pub fn sub(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_sub(b).ok_or(Error::Overflow)
}

pub fn mul(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_mul(b).ok_or(Error::Overflow)
}

pub fn div(a: Decimal, b: Decimal) -> Result<Decimal> {
    if b == Decimal::ZERO {
        return Err(Error::DivByZero);
    }
    a.checked_div(b).ok_or(Error::Overflow)
}

pub fn neg(a: Decimal) -> Result<Decimal> {
    Decimal::ZERO.checked_sub(a).ok_or(Error::Overflow)
}

pub fn abs(a: Decimal) -> Result<Decimal> {
    if a < Decimal::ZERO {
        neg(a)
    } else {
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn add_and_sub_work() {
        assert_eq!(
            add(Decimal::from(2), Decimal::from(3)).unwrap(),
            Decimal::from(5)
        );
        assert_eq!(
            sub(Decimal::from(2), Decimal::from(3)).unwrap(),
            Decimal::from(-1)
        );
    }

    #[test]
    fn div_by_zero_is_an_error() {
        assert_eq!(div(Decimal::ONE, Decimal::ZERO), Err(Error::DivByZero));
    }

    #[test]
    fn overflow_is_an_error() {
        assert_eq!(add(Decimal::MAX, Decimal::ONE), Err(Error::Overflow));
    }

    #[test]
    fn neg_and_abs_work() {
        assert_eq!(neg(Decimal::from(5)).unwrap(), Decimal::from(-5));
        assert_eq!(abs(Decimal::from(-5)).unwrap(), Decimal::from(5));
        assert_eq!(abs(Decimal::from(5)).unwrap(), Decimal::from(5));
    }
}
