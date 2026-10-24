use crate::harness::dataset::Dataset;
use crate::harness::replay::build_replay;
use crate::harness::sim::{simulate, unrealized_pnl, SimOutcome, SimSignal};
use crate::harness::sweep::costs::{net_open_pnl, net_trade_pnl, CostScenario};
use crate::harness::sweep::evaluate::{metrics_from_outcome, PartitionMetrics};
use crate::harness::sweep::partition::{slice_with_warmup, Slice};
use crate::harness::sweep::sampling::{generate, Candidate, SEED_DEFAULT};
use crate::strategy::config::Config;
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

pub const STRATEGY_VERSION: &str = "rsi-mom-v1";
pub const DEFAULT_WINDOW: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperimentParams {
    pub window: usize,
    pub init_fund: i64,
    /// Number of development candidates frozen for the M5/M6-M10 report.
    pub top_n: usize,
    pub max_dd_pct: Decimal,
}

impl Default for ExperimentParams {
    fn default() -> Self {
        Self {
            window: DEFAULT_WINDOW,
            init_fund: 1000,
            top_n: 3,
            max_dd_pct: Decimal::from(20),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Split {
    pub dev: Slice,
    pub first_oos: Slice,
    pub extended_oos: Slice,
}

#[derive(Debug, Clone)]
pub struct SliceOutcome {
    pub metrics: PartitionMetrics,
    pub outcome: SimOutcome,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Clone)]
pub struct CandidateEvaluation {
    pub candidate: Candidate,
    pub stress: PartitionMetrics,
    pub dev: SliceOutcome,
}

/// One frozen development candidate with its M5 and M6-M10 evaluations.
#[derive(Debug, Clone)]
pub struct TopCandidate {
    pub candidate: Candidate,
    pub first_oos: SliceOutcome,
    pub first_oos_stress: PartitionMetrics,
    pub extended_oos: SliceOutcome,
    pub extended_oos_stress: PartitionMetrics,
    /// M5 is the only decision partition: PF > 1 and net expectancy > 0.
    pub m5_pass: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SideMetrics {
    pub trades: usize,
    pub net_pnl: Decimal,
    pub open_mark: Decimal,
    pub net_expectancy: Decimal,
    pub profit_factor: Option<Decimal>,
    pub win_rate: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RegimeMetrics {
    pub bull: SideMetrics,
    pub bear: SideMetrics,
    pub unknown: SideMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Decision {
    Pass,
    Fail,
    Inconclusive,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Pass => "Pass",
            Decision::Fail => "Fail",
            Decision::Inconclusive => "Inconclusive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecisionReport {
    pub verdict: Decision,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SymbolExperiment {
    pub symbol: String,
    pub dataset: Dataset,
    pub split: Split,
    pub candidates: Vec<CandidateEvaluation>,
    /// Frozen top-N candidates in combined-rank order.
    pub top: Vec<TopCandidate>,
    pub selected: Option<Candidate>,
    pub first_oos: Option<SliceOutcome>,
    pub first_oos_stress: Option<PartitionMetrics>,
    pub extended_oos: Option<SliceOutcome>,
    pub extended_oos_stress: Option<PartitionMetrics>,
    pub decision: DecisionReport,
    /// Open time -> `close > 200-period SMA` (true bull) for BTC-USDT 4hour.
    /// Populated by the runner; empty when the BTC regime is unavailable.
    pub btc_regime: BTreeMap<i64, bool>,
}

/// Chronological one-year split: M1-M4 development, M5 first OOS, M6-M10
/// extended OOS, and M11-M12 excluded.
pub fn split_one_year(n: usize) -> Split {
    Split {
        dev: Slice {
            start: 0,
            end: n * 4 / 12,
        },
        first_oos: Slice {
            start: n * 4 / 12,
            end: n * 5 / 12,
        },
        extended_oos: Slice {
            start: n * 5 / 12,
            end: n * 10 / 12,
        },
    }
}

fn config_for(candidate: &Candidate, window: usize) -> Config {
    let mut cfg = candidate.to_config();
    cfg.window = window;
    cfg
}

/// Replays one slice with warm-up and returns the simulated outcome plus
/// base/stress metrics. Only signals inside the slice are passed to the
/// simulator; positions open at the boundary are marked and dropped.
pub fn evaluate_slice(
    dataset: &Dataset,
    cfg: &Config,
    slice: Slice,
    window: usize,
    init_fund: i64,
    scenario: &CostScenario,
) -> Result<SliceOutcome, String> {
    if window != cfg.window {
        return Err(format!("window {window} != cfg.window {}", cfg.window));
    }
    let partition = slice_with_warmup(&dataset.candles, slice, window);
    let n_candles = partition.candles.len();
    if n_candles == 0 {
        return Err(format!(
            "{}: empty slice {}..{}",
            dataset.symbol, slice.start, slice.end
        ));
    }
    let eval_start = partition.partition_start;
    let last_index = n_candles - 1;
    let replayed_candles = n_candles.saturating_sub(window);
    let slice_dataset = Dataset {
        symbol: dataset.symbol.clone(),
        candles: partition.candles,
        provenance: dataset.provenance.clone(),
    };
    let replay = build_replay(&slice_dataset, cfg)
        .map_err(|error| format!("{}: {error}", dataset.symbol))?;
    let sim_signals: Vec<SimSignal> = replay
        .signals
        .into_iter()
        .filter(|signal| signal.candle_index >= eval_start && signal.candle_index + 1 < n_candles)
        .map(|signal| SimSignal {
            candle_index: signal.candle_index,
            signal: signal.signal,
            signal_id: signal.signal_id,
        })
        .collect();
    let outcome = simulate(&slice_dataset.candles, &sim_signals, init_fund, window);
    let last_close = slice_dataset
        .candles
        .last()
        .expect("slice always contains candles")
        .close;
    let metrics = metrics_from_outcome(
        &outcome,
        last_close,
        last_index,
        replayed_candles,
        Decimal::from(init_fund),
        scenario,
    );
    Ok(SliceOutcome {
        metrics,
        outcome,
        candles: slice_dataset.candles,
    })
}

fn profit_factor(gross_profit: Decimal, gross_loss: Decimal) -> Option<Decimal> {
    if gross_loss.is_zero() {
        None
    } else {
        Some(gross_profit / gross_loss.abs())
    }
}

fn profit_factor_passes(profit_factor: Option<Decimal>) -> bool {
    // No losing trades means the profit factor is infinite rather than below
    // one; the expectancy gate still rejects a flat all-zero record.
    profit_factor
        .map(|value| value > Decimal::ONE)
        .unwrap_or(true)
}

#[derive(Default)]
struct SideBucket {
    trades: usize,
    wins: usize,
    gross_profit: Decimal,
    gross_loss: Decimal,
    net: Decimal,
    open_mark: Decimal,
}

impl SideBucket {
    fn add_closed(&mut self, net: Decimal) {
        self.trades += 1;
        self.net += net;
        if net > Decimal::ZERO {
            self.wins += 1;
            self.gross_profit += net;
        } else if net < Decimal::ZERO {
            self.gross_loss += net;
        }
    }

    fn finish(self) -> SideMetrics {
        let trades = Decimal::from(self.trades);
        SideMetrics {
            trades: self.trades,
            net_pnl: self.net,
            open_mark: self.open_mark,
            net_expectancy: if self.trades == 0 {
                Decimal::ZERO
            } else {
                self.net / trades
            },
            profit_factor: profit_factor(self.gross_profit, self.gross_loss),
            win_rate: if self.trades == 0 {
                Decimal::ZERO
            } else {
                Decimal::from(self.wins) / trades * Decimal::from(100)
            },
        }
    }
}

fn closed_net(trade: &crate::harness::sim::ClosedTrade, scenario: &CostScenario) -> Decimal {
    if trade.fill.is_zero() {
        trade.pnl
    } else {
        net_trade_pnl(
            scenario, trade.side, trade.e, trade.fill, trade.exit, trade.pnl,
        )
    }
}

/// Metrics restricted to one side. `net_pnl` is realized closed PnL and
/// `open_mark` is the net mark-to-market of positions still open at the end.
pub fn side_metrics(
    outcome: &SimOutcome,
    scenario: &CostScenario,
    side: Direction,
    last_close: Decimal,
) -> SideMetrics {
    let mut bucket = SideBucket::default();
    for trade in &outcome.trades {
        if trade.side == side {
            bucket.add_closed(closed_net(trade, scenario));
        }
    }
    for position in &outcome.open_positions {
        if position.side != side || position.fill.is_zero() {
            continue;
        }
        let gross = unrealized_pnl(position.side, position.fill, last_close, position.e);
        bucket.open_mark += net_open_pnl(scenario, position.e, position.fill, last_close, gross);
    }
    bucket.finish()
}

fn regime_at(index: usize, candles: &[Candle], btc_regime: &BTreeMap<i64, bool>) -> Option<bool> {
    candles
        .get(index)
        .and_then(|candle| btc_regime.get(&candle.open_time_ms).copied())
}

/// Splits trades and open positions by the BTC regime at their entry candle.
/// Entries whose BTC regime is unknown (before the 200-period SMA warm-up or
/// outside the BTC series) are reported separately.
pub fn regime_metrics(
    outcome: &SimOutcome,
    scenario: &CostScenario,
    slice_candles: &[Candle],
    btc_regime: &BTreeMap<i64, bool>,
    last_close: Decimal,
) -> RegimeMetrics {
    let mut bull = SideBucket::default();
    let mut bear = SideBucket::default();
    let mut unknown = SideBucket::default();

    for trade in &outcome.trades {
        let bucket = match regime_at(trade.entry_index, slice_candles, btc_regime) {
            Some(true) => &mut bull,
            Some(false) => &mut bear,
            None => &mut unknown,
        };
        bucket.add_closed(closed_net(trade, scenario));
    }
    for position in &outcome.open_positions {
        if position.fill.is_zero() {
            continue;
        }
        let gross = unrealized_pnl(position.side, position.fill, last_close, position.e);
        let net = net_open_pnl(scenario, position.e, position.fill, last_close, gross);
        let bucket = match regime_at(position.entry_index, slice_candles, btc_regime) {
            Some(true) => &mut bull,
            Some(false) => &mut bear,
            None => &mut unknown,
        };
        bucket.open_mark += net;
    }

    RegimeMetrics {
        bull: bull.finish(),
        bear: bear.finish(),
        unknown: unknown.finish(),
    }
}

/// Orders profit factors for a descending sort. `None` means there were no
/// losing trades, so the profit factor is infinite and ranks first.
fn profit_factor_order(left: Option<Decimal>, right: Option<Decimal>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => right.cmp(&left),
    }
}

/// Selects the top `params.top_n` development candidates by combined rank.
///
/// Candidates are ranked independently by profit factor and by net expectancy
/// (ties broken by label); the combined score is the sum of the two ranks, so
/// lower is better. Only the max-drawdown gate is applied here: trade counts
/// and the signs of expectancy and PF do not filter candidates.
fn select_top(candidates: &[CandidateEvaluation], params: &ExperimentParams) -> Vec<Candidate> {
    if params.top_n == 0 {
        return Vec::new();
    }
    let eligible: Vec<&CandidateEvaluation> = candidates
        .iter()
        .filter(|evaluation| evaluation.dev.metrics.max_drawdown_pct <= params.max_dd_pct)
        .collect();
    if eligible.is_empty() {
        return Vec::new();
    }

    let mut by_pf = eligible.clone();
    by_pf.sort_by(|a, b| {
        profit_factor_order(
            a.dev.metrics.net_profit_factor,
            b.dev.metrics.net_profit_factor,
        )
        .then_with(|| {
            b.dev
                .metrics
                .net_expectancy
                .cmp(&a.dev.metrics.net_expectancy)
        })
        .then_with(|| a.candidate.label.cmp(&b.candidate.label))
    });
    let pf_rank: BTreeMap<String, usize> = by_pf
        .iter()
        .enumerate()
        .map(|(rank, evaluation)| (evaluation.candidate.label.clone(), rank))
        .collect();

    let mut by_expectancy = eligible.clone();
    by_expectancy.sort_by(|a, b| {
        b.dev
            .metrics
            .net_expectancy
            .cmp(&a.dev.metrics.net_expectancy)
            .then_with(|| {
                profit_factor_order(
                    a.dev.metrics.net_profit_factor,
                    b.dev.metrics.net_profit_factor,
                )
            })
            .then_with(|| a.candidate.label.cmp(&b.candidate.label))
    });
    let expectancy_rank: BTreeMap<String, usize> = by_expectancy
        .iter()
        .enumerate()
        .map(|(rank, evaluation)| (evaluation.candidate.label.clone(), rank))
        .collect();

    let mut ranked: Vec<(&CandidateEvaluation, usize, usize)> = eligible
        .iter()
        .map(|evaluation| {
            let label = &evaluation.candidate.label;
            (*evaluation, pf_rank[label], expectancy_rank[label])
        })
        .collect();
    ranked.sort_by(|a, b| {
        (a.1 + a.2)
            .cmp(&(b.1 + b.2))
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.0.candidate.label.cmp(&b.0.candidate.label))
    });
    ranked.truncate(params.top_n);
    ranked
        .into_iter()
        .map(|(evaluation, _, _)| evaluation.candidate.clone())
        .collect()
}

fn decision_report(top: &[TopCandidate]) -> DecisionReport {
    if top.is_empty() {
        return DecisionReport {
            verdict: Decision::Fail,
            reasons: vec![
                "No development candidate passed the max drawdown gate; the holdout cannot be evaluated."
                    .to_string(),
            ],
        };
    }

    let mut reasons = Vec::with_capacity(top.len() + 1);
    for (index, candidate) in top.iter().enumerate() {
        let metrics = &candidate.first_oos.metrics;
        reasons.push(format!(
            "Top-{} {}: M5 {} trades, net expectancy {}, profit factor {} — {}.",
            index + 1,
            candidate.candidate.label,
            metrics.trades,
            metrics.net_expectancy,
            metrics
                .net_profit_factor
                .map(|value| value.to_string())
                .unwrap_or_else(|| "infinite".to_string()),
            if candidate.m5_pass { "pass" } else { "fail" }
        ));
    }

    if top.iter().any(|candidate| candidate.m5_pass) {
        reasons.push(
            "Pass: at least one top config has M5 net expectancy > 0 and profit factor > 1."
                .to_string(),
        );
        DecisionReport {
            verdict: Decision::Pass,
            reasons,
        }
    } else {
        reasons.push(
            "Fail: no top config has M5 net expectancy > 0 and profit factor > 1.".to_string(),
        );
        DecisionReport {
            verdict: Decision::Fail,
            reasons,
        }
    }
}

/// Evaluates every grid candidate on the development window, freezes the top N
/// by combined PF/expectancy rank, then evaluates each of them on M5 and
/// M6-M10. The M6-M10 result is logged but never changes the decision.
pub fn evaluate_symbol(
    dataset: &Dataset,
    params: &ExperimentParams,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<SymbolExperiment, String> {
    let split = split_one_year(dataset.candles.len());
    let mut candidates = Vec::with_capacity(24);
    for candidate in generate(SEED_DEFAULT) {
        let cfg = config_for(&candidate, params.window);
        let dev = evaluate_slice(
            dataset,
            &cfg,
            split.dev,
            params.window,
            params.init_fund,
            base,
        )?;
        let stress_metrics = evaluate_slice(
            dataset,
            &cfg,
            split.dev,
            params.window,
            params.init_fund,
            stress,
        )?
        .metrics;
        candidates.push(CandidateEvaluation {
            candidate,
            stress: stress_metrics,
            dev,
        });
    }

    let mut top = Vec::new();
    for candidate in select_top(&candidates, params) {
        let cfg = config_for(&candidate, params.window);
        let first_oos = evaluate_slice(
            dataset,
            &cfg,
            split.first_oos,
            params.window,
            params.init_fund,
            base,
        )?;
        let first_oos_stress = evaluate_slice(
            dataset,
            &cfg,
            split.first_oos,
            params.window,
            params.init_fund,
            stress,
        )?
        .metrics;
        let extended_oos = evaluate_slice(
            dataset,
            &cfg,
            split.extended_oos,
            params.window,
            params.init_fund,
            base,
        )?;
        let extended_oos_stress = evaluate_slice(
            dataset,
            &cfg,
            split.extended_oos,
            params.window,
            params.init_fund,
            stress,
        )?
        .metrics;
        let m5_pass = first_oos.metrics.net_expectancy > Decimal::ZERO
            && profit_factor_passes(first_oos.metrics.net_profit_factor);
        top.push(TopCandidate {
            candidate,
            first_oos,
            first_oos_stress,
            extended_oos,
            extended_oos_stress,
            m5_pass,
        });
    }

    let selected = top.first().map(|candidate| candidate.candidate.clone());
    let first_oos = top.first().map(|candidate| candidate.first_oos.clone());
    let first_oos_stress = top
        .first()
        .map(|candidate| candidate.first_oos_stress.clone());
    let extended_oos = top.first().map(|candidate| candidate.extended_oos.clone());
    let extended_oos_stress = top
        .first()
        .map(|candidate| candidate.extended_oos_stress.clone());
    let decision = decision_report(&top);

    Ok(SymbolExperiment {
        symbol: dataset.symbol.clone(),
        dataset: dataset.clone(),
        split,
        candidates,
        top,
        selected,
        first_oos,
        first_oos_stress,
        extended_oos,
        extended_oos_stress,
        decision,
        btc_regime: BTreeMap::new(),
    })
}

/// Development net expectancy range for configs with the same period and
/// threshold changes of at most five points in either direction.
///
/// Returns `(min, median, max, count)`. The selected config is included.
pub fn neighbor_metrics(
    selected: &Candidate,
    candidates: &[CandidateEvaluation],
) -> (Decimal, Decimal, Decimal, usize) {
    let mut values: Vec<Decimal> = candidates
        .iter()
        .filter(|evaluation| {
            let other = &evaluation.candidate;
            other.rsi_period == selected.rsi_period
                && (other.rsi_long_threshold - selected.rsi_long_threshold).abs()
                    <= Decimal::from(5)
                && (other.rsi_short_threshold - selected.rsi_short_threshold).abs()
                    <= Decimal::from(5)
        })
        .map(|evaluation| evaluation.dev.metrics.net_expectancy)
        .collect();
    if values.is_empty() {
        return (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, 0);
    }
    values.sort();
    let min = values[0];
    let max = values[values.len() - 1];
    let median = if values.len().is_multiple_of(2) {
        let upper = values.len() / 2;
        (values[upper - 1] + values[upper]) / Decimal::from(2)
    } else {
        values[values.len() / 2]
    };
    (min, median, max, values.len())
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

    fn evaluation(candidate: Candidate, dev_metrics: PartitionMetrics) -> CandidateEvaluation {
        CandidateEvaluation {
            candidate,
            stress: PartitionMetrics::default(),
            dev: SliceOutcome {
                metrics: dev_metrics,
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
        }
    }

    fn top_candidate(
        candidate: Candidate,
        first_oos_metrics: PartitionMetrics,
        m5_pass: bool,
    ) -> TopCandidate {
        TopCandidate {
            candidate,
            first_oos: SliceOutcome {
                metrics: first_oos_metrics,
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            first_oos_stress: PartitionMetrics::default(),
            extended_oos: SliceOutcome {
                metrics: PartitionMetrics::default(),
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            extended_oos_stress: PartitionMetrics::default(),
            m5_pass,
        }
    }

    #[test]
    fn split_boundaries_follow_the_one_year_policy() {
        let split = split_one_year(1200);
        assert_eq!(split.dev, Slice { start: 0, end: 400 });
        assert_eq!(
            split.first_oos,
            Slice {
                start: 400,
                end: 500
            }
        );
        assert_eq!(
            split.extended_oos,
            Slice {
                start: 500,
                end: 1000
            }
        );

        let split = split_one_year(1000);
        assert_eq!(split.dev, Slice { start: 0, end: 333 });
        assert_eq!(
            split.first_oos,
            Slice {
                start: 333,
                end: 416
            }
        );
        assert_eq!(
            split.extended_oos,
            Slice {
                start: 416,
                end: 833
            }
        );
    }

    #[test]
    fn selection_uses_the_combined_pf_and_expectancy_rank() {
        let params = ExperimentParams::default();
        let candidates = vec![
            evaluation(
                candidate(14, 60, 45, "pf-leader"),
                metrics(20, 0, Some(10), 5),
            ),
            evaluation(
                candidate(21, 65, 40, "expectancy-leader"),
                metrics(20, 20, Some(2), 5),
            ),
            evaluation(
                candidate(21, 60, 50, "balanced"),
                metrics(20, 12, Some(4), 5),
            ),
            evaluation(candidate(14, 65, 45, "mid"), metrics(20, 8, Some(3), 5)),
            evaluation(candidate(14, 55, 40, "weak"), metrics(20, -5, Some(1), 5)),
            evaluation(
                candidate(21, 70, 40, "too-much-dd"),
                metrics(20, 50, Some(20), 25),
            ),
        ];

        let top = select_top(&candidates, &params);

        // Balanced scores 1 + 1 = 2. The PF and expectancy leaders each score
        // 0 + 3 = 3, so the PF leader breaks the tie first.
        assert_eq!(
            top.iter()
                .map(|candidate| candidate.label.as_str())
                .collect::<Vec<_>>(),
            vec!["balanced", "pf-leader", "expectancy-leader"]
        );
        assert!(!top.iter().any(|candidate| candidate.label == "too-much-dd"));
    }

    #[test]
    fn selection_keeps_a_high_pf_candidate_with_few_trades() {
        let params = ExperimentParams::default();
        let candidates = vec![
            evaluation(
                candidate(14, 70, 40, "few-trades"),
                metrics(1, 8, Some(9), 5),
            ),
            evaluation(
                candidate(21, 60, 50, "many-trades"),
                metrics(40, 2, Some(2), 5),
            ),
            evaluation(
                candidate(21, 55, 45, "moderate"),
                metrics(10, 3, Some(3), 5),
            ),
        ];

        let top = select_top(&candidates, &params);

        assert_eq!(top[0].label, "few-trades");
        assert!(top.len() <= params.top_n);
    }

    #[test]
    fn selection_applies_only_the_drawdown_gate() {
        let params = ExperimentParams::default();
        let candidates = vec![
            // Drawdown exactly at the gate still passes.
            evaluation(
                candidate(14, 60, 45, "too-much-dd"),
                metrics(20, 100, Some(20), 20),
            ),
            evaluation(
                candidate(21, 60, 50, "over-dd"),
                metrics(20, 100, Some(20), 21),
            ),
            evaluation(
                candidate(21, 65, 45, "loss-making"),
                metrics(0, -20, Some(0), 0),
            ),
        ];

        let top = select_top(&candidates, &params);

        assert!(top.iter().any(|candidate| candidate.label == "too-much-dd"));
        assert!(!top.iter().any(|candidate| candidate.label == "over-dd"));
        assert!(top.iter().any(|candidate| candidate.label == "loss-making"));
    }

    #[test]
    fn selection_respects_top_n() {
        let params = ExperimentParams {
            top_n: 1,
            ..ExperimentParams::default()
        };
        let candidates = vec![
            evaluation(candidate(14, 60, 45, "first"), metrics(20, 10, Some(3), 5)),
            evaluation(candidate(21, 60, 50, "second"), metrics(20, 8, Some(2), 5)),
            evaluation(candidate(21, 65, 45, "third"), metrics(20, 6, Some(1), 5)),
        ];

        let top = select_top(&candidates, &params);

        assert_eq!(top.len(), 1);
        assert_eq!(top[0].label, "first");
    }

    #[test]
    fn decision_passes_if_any_top_candidate_passes_m5() {
        let top = vec![
            top_candidate(
                candidate(14, 60, 45, "fails"),
                metrics(5, -2, Some(0), 3),
                false,
            ),
            top_candidate(
                candidate(21, 60, 50, "passes"),
                metrics(5, 7, Some(2), 3),
                true,
            ),
        ];

        let report = decision_report(&top);

        assert_eq!(report.verdict, Decision::Pass);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("Top-2 passes")));
    }

    #[test]
    fn decision_fails_if_no_top_candidate_passes_m5() {
        let top = vec![
            top_candidate(
                candidate(14, 60, 45, "weak"),
                metrics(5, -2, Some(1), 3),
                false,
            ),
            top_candidate(
                candidate(21, 60, 50, "flat"),
                metrics(0, 0, Some(0), 0),
                false,
            ),
        ];

        let report = decision_report(&top);

        assert_eq!(report.verdict, Decision::Fail);
        let none = decision_report(&[]);
        assert_eq!(none.verdict, Decision::Fail);
    }

    #[test]
    fn neighbors_are_same_period_within_five_points() {
        let selected = candidate(21, 60, 50, "selected");
        let candidates = vec![
            evaluation(candidate(21, 60, 50, "self"), metrics(20, 10, Some(2), 5)),
            evaluation(candidate(21, 65, 45, "near"), metrics(20, 6, Some(2), 5)),
            evaluation(
                candidate(21, 70, 45, "far-long"),
                metrics(20, 100, Some(2), 5),
            ),
            evaluation(
                candidate(14, 60, 50, "other-period"),
                metrics(20, 200, Some(2), 5),
            ),
            evaluation(
                candidate(21, 60, 45, "near-short"),
                metrics(20, 14, Some(2), 5),
            ),
        ];

        let (min, median, max, count) = neighbor_metrics(&selected, &candidates);

        assert_eq!(count, 3);
        assert_eq!(min, d(6));
        assert_eq!(median, d(10));
        assert_eq!(max, d(14));
    }
}
