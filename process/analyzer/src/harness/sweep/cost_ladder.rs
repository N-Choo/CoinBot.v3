use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::sampling::{baseline, Candidate};
use rust_decimal::Decimal;

/// Per-side cost levels in tenths of a basis point.
pub const LEVELS_TENTHS_BPS: [u32; 7] = [0, 25, 50, 75, 100, 150, 300];

/// Baseline plus the four 4hour screening candidates frozen for the cost ladder.
pub fn frozen_candidates() -> Vec<Candidate> {
    let mut out = vec![baseline()];
    let mut push = |label: &str, long: i64, short: i64, atr: bool, sl: Decimal, tp: Decimal| {
        out.push(Candidate {
            label: label.into(),
            origin: "screening-4hour".into(),
            rsi_period: 21,
            rsi_long_threshold: Decimal::from(long),
            rsi_short_threshold: Decimal::from(short),
            atr_enabled: atr,
            atr_sl_multiplier: sl,
            atr_tp_multiplier: tp,
        });
    };
    push(
        "long65-short50",
        65,
        50,
        false,
        Decimal::new(20, 1),
        Decimal::new(30, 1),
    );
    push(
        "long60-short45",
        60,
        45,
        false,
        Decimal::new(20, 1),
        Decimal::new(30, 1),
    );
    push(
        "long70-short50",
        70,
        50,
        false,
        Decimal::new(20, 1),
        Decimal::new(30, 1),
    );
    push(
        "atr-on-tp4.0",
        60,
        50,
        true,
        Decimal::new(20, 1),
        Decimal::new(40, 1),
    );
    out
}

/// One all-in per-side cost scenario per ladder level; there is no separate slippage.
pub fn scenarios() -> Vec<CostScenario> {
    LEVELS_TENTHS_BPS
        .iter()
        .map(|tenths| CostScenario {
            label: format!("{}.{}bps", tenths / 10, tenths % 10),
            fee_bps: Decimal::new(*tenths as i64, 1),
            slip_bps: Decimal::ZERO,
        })
        .collect()
}

/// Interpolates the per-side cost where pooled net expectancy crosses zero.
///
/// `points` must be ordered by ascending cost. Returns `Some(0)` when the first
/// level is already non-positive and `None` when expectancy is still positive at
/// the highest level.
pub fn break_even_bps(points: &[(Decimal, Decimal)]) -> Option<Decimal> {
    if points.is_empty() {
        return None;
    }
    if points[0].1 <= Decimal::ZERO {
        return Some(Decimal::ZERO);
    }
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if y0 > Decimal::ZERO && y1 <= Decimal::ZERO {
            return Some(x0 + (x1 - x0) * y0 / (y0 - y1));
        }
    }
    None
}
