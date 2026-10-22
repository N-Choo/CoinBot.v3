use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::sampling::Candidate;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct CandidateOutcome {
    pub label: String,
    pub origin: String,
    pub candidate: Candidate,
    /// Pooled across symbols, base-cost scenario.
    pub pooled_validation: PartitionMetrics,
    pub per_symbol_validation: Vec<(String, PartitionMetrics)>,
    /// The 3 expanding-fold validation metrics (base scenario).
    pub folds: Vec<PartitionMetrics>,
    /// Full partition metrics for base and stress scenarios.
    pub base: BTreeMap<String, PartitionMetrics>,
    pub stress: BTreeMap<String, PartitionMetrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Thresholds {
    pub min_trades: usize,
    pub max_dd_pct: Decimal,
    pub min_positive_folds: usize,
    pub min_positive_symbols: usize,
    pub max_symbol_share_pct: Decimal,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_trades: 30,
            max_dd_pct: Decimal::from(20),
            min_positive_folds: 2,
            min_positive_symbols: 2,
            max_symbol_share_pct: Decimal::from(70),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Selection {
    pub winners: Vec<String>,
    pub notes: Vec<String>,
}

fn positive_folds(outcome: &CandidateOutcome) -> usize {
    outcome
        .folds
        .iter()
        .filter(|metrics| metrics.net_expectancy > Decimal::ZERO)
        .count()
}

fn positive_symbols(outcome: &CandidateOutcome) -> usize {
    outcome
        .per_symbol_validation
        .iter()
        .filter(|(_, metrics)| metrics.net_expectancy > Decimal::ZERO)
        .count()
}

/// Largest share of pooled validation trades contributed by a single symbol.
fn max_symbol_share_pct(outcome: &CandidateOutcome) -> Decimal {
    let pooled_trades = outcome.pooled_validation.trades;
    if pooled_trades == 0 {
        return Decimal::ZERO;
    }
    outcome
        .per_symbol_validation
        .iter()
        .map(|(_, metrics)| {
            Decimal::from(metrics.trades) / Decimal::from(pooled_trades) * Decimal::from(100)
        })
        .max()
        .unwrap_or(Decimal::ZERO)
}

/// Returns the first failed criterion as a note naming the candidate.
fn rejection_reason(outcome: &CandidateOutcome, thresholds: &Thresholds) -> Option<String> {
    if outcome.pooled_validation.net_expectancy <= Decimal::ZERO {
        return Some(format!(
            "{}: rejected: validation net expectancy {} is not positive",
            outcome.label, outcome.pooled_validation.net_expectancy
        ));
    }

    if let Some((symbol, metrics)) = outcome
        .per_symbol_validation
        .iter()
        .find(|(_, metrics)| metrics.max_drawdown_pct > thresholds.max_dd_pct)
    {
        return Some(format!(
            "{}: rejected: {} max drawdown {}% exceeds {}%",
            outcome.label, symbol, metrics.max_drawdown_pct, thresholds.max_dd_pct
        ));
    }

    let folds = positive_folds(outcome);
    if folds < thresholds.min_positive_folds {
        return Some(format!(
            "{}: rejected: only {folds} of {} folds have positive net expectancy (need {})",
            outcome.label,
            outcome.folds.len(),
            thresholds.min_positive_folds
        ));
    }

    if outcome.pooled_validation.trades < thresholds.min_trades {
        return Some(format!(
            "{}: rejected: {} validation trades is below the minimum of {}",
            outcome.label, outcome.pooled_validation.trades, thresholds.min_trades
        ));
    }

    let share = max_symbol_share_pct(outcome);
    if share > thresholds.max_symbol_share_pct {
        return Some(format!(
            "{}: rejected: a single symbol contributes {}% of validation trades (max {}%)",
            outcome.label, share, thresholds.max_symbol_share_pct
        ));
    }

    let symbols = positive_symbols(outcome);
    if symbols < thresholds.min_positive_symbols {
        return Some(format!(
            "{}: rejected: only {symbols} of {} symbols have positive net expectancy (need {})",
            outcome.label,
            outcome.per_symbol_validation.len(),
            thresholds.min_positive_symbols
        ));
    }

    None
}

/// Selects candidates using validation results only.
///
/// Criteria are applied in order: positive pooled validation expectancy, per-symbol drawdown,
/// positive folds, then trade count/symbol concentration/symbol consistency. Stress results do not
/// affect selection; a qualifier with negative stress validation net PnL is flagged in `notes`.
pub fn rank(outcomes: &[CandidateOutcome], thresholds: &Thresholds) -> Selection {
    let mut qualifiers: Vec<&CandidateOutcome> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    for outcome in outcomes {
        match rejection_reason(outcome, thresholds) {
            Some(note) => notes.push(note),
            None => qualifiers.push(outcome),
        }
    }

    qualifiers.sort_by(|a, b| {
        b.pooled_validation
            .net_expectancy
            .cmp(&a.pooled_validation.net_expectancy)
            .then_with(|| {
                a.pooled_validation
                    .max_drawdown_pct
                    .cmp(&b.pooled_validation.max_drawdown_pct)
            })
            .then_with(|| positive_folds(b).cmp(&positive_folds(a)))
            .then_with(|| b.pooled_validation.trades.cmp(&a.pooled_validation.trades))
            // Stable, deterministic tie-break for otherwise identical candidates.
            .then_with(|| a.label.cmp(&b.label))
    });

    for outcome in &qualifiers {
        if let Some(stress) = outcome.stress.get("validation") {
            if stress.net_pnl < Decimal::ZERO {
                notes.push(format!(
                    "{}: flagged: negative stress validation net PnL ({})",
                    outcome.label, stress.net_pnl
                ));
            }
        }
    }

    let winners: Vec<String> = qualifiers
        .iter()
        .map(|outcome| outcome.label.clone())
        .collect();
    if winners.is_empty() {
        notes.push("no candidate meets the criteria".to_string());
    }

    Selection { winners, notes }
}
