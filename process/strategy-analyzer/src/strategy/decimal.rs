use rust_decimal::{Decimal, RoundingStrategy};

pub const WORKING_SCALE: u32 = 12;
pub const RATIO_SCALE: u32 = 4;

/// Half-up rounding. rust_decimal defaults to half-even, so request explicitly.
pub fn round_to(value: Decimal, scale: u32) -> Decimal {
    value.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

/// Round a level to the symbol's price scale (half-up).
pub fn round_price(value: Decimal, price_scale: u32) -> Decimal {
    round_to(value, price_scale)
}

/// Round a ratio for serialization/reporting only (toward zero).
pub fn round_ratio_for_output(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(RATIO_SCALE, RoundingStrategy::ToZero)
}

/// Round an intermediate indicator value to the working scale.
pub fn work(value: Decimal) -> Decimal {
    round_to(value, WORKING_SCALE)
}

/// Which valid multiple of an increment a level snaps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapSide {
    /// Round down (toward −∞) to a multiple of the increment.
    Floor,
    /// Round up (toward +∞) to a multiple of the increment.
    Ceil,
    /// Nearest multiple, ties half-up (matches `round_to`).
    Nearest,
}

/// Snap `value` to a valid multiple of the symbol's `price_increment` (e.g. KuCoin `0.0005`, a
/// step-5 increment that a dp `price_scale` cannot express). `increment` must be positive; callers
/// only invoke this when an explicit increment is configured, otherwise they fall back to
/// `round_price` at the symbol's dp scale. The quotient is rounded half-up for `Nearest` to match
/// the rest of the level pipeline (`round_to`).
pub fn snap_to_increment(value: Decimal, increment: Decimal, side: SnapSide) -> Decimal {
    debug_assert!(increment > Decimal::ZERO, "increment must be positive");
    let steps = match side {
        SnapSide::Floor => (value / increment).floor(),
        SnapSide::Ceil => (value / increment).ceil(),
        SnapSide::Nearest => round_to(value / increment, 0),
    };
    work(steps * increment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn half_up_not_half_even() {
        assert_eq!(round_to(dec("2.5"), 0), Decimal::from(3));
        assert_eq!(round_to(dec("3.5"), 0), Decimal::from(4));
    }

    #[test]
    fn price_scale_respected() {
        assert_eq!(round_price(dec("1.23456"), 2), dec("1.23"));
    }

    #[test]
    fn ratio_output_rounds_toward_zero() {
        // 2.99996 -> 2.9999, so a 3.0 floor is not passed by rounding.
        assert_eq!(round_ratio_for_output(dec("2.99996")), dec("2.9999"));
    }

    #[test]
    fn snap_to_increment_floor_ceil_and_nearest() {
        // KuCoin increments can be step-5 at 4 dp (0.0005); a dp scale alone can't express them.
        assert_eq!(
            snap_to_increment(dec("1.0002"), dec("0.0005"), SnapSide::Floor),
            dec("1.0000")
        );
        assert_eq!(
            snap_to_increment(dec("1.0002"), dec("0.0005"), SnapSide::Ceil),
            dec("1.0005")
        );
        assert_eq!(
            snap_to_increment(dec("1.0002"), dec("0.0005"), SnapSide::Nearest),
            dec("1.0000")
        );
        assert_eq!(
            snap_to_increment(dec("1.0003"), dec("0.0005"), SnapSide::Nearest),
            dec("1.0005")
        );
        // A value already on the grid is unchanged by every side.
        for side in [SnapSide::Floor, SnapSide::Ceil, SnapSide::Nearest] {
            assert_eq!(
                snap_to_increment(dec("1.0005"), dec("0.0005"), side),
                dec("1.0005")
            );
        }
    }

    #[test]
    fn snap_to_increment_away_and_toward_differ_at_a_coarse_step() {
        // Long stop raw 103.87 at a 0.1 step: away-from-entry floors to 103.8, toward-entry
        // (or nearest) reaches 103.9.
        assert_eq!(
            snap_to_increment(dec("103.87"), dec("0.1"), SnapSide::Floor),
            dec("103.8")
        );
        assert_eq!(
            snap_to_increment(dec("103.87"), dec("0.1"), SnapSide::Ceil),
            dec("103.9")
        );
        assert_eq!(
            snap_to_increment(dec("103.87"), dec("0.1"), SnapSide::Nearest),
            dec("103.9")
        );
    }
}
