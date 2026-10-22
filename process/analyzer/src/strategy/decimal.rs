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
    Ok(a.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn mul_and_div_success_paths_work() {
        assert_eq!(
            mul(Decimal::from(2), Decimal::from(3)).unwrap(),
            Decimal::from(6)
        );
        assert_eq!(
            div(Decimal::from(6), Decimal::from(3)).unwrap(),
            Decimal::from(2)
        );
    }

    #[test]
    fn div_by_zero_is_an_error() {
        assert_eq!(div(Decimal::ONE, Decimal::ZERO), Err(Error::DivByZero));
    }

    #[test]
    fn overflow_is_an_error_on_every_operation() {
        let tiny = Decimal::new(1, 28);
        assert_eq!(add(Decimal::MAX, Decimal::ONE), Err(Error::Overflow));
        assert_eq!(sub(Decimal::MIN, Decimal::ONE), Err(Error::Overflow));
        assert_eq!(mul(Decimal::MAX, Decimal::from(2)), Err(Error::Overflow));
        assert_eq!(div(Decimal::MAX, tiny), Err(Error::Overflow));
    }

    #[test]
    fn neg_and_abs_work() {
        assert_eq!(neg(Decimal::from(5)).unwrap(), Decimal::from(-5));
        assert_eq!(abs(Decimal::from(-5)).unwrap(), Decimal::from(5));
        assert_eq!(abs(Decimal::from(5)).unwrap(), Decimal::from(5));
    }

    #[test]
    fn abs_normalizes_negative_zero() {
        let mut zero = Decimal::ZERO;
        zero.set_sign_negative(true);
        let result = abs(zero).unwrap();
        assert!(!result.is_sign_negative());
        assert_eq!(result, Decimal::ZERO);
    }
}
