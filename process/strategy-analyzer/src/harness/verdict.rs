//! Coded Phase 0 verdict: the pre-registered §10.4 bar, expressed as code with the thresholds
//! hard-coded (never re-derived from observed results). Diagnostics around it (perturbation, 1m
//! calibration) are separate modules.

use crate::harness::report::{time_blocked_bootstrap, TradeRecord, BOOTSTRAP_BLOCK_MS};
use rust_decimal::Decimal;

/// Pre-registered minimum number of *executed* OOS trades (skips do not count).
pub const MIN_EXECUTED_TRADES: usize = 200;
/// Pre-registered bootstrap resample count and seed (fixed before the OOS run).
pub const BOOTSTRAP_RESAMPLES: usize = 10_000;
pub const BOOTSTRAP_SEED: u64 = 12_345;

/// Regime by the 200-day average: `true` = uptrend. `closes` are daily-ish closes.
pub fn is_uptrend(closes: &[Decimal]) -> bool {
    if closes.len() < 200 {
        return false;
    }
    let mean200: Decimal = closes[closes.len() - 200..]
        .iter()
        .copied()
        .sum::<Decimal>()
        / Decimal::from(200);
    closes.last().is_some_and(|c| *c >= mean200)
}

/// Three-state outcome of the pre-registered §10.4 bar. `Insufficient` is deliberately distinct
/// from `Fail`: below `MIN_EXECUTED_TRADES` the bar cannot be evaluated, so the result is neither
/// a pass nor a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictOutcome {
    /// Fewer than `MIN_EXECUTED_TRADES` executed trades: the bar cannot be evaluated.
    Insufficient,
    /// Enough executed trades, but at least one gate failed.
    Fail,
    /// Enough executed trades and every gate passed.
    Pass,
}

#[derive(Debug, PartialEq)]
pub struct Verdict {
    pub n: usize,
    pub mean_r: f64,
    pub lower_95: f64,
    pub outcome: VerdictOutcome,
    pub inconclusive_single_regime: bool,
    pub concentrated: bool,
    pub cost_stress_ok: bool,
}

/// Evaluate the pre-registered §10.4 bar. Gates (all hard-coded): `n >= 200` executed trades,
/// composite mean net R > 0, time-blocked bootstrap 5th-percentile lower bound > 0, not a single
/// regime, no pair supplying more than 50% of total R, and the mean still positive at 1.5x costs.
///
/// `n < MIN_EXECUTED_TRADES` returns `Insufficient` (neither `Pass` nor `Fail`); otherwise the
/// outcome is `Pass`/`Fail` per the gates. The bootstrap is **calendar-blocked**: it resamples
/// whole blocks assigned by `TradeRecord.entry_time_ms` over the pre-registered
/// `BOOTSTRAP_BLOCK_MS` window.
pub fn verdict(
    composite: &[TradeRecord],
    one_regime_only: bool,
    total_r: Decimal,
    max_pair_r: Decimal,
    mean_r_at_1_5x_cost: f64,
) -> Verdict {
    let (mean_r, lower_95) = time_blocked_bootstrap(
        composite,
        BOOTSTRAP_BLOCK_MS,
        BOOTSTRAP_RESAMPLES,
        BOOTSTRAP_SEED,
    );
    // Concentration is strict: exactly 50% is allowed, only "more than 50%" trips the gate.
    let half = Decimal::new(5, 1); // 0.5
    let concentrated = total_r > Decimal::ZERO && (max_pair_r / total_r) > half;
    let cost_stress_ok = mean_r_at_1_5x_cost > 0.0;
    let outcome = if composite.len() < MIN_EXECUTED_TRADES {
        VerdictOutcome::Insufficient
    } else {
        let all_gates_pass =
            mean_r > 0.0 && lower_95 > 0.0 && !one_regime_only && !concentrated && cost_stress_ok;
        if all_gates_pass {
            VerdictOutcome::Pass
        } else {
            VerdictOutcome::Fail
        }
    };
    Verdict {
        n: composite.len(),
        mean_r,
        lower_95,
        outcome,
        inconclusive_single_regime: one_regime_only,
        concentrated,
        cost_stress_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Direction;
    use std::str::FromStr;

    fn rec(r: &str) -> TradeRecord {
        TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(r).unwrap(),
            pair: "A".into(),
            entry_time_ms: 0,
            exit_time_ms: 0,
        }
    }
    fn at(r: &str, day: i64) -> TradeRecord {
        TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(r).unwrap(),
            pair: "A".into(),
            entry_time_ms: day * 86_400_000,
            exit_time_ms: day * 86_400_000,
        }
    }
    fn sum_r(data: &[TradeRecord]) -> Decimal {
        data.iter().map(|t| t.r).sum()
    }

    #[test]
    fn healthy_sample_passes() {
        let data: Vec<_> = (0..220)
            .map(|i| if i % 4 == 0 { rec("-1.0") } else { rec("3.0") })
            .collect();
        let total = sum_r(&data);
        let v = verdict(&data, false, total, total / Decimal::from(2), 0.5);
        assert_eq!(v.outcome, VerdictOutcome::Pass, "{v:?}");
        assert_eq!(v.n, 220);
        assert!(!v.concentrated);
        assert!(v.cost_stress_ok);
    }

    #[test]
    fn trade_count_gate_is_a_hard_200() {
        // 199 executed trades is below the pre-registered bar: Insufficient, not Fail.
        let data: Vec<_> = (0..199).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        assert_eq!(
            verdict(&data, false, total, total / Decimal::from(2), 0.5).outcome,
            VerdictOutcome::Insufficient
        );
        // exactly 200 passes.
        let data: Vec<_> = (0..200).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        let v = verdict(&data, false, total, total / Decimal::from(2), 0.5);
        assert_eq!(v.outcome, VerdictOutcome::Pass, "n=200 must pass: {v:?}");
    }

    #[test]
    fn below_min_trades_is_insufficient_never_pass_or_fail() {
        let data: Vec<_> = (0..199).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        let v = verdict(&data, false, total, total / Decimal::from(2), 0.5);
        assert_eq!(v.outcome, VerdictOutcome::Insufficient);
        assert_ne!(v.outcome, VerdictOutcome::Pass);
        assert_ne!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn negative_cost_stress_fails_a_healthy_base_series() {
        // The base series is healthy at 1x cost, but its mean at 1.5x costs is negative -> Fail.
        let data: Vec<_> = (0..220).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        let v = verdict(&data, false, total, total / Decimal::from(2), -0.5);
        assert!(!v.cost_stress_ok);
        assert_eq!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn concentration_gate_is_strictly_over_half() {
        let data: Vec<_> = (0..220).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        // Exactly 50% is allowed (the gate is "> 50%").
        let at_half = verdict(&data, false, total, total / Decimal::from(2), 0.5);
        assert!(!at_half.concentrated);
        assert_eq!(at_half.outcome, VerdictOutcome::Pass, "{at_half:?}");
        // Just over 50% trips the gate.
        let over = verdict(
            &data,
            false,
            total,
            total / Decimal::from(2) + Decimal::new(1, 3),
            0.5,
        );
        assert!(over.concentrated);
        assert_eq!(over.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn cost_stress_gate_requires_strictly_positive() {
        let data: Vec<_> = (0..220).map(|_| rec("3.0")).collect();
        let total = sum_r(&data);
        let flat = verdict(&data, false, total, total / Decimal::from(2), 0.0);
        assert!(!flat.cost_stress_ok);
        assert_eq!(flat.outcome, VerdictOutcome::Fail);
        assert_eq!(
            verdict(&data, false, total, total / Decimal::from(2), 0.01).outcome,
            VerdictOutcome::Pass
        );
    }

    #[test]
    fn single_regime_and_concentration_block() {
        let data: Vec<_> = (0..220)
            .map(|i| if i % 4 == 0 { rec("-1.0") } else { rec("3.0") })
            .collect();
        let total = sum_r(&data);
        let one_regime = verdict(&data, true, total, total / Decimal::from(2), 0.5);
        assert_eq!(one_regime.outcome, VerdictOutcome::Fail); // a single regime is inconclusive
        assert!(one_regime.inconclusive_single_regime);
        let concentrated = verdict(&data, false, total, total, 0.5); // one pair holds 100% of R
        assert!(concentrated.concentrated);
        assert_eq!(concentrated.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn bootstrap_is_calendar_blocked_on_entry_time() {
        // Two calendar blocks with opposite means. A block bootstrap resamples whole blocks, so
        // a draw can land entirely on the negative block and the 5th percentile drops below the
        // overall mean; a per-trade bootstrap would not produce that dispersion.
        let mut data: Vec<TradeRecord> = (0..100).map(|_| at("3.0", 0)).collect();
        data.extend((0..100).map(|_| at("-1.0", 100))); // > 14 days later -> separate block
        let (mean, lower) = time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 10_000, 12_345);
        assert!((mean - 1.0).abs() < 1e-9, "portfolio mean = {mean}");
        assert!(
            lower < mean,
            "calendar-block lower bound must be below the mean, got {lower}"
        );
    }

    #[test]
    fn is_uptrend_uses_the_trailing_200_day_mean() {
        let mut up: Vec<Decimal> = (0..199).map(Decimal::from).collect();
        up.push(Decimal::from(1000));
        assert!(is_uptrend(&up));
        let mut down: Vec<Decimal> = up[..199].to_vec();
        down.push(Decimal::from(0));
        assert!(!is_uptrend(&down));
        assert!(!is_uptrend(&up[..199])); // fewer than 200 closes is not an uptrend
    }
}
