use crate::harness::dataset::Dataset;
use crate::harness::experiment::{
    evaluate_slice, CandidateEvaluation, Decision, DecisionReport, SliceOutcome,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::partition::Slice;
use crate::harness::sweep::sampling::{generate, Candidate, SEED_DEFAULT};
use crate::strategy::config::Config;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalkForwardParams {
    pub window: usize,
    pub init_fund: i64,
    pub min_train_trades: usize,
    pub min_oos_trades: usize,
    pub max_dd_pct: Decimal,
    pub train_months: [usize; 5],
    pub oos_months: usize,
    pub total_months: usize,
}

impl Default for WalkForwardParams {
    fn default() -> Self {
        Self {
            window: crate::harness::experiment::DEFAULT_WINDOW,
            init_fund: 1000,
            min_train_trades: 10,
            min_oos_trades: 10,
            max_dd_pct: Decimal::from(20),
            train_months: [6, 7, 8, 9, 10],
            oos_months: 5,
            total_months: 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSplit {
    pub months: usize,
    pub train: Slice,
    pub oos: Slice,
}

#[derive(Debug, Clone)]
pub struct ConfigRun {
    pub candidate: Candidate,
    pub window_months: usize,
    pub train: SliceOutcome,
    pub train_stress: PartitionMetrics,
    pub oos: SliceOutcome,
    pub oos_stress: PartitionMetrics,
    pub successful: bool,
}

#[derive(Debug, Clone)]
pub struct WindowResult {
    pub split: WindowSplit,
    pub top: Vec<ConfigRun>,
    pub candidates: Vec<CandidateEvaluation>,
}

#[derive(Debug, Clone)]
pub struct SymbolWalkForward {
    pub symbol: String,
    pub dataset: Dataset,
    pub windows: Vec<WindowResult>,
    pub successful_runs: Vec<ConfigRun>,
    pub winner: Option<ConfigRun>,
    pub decision: DecisionReport,
    pub btc_regime: BTreeMap<i64, bool>,
}

pub fn split_windows(n: usize, params: &WalkForwardParams) -> Vec<WindowSplit> {
    let total = params.total_months.max(1);
    params
        .train_months
        .iter()
        .map(|&months| {
            let train_end = n * months / total;
            let oos_end = n * (months + params.oos_months) / total;
            WindowSplit {
                months,
                train: Slice {
                    start: 0,
                    end: train_end,
                },
                oos: Slice {
                    start: train_end,
                    end: oos_end,
                },
            }
        })
        .collect()
}

fn config_for(candidate: &Candidate, window: usize) -> Config {
    let mut cfg = candidate.to_config();
    cfg.window = window;
    cfg
}

fn profit_factor_passes(profit_factor: Option<Decimal>) -> bool {
    profit_factor
        .map(|value| value > Decimal::ONE)
        .unwrap_or(true)
}

fn train_eligible(metrics: &PartitionMetrics, params: &WalkForwardParams) -> bool {
    metrics.trades >= params.min_train_trades
        && metrics.net_expectancy > Decimal::ZERO
        && profit_factor_passes(metrics.net_profit_factor)
        && metrics.max_drawdown_pct <= params.max_dd_pct
}

pub(crate) fn select_top_two<'a>(
    candidates: &'a [CandidateEvaluation],
    params: &WalkForwardParams,
) -> Vec<&'a CandidateEvaluation> {
    let mut eligible: Vec<&CandidateEvaluation> = candidates
        .iter()
        .filter(|evaluation| train_eligible(&evaluation.dev.metrics, params))
        .collect();
    eligible.sort_by(|a, b| {
        b.dev
            .metrics
            .net_expectancy
            .cmp(&a.dev.metrics.net_expectancy)
            .then_with(|| a.candidate.label.cmp(&b.candidate.label))
    });
    eligible.truncate(2);
    eligible
}

fn oos_successful(metrics: &PartitionMetrics, params: &WalkForwardParams) -> bool {
    metrics.trades >= params.min_oos_trades
        && metrics.net_expectancy > Decimal::ZERO
        && profit_factor_passes(metrics.net_profit_factor)
}

fn symbol_decision(successful: &[ConfigRun], failures: &[String]) -> DecisionReport {
    if successful.is_empty() {
        let reasons = if failures.is_empty() {
            vec!["No out-of-sample run met the success gates.".to_string()]
        } else {
            failures.to_vec()
        };
        return DecisionReport {
            verdict: Decision::Fail,
            reasons,
        };
    }

    let reasons = successful
        .iter()
        .map(|run| {
            format!(
                "M1-M{}: {} passed OOS with {} trades, net expectancy {}, profit factor {}.",
                run.window_months,
                run.candidate.label,
                run.oos.metrics.trades,
                run.oos.metrics.net_expectancy,
                run.oos
                    .metrics
                    .net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "infinite".to_string())
            )
        })
        .collect();
    DecisionReport {
        verdict: Decision::Pass,
        reasons,
    }
}

pub fn overall_decision(verdicts: &[Decision], expected_symbols: usize) -> Decision {
    if verdicts.is_empty()
        || verdicts.len() != expected_symbols
        || verdicts.iter().any(|verdict| *verdict != Decision::Pass)
    {
        Decision::Fail
    } else {
        Decision::Pass
    }
}

/// Runs the expanding-window walk-forward experiment for one symbol.
///
/// Each window evaluates all 24 candidates on the training slice, selects the
/// two best eligible candidates, then evaluates those two on the following five
/// months. A symbol passes when at least one tested run succeeds, and the winner
/// is the successful run with the highest OOS net expectancy.
pub fn evaluate_symbol_walkforward(
    dataset: &Dataset,
    params: &WalkForwardParams,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<SymbolWalkForward, String> {
    let splits = split_windows(dataset.candles.len(), params);
    let grid = generate(SEED_DEFAULT);
    let mut windows = Vec::with_capacity(splits.len());
    let mut successful_runs = Vec::new();
    let mut failures = Vec::new();

    for split in splits {
        let mut candidates = Vec::with_capacity(grid.len());
        for candidate in &grid {
            let cfg = config_for(candidate, params.window);
            let train = evaluate_slice(
                dataset,
                &cfg,
                split.train,
                params.window,
                params.init_fund,
                base,
            )?;
            let train_stress = evaluate_slice(
                dataset,
                &cfg,
                split.train,
                params.window,
                params.init_fund,
                stress,
            )?
            .metrics;
            candidates.push(CandidateEvaluation {
                candidate: candidate.clone(),
                stress: train_stress,
                dev: train,
            });
        }

        let selected: Vec<&CandidateEvaluation> =
            select_top_two(&candidates, params).into_iter().collect();
        if selected.is_empty() {
            failures.push(format!(
                "M1-M{}: no candidate met the training gates.",
                split.months
            ));
        }

        let mut top = Vec::with_capacity(selected.len());
        for evaluation in selected {
            let candidate = evaluation.candidate.clone();
            let cfg = config_for(&candidate, params.window);
            let oos = evaluate_slice(
                dataset,
                &cfg,
                split.oos,
                params.window,
                params.init_fund,
                base,
            )?;
            let oos_stress = evaluate_slice(
                dataset,
                &cfg,
                split.oos,
                params.window,
                params.init_fund,
                stress,
            )?
            .metrics;
            let successful = oos_successful(&oos.metrics, params);
            let run = ConfigRun {
                candidate,
                window_months: split.months,
                train: evaluation.dev.clone(),
                train_stress: evaluation.stress.clone(),
                oos,
                oos_stress,
                successful,
            };
            if successful {
                successful_runs.push(run.clone());
            } else {
                failures.push(format!(
                    "M1-M{}: {} failed OOS ({} trades, net expectancy {}, PF {}).",
                    split.months,
                    run.candidate.label,
                    run.oos.metrics.trades,
                    run.oos.metrics.net_expectancy,
                    run.oos
                        .metrics
                        .net_profit_factor
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "infinite".to_string())
                ));
            }
            top.push(run);
        }

        windows.push(WindowResult {
            split,
            top,
            candidates,
        });
    }

    let winner = successful_runs
        .iter()
        .max_by(|a, b| {
            a.oos
                .metrics
                .net_expectancy
                .cmp(&b.oos.metrics.net_expectancy)
                .then_with(|| {
                    a.train
                        .metrics
                        .net_expectancy
                        .cmp(&b.train.metrics.net_expectancy)
                })
                .then_with(|| b.window_months.cmp(&a.window_months))
                .then_with(|| b.candidate.label.cmp(&a.candidate.label))
        })
        .cloned();
    let decision = symbol_decision(&successful_runs, &failures);

    Ok(SymbolWalkForward {
        symbol: dataset.symbol.clone(),
        dataset: dataset.clone(),
        windows,
        successful_runs,
        winner,
        decision,
        btc_regime: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::sim::SimOutcome;

    fn d(value: i64) -> Decimal {
        Decimal::from(value)
    }

    fn candidate(period: usize, long: i64, short: i64, label: &str) -> Candidate {
        Candidate {
            label: label.to_string(),
            origin: "test".to_string(),
            rsi_period: period,
            rsi_long_threshold: d(long),
            rsi_short_threshold: d(short),
        }
    }

    fn metrics(trades: usize, expectancy: i64, pf: Option<i64>, dd: i64) -> PartitionMetrics {
        PartitionMetrics {
            trades,
            net_expectancy: d(expectancy),
            net_profit_factor: pf.map(d),
            max_drawdown_pct: d(dd),
            ..Default::default()
        }
    }

    fn evaluation(candidate: Candidate, train_metrics: PartitionMetrics) -> CandidateEvaluation {
        CandidateEvaluation {
            candidate,
            stress: PartitionMetrics::default(),
            dev: SliceOutcome {
                metrics: train_metrics,
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
        }
    }

    fn run(label: &str, window: usize, oos_expectancy: i64, successful: bool) -> ConfigRun {
        ConfigRun {
            candidate: candidate(21, 60, 50, label),
            window_months: window,
            train: SliceOutcome {
                metrics: metrics(20, 5, Some(2), 5),
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            train_stress: metrics(20, 3, Some(2), 5),
            oos: SliceOutcome {
                metrics: metrics(12, oos_expectancy, Some(2), 5),
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            oos_stress: metrics(12, oos_expectancy - 1, Some(2), 5),
            successful,
        }
    }

    #[test]
    fn split_windows_uses_fifteen_month_fractions() {
        let params = WalkForwardParams::default();
        let splits = split_windows(15, &params);
        assert_eq!(splits.len(), 5);
        let expected = [
            (6, 0, 6, 6, 11),
            (7, 0, 7, 7, 12),
            (8, 0, 8, 8, 13),
            (9, 0, 9, 9, 14),
            (10, 0, 10, 10, 15),
        ];
        for (split, (months, train_start, train_end, oos_start, oos_end)) in
            splits.iter().zip(expected)
        {
            assert_eq!(
                *split,
                WindowSplit {
                    months,
                    train: Slice {
                        start: train_start,
                        end: train_end
                    },
                    oos: Slice {
                        start: oos_start,
                        end: oos_end
                    },
                }
            );
        }
    }

    #[test]
    fn split_windows_keeps_proportional_boundaries() {
        let params = WalkForwardParams::default();
        let splits = split_windows(2700, &params);
        assert_eq!(
            splits[0].train,
            Slice {
                start: 0,
                end: 1080
            }
        );
        assert_eq!(
            splits[0].oos,
            Slice {
                start: 1080,
                end: 1980
            }
        );
        assert_eq!(
            splits[4].train,
            Slice {
                start: 0,
                end: 1800
            }
        );
        assert_eq!(
            splits[4].oos,
            Slice {
                start: 1800,
                end: 2700
            }
        );
    }

    #[test]
    fn top_two_selection_is_ranked_and_gated() {
        let params = WalkForwardParams::default();
        let candidates = vec![
            evaluation(
                candidate(14, 60, 45, "ineligible-few-trades"),
                metrics(2, 999, Some(9), 1),
            ),
            evaluation(
                candidate(21, 65, 45, "best-eligible"),
                metrics(20, 12, Some(2), 5),
            ),
            evaluation(
                candidate(21, 60, 50, "second-eligible"),
                metrics(20, 8, Some(2), 5),
            ),
            evaluation(
                candidate(14, 70, 40, "too-much-dd"),
                metrics(30, 50, Some(4), 25),
            ),
        ];

        let top = select_top_two(&candidates, &params);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].candidate.label, "best-eligible");
        assert_eq!(top[1].candidate.label, "second-eligible");
    }

    #[test]
    fn success_gate_requires_trades_expectancy_and_profit_factor() {
        let params = WalkForwardParams::default();
        assert!(oos_successful(&metrics(10, 1, Some(2), 90), &params));
        assert!(!oos_successful(&metrics(9, 1, Some(2), 1), &params));
        assert!(!oos_successful(&metrics(10, 0, Some(2), 1), &params));
        assert!(!oos_successful(&metrics(10, 1, Some(1), 1), &params));
    }

    #[test]
    fn symbol_passes_when_any_window_or_config_succeeds() {
        let successful = vec![run("winner", 8, 7, true)];
        let report = symbol_decision(&successful, &[]);
        assert_eq!(report.verdict, Decision::Pass);

        let failed = symbol_decision(&[], &["M1-M6: no eligible candidate".to_string()]);
        assert_eq!(failed.verdict, Decision::Fail);
    }

    #[test]
    fn overall_passes_only_when_every_symbol_passes() {
        let all = [
            Decision::Pass,
            Decision::Pass,
            Decision::Pass,
            Decision::Pass,
            Decision::Pass,
        ];
        assert_eq!(overall_decision(&all, 5), Decision::Pass);
        assert_eq!(
            overall_decision(
                &[
                    Decision::Pass,
                    Decision::Pass,
                    Decision::Fail,
                    Decision::Pass,
                    Decision::Pass,
                ],
                5,
            ),
            Decision::Fail
        );
        assert_eq!(overall_decision(&all[..4], 5), Decision::Fail);
    }
}
