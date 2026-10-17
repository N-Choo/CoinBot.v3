//! Perturbation diagnostics (plan Task 30). These are **diagnostics, never selectors**: nothing
//! here may feed parameters back into the frozen config, the freeze lock, or the verdict.
//!
//! The CLI runs them on **4H synthesis only** (never tick granularity), inside the pre-registered
//! `DIAGNOSTICS_BUDGET_SECS`. All randomness is seeded via `report::Lcg`, so a run reproduces
//! exactly; the budgets/thresholds below are pre-registered constants (spec §10.4).

use crate::harness::report::Lcg;
use rust_decimal::Decimal;
use std::time::{Duration, Instant};

/// Pre-registered wall-clock budget for the whole diagnostics pass (4H synthesis only).
/// Exceeding it is reported (`within_budget`), never silently ignored.
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

/// One-at-a-time ±20% perturbation of a single parameter. Returns the fraction of the two cells
/// (0.8×, 1.2×) whose expectancy is positive.
pub fn oat_positive_fraction(base: Decimal, eval: &dyn Fn(Decimal) -> Decimal) -> f64 {
    let (ok, total) = oat_cells(base, eval);
    ok as f64 / total as f64
}

fn oat_cells(base: Decimal, eval: &dyn Fn(Decimal) -> Decimal) -> (usize, usize) {
    let factors = oat_factors();
    let mut ok = 0;
    for f in factors {
        if eval(base * f) > Decimal::ZERO {
            ok += 1;
        }
    }
    (ok, factors.len())
}

/// OAT across the whole `axis`: each parameter in turn at ±20%, all others held at base.
/// Returns `(#positive cells, #cells)` so the caller can log how many variants were tried.
pub fn oat_axis_positive_cells(
    base: &[Decimal],
    eval: &dyn Fn(&[Decimal]) -> Decimal,
) -> (usize, usize) {
    let mut ok = 0;
    let mut total = 0;
    for i in 0..base.len() {
        for f in oat_factors() {
            let mut v = base.to_vec();
            v[i] = base[i] * f;
            if eval(&v) > Decimal::ZERO {
                ok += 1;
            }
            total += 1;
        }
    }
    (ok, total)
}

/// Seeded joint perturbation: each parameter is uniform in ±20%; returns the fraction of `draws`
/// with positive expectancy. Deterministic for a fixed `seed`.
pub fn joint_positive_fraction(
    base: &[Decimal],
    draws: usize,
    seed: u64,
    eval: &dyn Fn(&[Decimal]) -> Decimal,
) -> f64 {
    if draws == 0 {
        return 0.0;
    }
    joint_positive_count(base, draws, seed, eval) as f64 / draws as f64
}

fn joint_positive_count(
    base: &[Decimal],
    draws: usize,
    seed: u64,
    eval: &dyn Fn(&[Decimal]) -> Decimal,
) -> usize {
    let mut rng = Lcg::new(seed);
    let mut ok = 0;
    for _ in 0..draws {
        let v: Vec<Decimal> = base
            .iter()
            .map(|b| {
                // Uniform offset in [-0.2, 0.2] via per-mille: -200..=200.
                let off = rng.below(401) as i64 - 200;
                *b * (Decimal::ONE + Decimal::from(off) / Decimal::from(1000))
            })
            .collect();
        if eval(&v) > Decimal::ZERO {
            ok += 1;
        }
    }
    ok
}

/// The full diagnostics pass: OAT across the axis plus the seeded joint draws, timed against the
/// pre-registered wall-clock budget. `eval` runs 4H synthesis (the caller guarantees this).
#[derive(Debug, Clone, PartialEq)]
pub struct PerturbationReport {
    pub oat_positive_cells: usize,
    pub oat_cells: usize,
    pub oat_positive_fraction: f64,
    pub joint_positive_draws: usize,
    pub joint_draws: usize,
    pub joint_positive_fraction: f64,
    pub elapsed: Duration,
    pub budget: Duration,
}

impl PerturbationReport {
    /// How many variants the pass actually tried (OAT cells + joint draws).
    pub fn variants_tried(&self) -> usize {
        self.oat_cells + self.joint_draws
    }

    pub fn within_budget(&self) -> bool {
        self.elapsed <= self.budget
    }

    pub fn oat_gate_met(&self) -> bool {
        self.oat_positive_fraction >= OAT_POSITIVE_FRACTION_MIN
    }

    pub fn joint_gate_met(&self) -> bool {
        self.joint_positive_fraction >= JOINT_POSITIVE_FRACTION_MIN
    }
}

/// Run the full diagnostics pass over a parameter `axis`. Wall-clock timing is harness-only (the
/// core stays clock-free). Diagnostics never feed selection.
pub fn run_diagnostics(
    axis: &[Decimal],
    eval: &dyn Fn(&[Decimal]) -> Decimal,
) -> PerturbationReport {
    let start = Instant::now();
    let (oat_positive_cells, oat_cells) = oat_axis_positive_cells(axis, eval);
    let joint_positive_draws = joint_positive_count(axis, JOINT_DRAWS, JOINT_SEED, eval);
    let elapsed = start.elapsed();
    PerturbationReport {
        oat_positive_cells,
        oat_cells,
        oat_positive_fraction: if oat_cells == 0 {
            0.0
        } else {
            oat_positive_cells as f64 / oat_cells as f64
        },
        joint_positive_draws,
        joint_draws: JOINT_DRAWS,
        joint_positive_fraction: joint_positive_draws as f64 / JOINT_DRAWS as f64,
        elapsed,
        budget: Duration::from_secs(DIAGNOSTICS_BUDGET_SECS),
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
        assert_eq!(oat_positive_fraction(dec("2.0"), &|_| Decimal::ONE), 1.0);
    }

    #[test]
    fn oat_is_zero_when_eval_is_never_positive() {
        assert_eq!(oat_positive_fraction(dec("2.0"), &|_| Decimal::ZERO), 0.0);
    }

    #[test]
    fn joint_is_deterministic_for_a_fixed_seed() {
        let axis = [dec("1.0"), dec("2.0"), dec("0.5")];
        let eval = |v: &[Decimal]| v[0] + v[1] - v[2];
        let a = joint_positive_fraction(&axis, 200, JOINT_SEED, &eval);
        let b = joint_positive_fraction(&axis, 200, JOINT_SEED, &eval);
        assert_eq!(a, b, "same seed must reproduce the same fraction");
    }

    #[test]
    fn joint_is_one_when_eval_is_always_positive() {
        let axis = [dec("1.0"), dec("2.0")];
        assert_eq!(
            joint_positive_fraction(&axis, 200, JOINT_SEED, &|_| Decimal::ONE),
            1.0
        );
    }

    #[test]
    fn diagnostics_report_counts_budget_and_gates() {
        let axis = [dec("1.0"), dec("2.0")];
        let rep = run_diagnostics(&axis, &|_| Decimal::ONE);
        assert_eq!(rep.oat_cells, 4, "two parameters × two ±20% cells");
        assert_eq!(rep.variants_tried(), 4 + JOINT_DRAWS);
        assert_eq!(rep.oat_positive_fraction, 1.0);
        assert_eq!(rep.joint_positive_fraction, 1.0);
        assert!(rep.within_budget());
        assert!(rep.oat_gate_met() && rep.joint_gate_met());
    }
}
