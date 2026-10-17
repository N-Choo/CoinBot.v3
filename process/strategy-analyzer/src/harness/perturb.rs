//! Perturbation diagnostics (plan Task 30). These are **diagnostics, never selectors**: nothing
//! here may feed parameters back into the frozen config, the freeze lock, or the verdict.
//!
//! The CLI runs them on **4H synthesis only** (never tick granularity), inside the pre-registered
//! `DIAGNOSTICS_BUDGET_SECS`. All randomness is seeded via `report::Lcg`, so a run reproduces
//! exactly; the budgets/thresholds below are pre-registered constants (spec §10.4).
//!
//! `eval` returns `None` for a cell whose config is invalid (e.g. a −20% `target_r` falling below
//! the engine's net floor). Invalid cells are **skipped and counted separately**, never panicked on.

use crate::harness::report::Lcg;
use rust_decimal::Decimal;
use std::time::{Duration, Instant};

/// Pre-registered wall-clock budget for the whole diagnostics pass (4H synthesis only).
/// The pass **stops** when the budget is exceeded (it is enforced, not merely measured) and reports
/// `within_budget == false`.
pub const DIAGNOSTICS_BUDGET_SECS: u64 = 3_600;

/// Pre-registered diagnostic thresholds (spec §10.4): OAT ≥ 70% of ±20% cells positive;
/// joint ≥ 60% of 200 seeded draws positive. Reported only — never used to select.
pub const OAT_POSITIVE_FRACTION_MIN: f64 = 0.70;
pub const JOINT_POSITIVE_FRACTION_MIN: f64 = 0.60;

/// Pre-registered joint-perturbation settings (fixed before the OOS run).
pub const JOINT_DRAWS: usize = 200;
pub const JOINT_SEED: u64 = 13_337;

/// The two one-at-a-time multipliers (±20%): 0.8 and 1.2.
fn oat_factors() -> [Decimal; 2] {
    [Decimal::new(8, 1), Decimal::new(12, 1)]
}

/// True once `deadline` has been reached. `None` means no budget (used by the standalone helpers).
fn deadline_passed(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|d| Instant::now() >= d)
}

/// One-at-a-time ±20% perturbation of a single parameter. Returns the fraction of the **valid**
/// cells (0.8×, 1.2×) whose expectancy is positive; invalid cells are skipped (not counted).
pub fn oat_positive_fraction(base: Decimal, eval: &dyn Fn(Decimal) -> Option<Decimal>) -> f64 {
    let mut ok = 0usize;
    let mut valid = 0usize;
    for f in oat_factors() {
        if let Some(x) = eval(base * f) {
            valid += 1;
            if x > Decimal::ZERO {
                ok += 1;
            }
        }
    }
    if valid == 0 {
        0.0
    } else {
        ok as f64 / valid as f64
    }
}

/// OAT across the whole `axis`: each parameter in turn at ±20%, all others held at base.
/// `eval` returns `None` for an invalid cell (skipped). Returns
/// `(positive cells, attempted cells, invalid cells)` so the caller can log variants tried and how
/// many cells were skipped. Stops early (setting `budget_exceeded`) once `deadline` is reached.
fn oat_axis_positive_cells(
    base: &[Decimal],
    eval: &dyn Fn(&[Decimal]) -> Option<Decimal>,
    deadline: Option<Instant>,
    budget_exceeded: &mut bool,
) -> (usize, usize, usize) {
    let mut positive = 0usize;
    let mut attempted = 0usize;
    let mut invalid = 0usize;
    'outer: for i in 0..base.len() {
        for f in oat_factors() {
            if deadline_passed(deadline) {
                *budget_exceeded = true;
                break 'outer;
            }
            let mut v = base.to_vec();
            v[i] = base[i] * f;
            attempted += 1;
            match eval(&v) {
                None => invalid += 1,
                Some(x) => {
                    if x > Decimal::ZERO {
                        positive += 1;
                    }
                }
            }
        }
    }
    (positive, attempted, invalid)
}

/// Seeded joint perturbation: each parameter is uniform in ±20%; returns the fraction of the
/// **valid** `draws` with positive expectancy. Invalid draws are skipped. Deterministic for a fixed
/// `seed`.
pub fn joint_positive_fraction(
    base: &[Decimal],
    draws: usize,
    seed: u64,
    eval: &dyn Fn(&[Decimal]) -> Option<Decimal>,
) -> f64 {
    if draws == 0 {
        return 0.0;
    }
    let mut exceeded = false;
    let (positive, attempted, _invalid) =
        joint_positive_count(base, draws, seed, eval, None, &mut exceeded);
    if attempted == 0 {
        0.0
    } else {
        positive as f64 / attempted as f64
    }
}

/// Seeded joint draws. Returns `(positive, attempted, invalid)`, stopping early when `deadline` is
/// reached. The number of attempted draws is recorded so a truncated pass is visible.
fn joint_positive_count(
    base: &[Decimal],
    draws: usize,
    seed: u64,
    eval: &dyn Fn(&[Decimal]) -> Option<Decimal>,
    deadline: Option<Instant>,
    budget_exceeded: &mut bool,
) -> (usize, usize, usize) {
    let mut rng = Lcg::new(seed);
    let mut positive = 0usize;
    let mut attempted = 0usize;
    let mut invalid = 0usize;
    for _ in 0..draws {
        if deadline_passed(deadline) {
            *budget_exceeded = true;
            break;
        }
        let v: Vec<Decimal> = base
            .iter()
            .map(|b| {
                // Uniform offset in [-0.2, 0.2] via per-mille: -200..=200.
                let off = rng.below(401) as i64 - 200;
                *b * (Decimal::ONE + Decimal::from(off) / Decimal::from(1000))
            })
            .collect();
        attempted += 1;
        match eval(&v) {
            None => invalid += 1,
            Some(x) => {
                if x > Decimal::ZERO {
                    positive += 1;
                }
            }
        }
    }
    (positive, attempted, invalid)
}

/// The full diagnostics pass: OAT across the axis plus the seeded joint draws, timed against the
/// pre-registered wall-clock budget. `eval` runs 4H synthesis (the caller guarantees this).
///
/// Invalid cells are reported (`oat_invalid_cells`/`joint_invalid_draws`) and excluded from the
/// gate fractions. When the budget is reached the pass stops and `within_budget` is `false`.
#[derive(Debug, Clone, PartialEq)]
pub struct PerturbationReport {
    pub oat_positive_cells: usize,
    /// Cells attempted (2 × axis length), including invalid ones.
    pub oat_cells: usize,
    pub oat_invalid_cells: usize,
    pub oat_positive_fraction: f64,
    pub joint_positive_draws: usize,
    /// Draws attempted (≤ `JOINT_DRAWS`), including invalid ones.
    pub joint_draws: usize,
    pub joint_invalid_draws: usize,
    pub joint_positive_fraction: f64,
    pub elapsed: Duration,
    pub budget: Duration,
    /// `false` when the pass stopped because it exceeded the wall-clock budget.
    pub within_budget: bool,
}

impl PerturbationReport {
    /// How many variants the pass actually tried (OAT cells attempted + joint draws attempted).
    pub fn variants_tried(&self) -> usize {
        self.oat_cells + self.joint_draws
    }

    pub fn oat_gate_met(&self) -> bool {
        self.oat_positive_fraction >= OAT_POSITIVE_FRACTION_MIN
    }

    pub fn joint_gate_met(&self) -> bool {
        self.joint_positive_fraction >= JOINT_POSITIVE_FRACTION_MIN
    }
}

/// Run the full diagnostics pass over a parameter `axis` against the pre-registered budget.
pub fn run_diagnostics(
    axis: &[Decimal],
    eval: &dyn Fn(&[Decimal]) -> Option<Decimal>,
) -> PerturbationReport {
    run_diagnostics_with_budget(axis, eval, Duration::from_secs(DIAGNOSTICS_BUDGET_SECS))
}

/// Budget-parameterised entry point (the budget is injected so the enforcement is unit-testable
/// without waiting an hour).
pub fn run_diagnostics_with_budget(
    axis: &[Decimal],
    eval: &dyn Fn(&[Decimal]) -> Option<Decimal>,
    budget: Duration,
) -> PerturbationReport {
    let start = Instant::now();
    let deadline = start + budget;
    let mut budget_exceeded = false;

    let (oat_positive_cells, oat_cells, oat_invalid_cells) =
        oat_axis_positive_cells(axis, eval, Some(deadline), &mut budget_exceeded);
    let (joint_positive_draws, joint_draws, joint_invalid_draws) = if budget_exceeded {
        (0, 0, 0)
    } else {
        joint_positive_count(
            axis,
            JOINT_DRAWS,
            JOINT_SEED,
            eval,
            Some(deadline),
            &mut budget_exceeded,
        )
    };

    let oat_valid = oat_cells - oat_invalid_cells;
    let joint_valid = joint_draws - joint_invalid_draws;
    PerturbationReport {
        oat_positive_cells,
        oat_cells,
        oat_invalid_cells,
        oat_positive_fraction: if oat_valid == 0 {
            0.0
        } else {
            oat_positive_cells as f64 / oat_valid as f64
        },
        joint_positive_draws,
        joint_draws,
        joint_invalid_draws,
        joint_positive_fraction: if joint_valid == 0 {
            0.0
        } else {
            joint_positive_draws as f64 / joint_valid as f64
        },
        elapsed: start.elapsed(),
        budget,
        within_budget: !budget_exceeded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn oat_is_one_when_eval_is_always_positive() {
        assert_eq!(
            oat_positive_fraction(dec("2.0"), &|_| Some(Decimal::ONE)),
            1.0
        );
    }

    #[test]
    fn oat_is_zero_when_eval_is_never_positive() {
        assert_eq!(
            oat_positive_fraction(dec("2.0"), &|_| Some(Decimal::ZERO)),
            0.0
        );
    }

    #[test]
    fn joint_is_deterministic_for_a_fixed_seed() {
        let axis = [dec("1.0"), dec("2.0"), dec("0.5")];
        let eval = |v: &[Decimal]| Some(v[0] + v[1] - v[2]);
        let a = joint_positive_fraction(&axis, 200, JOINT_SEED, &eval);
        let b = joint_positive_fraction(&axis, 200, JOINT_SEED, &eval);
        assert_eq!(a, b, "same seed must reproduce the same fraction");
    }

    #[test]
    fn joint_is_one_when_eval_is_always_positive() {
        let axis = [dec("1.0"), dec("2.0")];
        assert_eq!(
            joint_positive_fraction(&axis, 200, JOINT_SEED, &|_| Some(Decimal::ONE)),
            1.0
        );
    }

    #[test]
    fn diagnostics_report_counts_budget_and_gates() {
        let axis = [dec("1.0"), dec("2.0")];
        let rep = run_diagnostics(&axis, &|_| Some(Decimal::ONE));
        assert_eq!(rep.oat_cells, 4, "two parameters × two ±20% cells");
        assert_eq!(rep.oat_invalid_cells, 0);
        assert_eq!(rep.variants_tried(), 4 + JOINT_DRAWS);
        assert_eq!(rep.oat_positive_fraction, 1.0);
        assert_eq!(rep.joint_positive_fraction, 1.0);
        assert!(rep.within_budget);
        assert!(rep.oat_gate_met() && rep.joint_gate_met());
    }

    #[test]
    fn invalid_cells_are_skipped_and_counted_separately() {
        let axis = [dec("1.0"), dec("2.0")];
        // Every cell is invalid: nothing is evaluated, nothing is counted as positive.
        let rep = run_diagnostics(&axis, &|_| None);
        assert_eq!(rep.oat_cells, 4);
        assert_eq!(rep.oat_invalid_cells, 4);
        assert_eq!(rep.oat_positive_fraction, 0.0);
        assert_eq!(rep.joint_draws, JOINT_DRAWS);
        assert_eq!(rep.joint_invalid_draws, JOINT_DRAWS);
        assert_eq!(rep.joint_positive_fraction, 0.0);
    }

    #[test]
    fn invalid_single_cells_are_excluded_from_the_fraction() {
        let axis = [dec("1.0")];
        // Only the 0.8× cell is valid (invalid as soon as v[0] >= 1.0, mirroring a failed validate).
        let eval = |v: &[Decimal]| (v[0] < Decimal::ONE).then_some(Decimal::ONE);
        let rep = run_diagnostics_with_budget(&axis, &eval, Duration::from_secs(3_600));
        assert_eq!(rep.oat_cells, 2);
        assert_eq!(rep.oat_invalid_cells, 1);
        assert_eq!(
            rep.oat_positive_fraction, 1.0,
            "1 positive over 1 valid cell"
        );
    }

    #[test]
    fn budget_is_enforced_not_just_measured() {
        let axis = [dec("1.0")];
        let rep = run_diagnostics_with_budget(&axis, &|_| Some(Decimal::ONE), Duration::ZERO);
        assert!(!rep.within_budget, "an exceeded budget must be reported");
        assert_eq!(rep.oat_cells, 0, "the pass stops before trying any cell");
        assert_eq!(
            rep.joint_draws, 0,
            "the joint pass is skipped once exceeded"
        );
        assert_eq!(rep.variants_tried(), 0);
    }
}
