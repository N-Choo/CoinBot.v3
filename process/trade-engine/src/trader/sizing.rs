//! Order sizing: per-order budget and exchange-compliant quantity. Pure functions, no I/O.

use rust_decimal::Decimal;

/// Divisor applied to a contract's initial fund to get the per-order budget.
const ORDER_FUND_DIVISOR: u32 = 2;

/// Budget for a single order: `init_fund / ORDER_FUND_DIVISOR` (half).
pub fn order_budget(init_fund: Decimal) -> Decimal {
    init_fund / Decimal::from(ORDER_FUND_DIVISOR)
}

/// Order size: `available / price`, floored to a multiple of `base_increment`.
///
/// Returns `None` when the inputs are non-positive, the pair has no valid
/// increment, the computed step count is zero, or the size is below
/// `base_min_size`.
pub(super) fn compute_quantity(
    available: f64,
    price: f64,
    base_increment: &str,
    base_min_size: &str,
) -> Option<String> {
    if price <= 0.0 || available <= 0.0 {
        return None;
    }
    let increment: f64 = base_increment.parse().ok()?;
    if !increment.is_finite() || increment <= 0.0 {
        return None;
    }

    let steps = ((available / price) / increment).floor();
    if steps < 1.0 {
        return None;
    }

    let size = steps * increment;
    let min_size: f64 = base_min_size.parse().unwrap_or(0.0);
    if size < min_size {
        return None;
    }

    Some(format!("{:.*}", decimal_scale(base_increment), size))
}

/// Number of fractional digits in a decimal string, e.g. `"0.001"` -> `3`.
fn decimal_scale(decimal: &str) -> usize {
    decimal.split_once('.').map_or(0, |(_, frac)| frac.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_size_to_the_pair_increment() {
        // SOL-style: 0.001 increment.
        assert_eq!(
            compute_quantity(7.993424, 80.0, "0.001", "0.001").as_deref(),
            Some("0.099")
        );
        // BTC-style: 1e-8 increment (float division rounds down one step, still valid).
        assert_eq!(
            compute_quantity(8.36, 83_600.0, "0.00000001", "0.000001").as_deref(),
            Some("0.00009999")
        );
    }

    #[test]
    fn rejects_sizes_below_one_increment() {
        // 7.993424 / 111.6 = 0.0716 -> 0 whole 0.1 steps.
        assert_eq!(compute_quantity(7.993424, 111.6, "0.1", "0.1"), None);
    }

    #[test]
    fn rejects_sizes_below_min_size() {
        assert_eq!(compute_quantity(15.0, 100.0, "0.1", "1.0"), None);
    }

    #[test]
    fn rejects_invalid_or_non_positive_inputs() {
        assert_eq!(compute_quantity(0.0, 100.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 0.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, -1.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 100.0, "not-a-number", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 100.0, "0", "0.1"), None);
    }

    #[test]
    fn counts_decimal_scale() {
        assert_eq!(decimal_scale("0.00000001"), 8);
        assert_eq!(decimal_scale("0.001"), 3);
        assert_eq!(decimal_scale("0.1"), 1);
        assert_eq!(decimal_scale("1"), 0);
    }
}
