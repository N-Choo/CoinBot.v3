//! Coded Phase 0 verdict: the pre-registered §10.4 bar, expressed as code with the thresholds
//! hard-coded (never re-derived from observed results). Diagnostics around it (perturbation, 1m
//! calibration) are separate modules.
//!
//! The concentration/cost-stress statistics are **derived from the trade ledger** (never
//! caller-supplied point values), and the regime is a three-state input: a regime that cannot be
//! computed fails **closed** (`Inconclusive`), never defaulting to a pass.

use crate::harness::report::{time_blocked_bootstrap, TradeRecord, BOOTSTRAP_BLOCK_MS};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Pre-registered minimum number of *executed* OOS trades (skips do not count).
pub const MIN_EXECUTED_TRADES: usize = 200;
/// Pre-registered bootstrap resample count and seed (fixed before the OOS run). This is the **only**
/// bootstrap seed in the crate; the lock records it (spec §10.4).
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

/// Three-state regime of the segment the verdict is computed over. It is deliberately not a
/// `bool`: `Unknown` means the regime could not be computed, which must not be treated as "two
/// regimes" (the old `one_regime_only = false` default). Only `Both` can pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Both an up and a down regime are present.
    Both,
    /// Only one regime is present -> inconclusive.
    OneRegimeOnly,
    /// The regime could not be computed (too little history) -> inconclusive.
    Unknown,
}

/// Outcome of the pre-registered §10.4 bar. `Insufficient` (too few trades) and `Inconclusive`
/// (regime not confirmable) are deliberately distinct from `Fail`: the bar cannot be evaluated, so
/// the result is neither a pass nor a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictOutcome {
    /// Fewer than `MIN_EXECUTED_TRADES` executed trades: the bar cannot be evaluated.
    Insufficient,
    /// Enough executed trades, but the regime is not confirmable: fails closed.
    Inconclusive,
    /// Enough executed trades, but at least one gate failed.
    Fail,
    /// Enough executed trades and every gate passed.
    Pass,
}

/// Concentration and cost-stress statistics, **derived from the trades** (ITEM 7) so the gates can
/// never be fed numbers inconsistent with the ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct VerdictStats {
    pub total_r: Decimal,
    pub max_pair_r: Decimal,
    pub mean_r_at_1_5x_cost: f64,
}

/// Mean net R in `f64` for reporting/gates (the ledger stays `Decimal`).
pub fn mean_r(trades: &[TradeRecord]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }
    trades
        .iter()
        .map(|t| t.r.to_string().parse::<f64>().unwrap())
        .sum::<f64>()
        / trades.len() as f64
}

/// Derive `total_r`, `max_pair_r` (the largest per-pair R total), and `mean_r_at_1_5x_cost` from
/// the trade sets themselves.
pub fn derive_stats(composite: &[TradeRecord], composite_at_1_5x: &[TradeRecord]) -> VerdictStats {
    let total_r: Decimal = composite.iter().map(|t| t.r).sum();
    let mut pair_totals: BTreeMap<&str, Decimal> = BTreeMap::new();
    for t in composite {
        *pair_totals.entry(t.pair.as_str()).or_default() += t.r;
    }
    let max_pair_r = pair_totals.values().copied().max().unwrap_or(Decimal::ZERO);
    VerdictStats {
        total_r,
        max_pair_r,
        mean_r_at_1_5x_cost: mean_r(composite_at_1_5x),
    }
}

/// The trade sets the verdict is computed from. `composite` is the adversarial composite (longs
/// from `low_first`, shorts from `high_first`); `low_first`/`high_first` are the full per-policy
/// sets for the "positive under both path policies" gate; `composite_at_1_5x` is the same composite
/// rerun at 1.5× costs.
pub struct VerdictInputs<'a> {
    pub composite: &'a [TradeRecord],
    pub low_first: &'a [TradeRecord],
    pub high_first: &'a [TradeRecord],
    pub composite_at_1_5x: &'a [TradeRecord],
    pub regime: Regime,
}

#[derive(Debug, PartialEq)]
pub struct Verdict {
    pub n: usize,
    pub mean_r: f64,
    pub lower_95: f64,
    pub outcome: VerdictOutcome,
    pub regime: Regime,
    pub inconclusive_single_regime: bool,
    pub concentrated: bool,
    pub cost_stress_ok: bool,
    pub both_policies_positive: bool,
    pub total_r: Decimal,
    pub max_pair_r: Decimal,
    pub mean_r_at_1_5x_cost: f64,
}

/// Evaluate the pre-registered §10.4 bar. Gates (all hard-coded): `n >= 200` executed trades,
/// composite mean net R > 0, time-blocked bootstrap 5th-percentile lower bound > 0, **positive
/// under both path policies**, not a single regime (three-state; anything but `Both` fails closed),
/// no pair supplying more than 50% of total R, and the mean still positive at 1.5× costs.
///
/// `n < MIN_EXECUTED_TRADES` returns `Insufficient`; a non-`Both` regime returns `Inconclusive`;
/// otherwise the outcome is `Pass`/`Fail` per the gates. The bootstrap is **calendar-blocked**: it
/// resamples whole blocks assigned by `TradeRecord.entry_time_ms` over the pre-registered
/// `BOOTSTRAP_BLOCK_MS` window.
pub fn verdict(inputs: &VerdictInputs) -> Verdict {
    let composite = inputs.composite;
    let (composite_mean, lower_95) = time_blocked_bootstrap(
        composite,
        BOOTSTRAP_BLOCK_MS,
        BOOTSTRAP_RESAMPLES,
        BOOTSTRAP_SEED,
    );
    let stats = derive_stats(composite, inputs.composite_at_1_5x);
    // Concentration is strict: exactly 50% is allowed, only "more than 50%" trips the gate.
    let half = Decimal::new(5, 1); // 0.5
    let concentrated = stats.total_r > Decimal::ZERO && (stats.max_pair_r / stats.total_r) > half;
    let cost_stress_ok = stats.mean_r_at_1_5x_cost > 0.0;
    // §10.4 "positive under both path policies", a gate separate from the composite bound.
    let both_policies_positive = mean_r(inputs.low_first) > 0.0 && mean_r(inputs.high_first) > 0.0;
    let outcome = if composite.len() < MIN_EXECUTED_TRADES {
        VerdictOutcome::Insufficient
    } else if inputs.regime != Regime::Both {
        // Fail closed: a regime we cannot confirm as two-sided is inconclusive, never a pass.
        VerdictOutcome::Inconclusive
    } else {
        let all_gates_pass = composite_mean > 0.0
            && lower_95 > 0.0
            && both_policies_positive
            && !concentrated
            && cost_stress_ok;
        if all_gates_pass {
            VerdictOutcome::Pass
        } else {
            VerdictOutcome::Fail
        }
    };
    Verdict {
        n: composite.len(),
        mean_r: composite_mean,
        lower_95,
        outcome,
        regime: inputs.regime,
        inconclusive_single_regime: inputs.regime != Regime::Both,
        concentrated,
        cost_stress_ok,
        both_policies_positive,
        total_r: stats.total_r,
        max_pair_r: stats.max_pair_r,
        mean_r_at_1_5x_cost: stats.mean_r_at_1_5x_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Direction;
    use std::str::FromStr;

    fn rec_pair(r: &str, pair: &str) -> TradeRecord {
        TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(r).unwrap(),
            pair: pair.into(),
            entry_time_ms: 0,
            exit_time_ms: 0,
        }
    }
    fn rec(r: &str) -> TradeRecord {
        rec_pair(r, "A")
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
    /// `n` trades on `pair`, 3 in 4 winning +3R and 1 in 4 losing −1R (mean +2R/trade).
    fn series(n: usize, pair: &str) -> Vec<TradeRecord> {
        (0..n)
            .map(|i| rec_pair(if i % 4 == 0 { "-1.0" } else { "3.0" }, pair))
            .collect()
    }
    /// A 220-trade composite split evenly across two pairs (no single pair > 50% of R).
    fn healthy() -> Vec<TradeRecord> {
        let mut v = series(110, "A");
        v.extend(series(110, "B"));
        v
    }
    fn positive(n: usize) -> Vec<TradeRecord> {
        (0..n).map(|_| rec("3.0")).collect()
    }
    /// Inputs for a healthy composite, overridable per test.
    fn inputs<'a>(
        composite: &'a [TradeRecord],
        low: &'a [TradeRecord],
        high: &'a [TradeRecord],
        stress: &'a [TradeRecord],
        regime: Regime,
    ) -> VerdictInputs<'a> {
        VerdictInputs {
            composite,
            low_first: low,
            high_first: high,
            composite_at_1_5x: stress,
            regime,
        }
    }

    #[test]
    fn healthy_sample_passes() {
        let data = healthy();
        let stress = positive(220);
        let v = verdict(&inputs(&data, &data, &data, &stress, Regime::Both));
        assert_eq!(v.outcome, VerdictOutcome::Pass, "{v:?}");
        assert_eq!(v.n, 220);
        assert!(!v.concentrated);
        assert!(v.cost_stress_ok);
        assert!(v.both_policies_positive);
    }

    #[test]
    fn trade_count_gate_is_a_hard_200() {
        // 199 executed trades is below the pre-registered bar: Insufficient, not Fail.
        let data = series(199, "A");
        let v = verdict(&inputs(&data, &data, &data, &positive(199), Regime::Both));
        assert_eq!(v.outcome, VerdictOutcome::Insufficient);
        // exactly 200 (split across two pairs) passes.
        let data = healthy();
        let v = verdict(&inputs(&data, &data, &data, &positive(220), Regime::Both));
        assert_eq!(v.outcome, VerdictOutcome::Pass, "n=200 must pass: {v:?}");
    }

    #[test]
    fn below_min_trades_is_insufficient_never_pass_or_fail() {
        let data = series(199, "A");
        let v = verdict(&inputs(&data, &data, &data, &positive(199), Regime::Both));
        assert_eq!(v.outcome, VerdictOutcome::Insufficient);
        assert_ne!(v.outcome, VerdictOutcome::Pass);
        assert_ne!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn negative_cost_stress_fails_a_healthy_base_series() {
        // The base series is healthy at 1x cost, but its mean at 1.5x costs is negative -> Fail.
        let data = healthy();
        let stress: Vec<TradeRecord> = (0..220).map(|_| rec("-0.5")).collect();
        let v = verdict(&inputs(&data, &data, &data, &stress, Regime::Both));
        assert!(!v.cost_stress_ok);
        assert_eq!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn concentration_gate_is_strictly_over_half() {
        // Exactly 50% is allowed (two equal pairs).
        let data = healthy();
        let at_half = verdict(&inputs(&data, &data, &data, &positive(220), Regime::Both));
        assert!(!at_half.concentrated);
        assert_eq!(at_half.outcome, VerdictOutcome::Pass, "{at_half:?}");
        // Just over 50% trips the gate (160 vs 60 trades on the same per-trade series).
        let over: Vec<TradeRecord> = {
            let mut v = series(160, "A");
            v.extend(series(60, "B"));
            v
        };
        let concentrated = verdict(&inputs(&over, &over, &over, &positive(220), Regime::Both));
        assert!(concentrated.concentrated);
        assert_eq!(concentrated.outcome, VerdictOutcome::Fail);
        // One pair holds 100% of R -> concentrated.
        let single = positive(220);
        let one = verdict(&inputs(
            &single,
            &single,
            &single,
            &positive(220),
            Regime::Both,
        ));
        assert!(one.concentrated);
    }

    #[test]
    fn cost_stress_gate_requires_strictly_positive() {
        let data = healthy();
        let flat: Vec<TradeRecord> = (0..220).map(|_| rec("0.0")).collect();
        let v = verdict(&inputs(&data, &data, &data, &flat, Regime::Both));
        assert!(!v.cost_stress_ok);
        assert_eq!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn single_regime_and_unknown_regime_fail_closed() {
        let data = healthy();
        let v = verdict(&inputs(
            &data,
            &data,
            &data,
            &positive(220),
            Regime::OneRegimeOnly,
        ));
        assert_eq!(v.outcome, VerdictOutcome::Inconclusive);
        assert!(v.inconclusive_single_regime);
        let v = verdict(&inputs(
            &data,
            &data,
            &data,
            &positive(220),
            Regime::Unknown,
        ));
        assert_eq!(v.outcome, VerdictOutcome::Inconclusive);
        assert_ne!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn both_path_policies_gate_blocks_when_one_policy_is_negative() {
        let data = healthy();
        let negative: Vec<TradeRecord> = (0..220).map(|_| rec("-3.0")).collect();
        let v = verdict(&inputs(
            &data,
            &data,
            &negative,
            &positive(220),
            Regime::Both,
        ));
        assert!(!v.both_policies_positive);
        assert_eq!(v.outcome, VerdictOutcome::Fail, "{v:?}");
    }

    #[test]
    fn derive_stats_reads_the_ledger_end_to_end() {
        // The stats come from the trades, so the verdict reflects a concentration the caller never
        // supplied: 160/60 trades -> pair A supplies > 50% of total R.
        let composite: Vec<TradeRecord> = {
            let mut v = series(160, "A");
            v.extend(series(60, "B"));
            v
        };
        let stress = positive(220);
        let stats = derive_stats(&composite, &stress);
        assert_eq!(stats.total_r, Decimal::from(440)); // 220 trades * 2R
        assert_eq!(stats.max_pair_r, Decimal::from(320)); // 160 trades * 2R
        assert!((stats.mean_r_at_1_5x_cost - 3.0).abs() < 1e-9);
        let v = verdict(&inputs(
            &composite,
            &composite,
            &composite,
            &stress,
            Regime::Both,
        ));
        assert!(v.concentrated);
        assert_eq!(v.total_r, stats.total_r);
        assert_eq!(v.max_pair_r, stats.max_pair_r);
        assert_eq!(v.outcome, VerdictOutcome::Fail);
    }

    #[test]
    fn bootstrap_is_calendar_blocked_on_entry_time() {
        // Two calendar blocks with opposite means. A block bootstrap resamples whole blocks, so
        // a draw can land entirely on the negative block and the 5th percentile drops below the
        // overall mean; a per-trade bootstrap would not produce that dispersion.
        let mut data: Vec<TradeRecord> = (0..100).map(|_| at("3.0", 0)).collect();
        data.extend((0..100).map(|_| at("-1.0", 100))); // > 14 days later -> separate block
        let (mean, lower) =
            time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 10_000, BOOTSTRAP_SEED);
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
