use crate::harness::dataset::Dataset;
use crate::harness::experiment::{Decision, DecisionReport, SliceOutcome};
use crate::harness::sim::{
    unrealized_pnl, AccountStep, ClosedTrade, OpenStep, Position, SimOutcome,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::{metrics_from_outcome, PartitionMetrics};
use crate::harness::sweep::partition::{slice_with_warmup, Slice};
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;
use std::cmp::Ordering;

pub const STRATEGY_VERSION: &str = "donchian-v1";
pub const DEFAULT_EMA_PERIOD: usize = 200;
pub const DEFAULT_ATR_PERIOD: usize = 14;
pub const DONCHIAN_EXIT: usize = 55;
pub const ENTRY_GRID: [usize; 2] = [20, 55];
pub const ATR_MULTIPLIERS: [i64; 3] = [2, 3, 4];
pub const TRAIN_MONTHS: [usize; 8] = [12, 18, 24, 30, 36, 42, 48, 54];
pub const OOS_MONTHS: usize = 6;
pub const TOTAL_MONTHS: usize = 60;
pub const TOP_CONFIGS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DonchianConfig {
    pub entry: usize,
    pub exit: usize,
    pub ema_period: usize,
    pub atr_period: usize,
    pub atr_multiplier: Decimal,
}

impl DonchianConfig {
    pub fn label(&self) -> String {
        format!(
            "entry{}-exit{}-ema{}-atr{}x{}",
            self.entry,
            self.exit,
            self.ema_period,
            self.atr_period,
            self.atr_multiplier.normalize()
        )
    }

    pub fn warmup(&self) -> usize {
        self.entry
            .max(self.exit)
            .max(self.ema_period)
            .max(self.atr_period)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DonchianParams {
    pub init_fund: i64,
    pub ema_period: usize,
    pub atr_period: usize,
    pub min_oos_trades: usize,
}

impl Default for DonchianParams {
    fn default() -> Self {
        Self {
            init_fund: 1000,
            ema_period: DEFAULT_EMA_PERIOD,
            atr_period: DEFAULT_ATR_PERIOD,
            min_oos_trades: 10,
        }
    }
}

pub fn config_grid(ema_period: usize, atr_period: usize) -> Vec<DonchianConfig> {
    let mut configs = Vec::with_capacity(ENTRY_GRID.len() * ATR_MULTIPLIERS.len());
    for entry in ENTRY_GRID {
        for multiplier in ATR_MULTIPLIERS {
            configs.push(DonchianConfig {
                entry,
                exit: DONCHIAN_EXIT,
                ema_period,
                atr_period,
                atr_multiplier: Decimal::from(multiplier),
            });
        }
    }
    configs
}

pub fn ema(values: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    let mut out = vec![None; values.len()];
    if period == 0 || values.len() < period {
        return out;
    }
    let seed: Decimal = values[..period].iter().copied().sum();
    out[period - 1] = Some(seed / Decimal::from(period));
    let alpha = Decimal::from(2) / Decimal::from(period + 1);
    let remainder = Decimal::ONE - alpha;
    for index in period..values.len() {
        let previous = out[index - 1].expect("EMA is seeded before the loop");
        out[index] = Some(alpha * values[index] + remainder * previous);
    }
    out
}

pub fn atr(candles: &[Candle], period: usize) -> Vec<Option<Decimal>> {
    let mut out = vec![None; candles.len()];
    if period == 0 || candles.len() < period {
        return out;
    }
    let mut ranges = Vec::with_capacity(candles.len());
    for (index, candle) in candles.iter().enumerate() {
        let range = if index == 0 {
            candle.high - candle.low
        } else {
            let previous_close = candles[index - 1].close;
            (candle.high - candle.low)
                .max((candle.high - previous_close).abs())
                .max((candle.low - previous_close).abs())
        };
        ranges.push(range);
    }
    let seed: Decimal = ranges[..period].iter().copied().sum();
    out[period - 1] = Some(seed / Decimal::from(period));
    let divisor = Decimal::from(period);
    let previous_weight = Decimal::from(period - 1);
    for index in period..candles.len() {
        let previous = out[index - 1].expect("ATR is seeded before the loop");
        out[index] = Some((previous * previous_weight + ranges[index]) / divisor);
    }
    out
}

pub fn donchian_high(candles: &[Candle], period: usize) -> Vec<Option<Decimal>> {
    let mut out = vec![None; candles.len()];
    if period == 0 {
        return out;
    }
    for index in period..candles.len() {
        out[index] = candles[index - period..index]
            .iter()
            .map(|candle| candle.high)
            .max();
    }
    out
}

struct OpenPosition {
    position: Position,
    highest_high: Decimal,
}

pub fn simulate_donchian(
    candles: &[Candle],
    cfg: &DonchianConfig,
    init_fund: i64,
    window: usize,
) -> SimOutcome {
    let init_fund = Decimal::from(init_fund);
    let budget = init_fund / Decimal::from(2);
    let closes: Vec<Decimal> = candles.iter().map(|candle| candle.close).collect();
    let ema_values = ema(&closes, cfg.ema_period);
    let atr_values = atr(candles, cfg.atr_period);
    let donchian_values = donchian_high(candles, cfg.entry);

    let mut available = init_fund;
    let mut state: Option<OpenPosition> = None;
    let mut pending_entry: Option<usize> = None;
    let mut pending_exit = false;
    let mut trades: Vec<ClosedTrade> = Vec::new();
    let mut account: Vec<AccountStep> = Vec::new();
    let mut unfilled_at_end = 0usize;

    for index in window..candles.len() {
        let candle = &candles[index];

        if pending_exit {
            if let Some(open) = state.take() {
                close_position(
                    &mut trades,
                    &mut available,
                    open.position,
                    candle.open,
                    "ema_exit",
                    index,
                );
            }
            pending_exit = false;
        }

        if state.is_none() {
            if let Some(signal_index) = pending_entry.take() {
                available -= budget;
                state = Some(OpenPosition {
                    position: Position {
                        side: Direction::Long,
                        signal_id: format!("donchian-entry-{signal_index}"),
                        signal_entry: candles[signal_index].close,
                        fill: candle.open,
                        e: budget,
                        entry_index: index,
                    },
                    highest_high: candle.open,
                });
            }
        }

        if let Some(open) = state.as_ref() {
            if let Some(previous_atr) = index.checked_sub(1).and_then(|i| atr_values[i]) {
                let stop = open.highest_high - cfg.atr_multiplier * previous_atr;
                let exit = if candle.open <= stop {
                    Some(candle.open)
                } else if candle.low <= stop {
                    Some(stop)
                } else {
                    None
                };
                if let Some(exit) = exit {
                    let open = state
                        .take()
                        .expect("position exists when checking the stop");
                    close_position(
                        &mut trades,
                        &mut available,
                        open.position,
                        exit,
                        "trailing_stop",
                        index,
                    );
                }
            }
        }

        if let Some(open) = state.as_mut() {
            open.highest_high = open.highest_high.max(candle.high);
        }

        let reserved = state
            .as_ref()
            .map(|open| open.position.e)
            .unwrap_or(Decimal::ZERO);
        let mut unrealized = Decimal::ZERO;
        let mut open_steps = Vec::new();
        if let Some(open) = state.as_ref() {
            unrealized = unrealized_pnl(
                open.position.side,
                open.position.fill,
                candle.close,
                open.position.e,
            );
            open_steps.push(OpenStep {
                signal_id: open.position.signal_id.clone(),
                unrealized,
            });
        }
        account.push(AccountStep {
            index,
            available_fund: available,
            reserved,
            unrealized,
            equity: available + reserved + unrealized,
            open: open_steps,
        });

        if state.is_some() {
            if index + 1 < candles.len()
                && ema_values[index]
                    .map(|value| candle.close < value)
                    .unwrap_or(false)
            {
                pending_exit = true;
            }
        } else if index + 1 < candles.len() {
            let above_ema = ema_values[index]
                .map(|value| candle.close > value)
                .unwrap_or(false);
            let above_channel = donchian_values[index]
                .map(|value| candle.close > value)
                .unwrap_or(false);
            if above_ema && above_channel {
                pending_entry = Some(index);
            }
        }
    }

    if pending_entry.is_some() {
        unfilled_at_end += 1;
    }
    if pending_exit {
        unfilled_at_end += 1;
    }

    SimOutcome {
        trades,
        open_positions: state.map(|open| open.position).into_iter().collect(),
        account,
        available_fund: available,
        skipped_funds: 0,
        unfilled_at_end,
    }
}

fn close_position(
    trades: &mut Vec<ClosedTrade>,
    available: &mut Decimal,
    position: Position,
    exit: Decimal,
    reason: &str,
    index: usize,
) {
    let proceeds = position.e * (exit / position.fill);
    let mut pnl = proceeds - position.e;
    if pnl < -position.e {
        pnl = -position.e;
    }
    *available += position.e + pnl;
    trades.push(ClosedTrade {
        side: position.side,
        signal_id: position.signal_id,
        signal_entry: position.signal_entry,
        fill: position.fill,
        exit,
        e: position.e,
        pnl,
        pnl_pct: if position.e.is_zero() {
            Decimal::ZERO
        } else {
            pnl / position.e * Decimal::from(100)
        },
        reason: reason.to_string(),
        entry_index: position.entry_index,
        exit_index: index,
    });
}

pub fn evaluate_donchian_slice(
    dataset: &Dataset,
    cfg: &DonchianConfig,
    slice: Slice,
    window: usize,
    init_fund: i64,
    scenario: &CostScenario,
) -> SliceOutcome {
    let partition = slice_with_warmup(&dataset.candles, slice, window);
    let n_candles = partition.candles.len();
    if n_candles == 0 {
        return SliceOutcome {
            metrics: metrics_from_outcome(
                &SimOutcome::default(),
                Decimal::ZERO,
                0,
                0,
                Decimal::from(init_fund),
                scenario,
            ),
            outcome: SimOutcome::default(),
            candles: Vec::new(),
        };
    }
    let last_index = n_candles - 1;
    let last_close = partition.candles[last_index].close;
    let replayed_candles = n_candles.saturating_sub(window);
    let outcome = simulate_donchian(&partition.candles, cfg, init_fund, window);
    let metrics = metrics_from_outcome(
        &outcome,
        last_close,
        last_index,
        replayed_candles,
        Decimal::from(init_fund),
        scenario,
    );
    SliceOutcome {
        metrics,
        outcome,
        candles: partition.candles,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Benchmark {
    pub net_pnl: Decimal,
    pub return_pct: Decimal,
}

pub fn buy_and_hold(
    slice_candles: &[Candle],
    init_fund: i64,
    scenario: &CostScenario,
) -> Benchmark {
    let Some(first) = slice_candles.first() else {
        return Benchmark::default();
    };
    let Some(last) = slice_candles.last() else {
        return Benchmark::default();
    };
    if first.open <= Decimal::ZERO {
        return Benchmark::default();
    }
    let budget = Decimal::from(init_fund) / Decimal::from(2);
    let exit_notional = budget * last.close / first.open;
    let gross = exit_notional - budget;
    let rate = scenario.per_side_rate();
    let costs = rate * budget + rate * exit_notional;
    let net_pnl = gross - costs;
    Benchmark {
        net_pnl,
        return_pct: if budget.is_zero() {
            Decimal::ZERO
        } else {
            net_pnl / budget * Decimal::from(100)
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DonchianWindow {
    pub months: usize,
    pub train: Slice,
    pub oos: Slice,
}

pub fn split_windows(n: usize) -> Vec<DonchianWindow> {
    TRAIN_MONTHS
        .iter()
        .map(|&months| DonchianWindow {
            months,
            train: Slice {
                start: 0,
                end: n * months / TOTAL_MONTHS,
            },
            oos: Slice {
                start: n * months / TOTAL_MONTHS,
                end: n * (months + OOS_MONTHS) / TOTAL_MONTHS,
            },
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct DonchianEvaluation {
    pub config: DonchianConfig,
    pub train: SliceOutcome,
    pub train_stress: PartitionMetrics,
}

#[derive(Debug, Clone)]
pub struct DonchianRun {
    pub config: DonchianConfig,
    pub window_months: usize,
    pub train: SliceOutcome,
    pub train_stress: PartitionMetrics,
    pub oos: SliceOutcome,
    pub oos_stress: PartitionMetrics,
    pub benchmark_base: Benchmark,
    pub benchmark_stress: Benchmark,
    pub parameter_stable: bool,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct WindowResult {
    pub window: DonchianWindow,
    pub top: Vec<DonchianRun>,
}

#[derive(Debug, Clone)]
pub struct SymbolDonchian {
    pub symbol: String,
    pub dataset: Dataset,
    pub params: DonchianParams,
    pub windows: Vec<WindowResult>,
    pub successful_runs: Vec<DonchianRun>,
    pub winner: Option<DonchianRun>,
    pub decision: DecisionReport,
}

fn profit_factor_passes(profit_factor: Option<Decimal>) -> bool {
    profit_factor
        .map(|value| value > Decimal::ONE)
        .unwrap_or(true)
}

pub(crate) fn run_success(
    oos_stress: &PartitionMetrics,
    benchmark: &Benchmark,
    min_oos_trades: usize,
) -> bool {
    oos_stress.net_expectancy > Decimal::ZERO
        && profit_factor_passes(oos_stress.net_profit_factor)
        && oos_stress.net_pnl > benchmark.net_pnl
        && oos_stress.trades >= min_oos_trades
}

fn compare_profit_factor(a: Option<Decimal>, b: Option<Decimal>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => b.cmp(&a),
    }
}

fn compare_profit_factor_desc(a: Option<Decimal>, b: Option<Decimal>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.cmp(&b),
    }
}

pub(crate) fn select_top_three(evaluations: &[DonchianEvaluation]) -> Vec<&DonchianEvaluation> {
    if evaluations.is_empty() {
        return Vec::new();
    }
    let mut pf_order: Vec<usize> = (0..evaluations.len()).collect();
    pf_order.sort_by(|&a, &b| {
        compare_profit_factor(
            evaluations[a].train.metrics.net_profit_factor,
            evaluations[b].train.metrics.net_profit_factor,
        )
        .then_with(|| {
            evaluations[b]
                .train
                .metrics
                .net_expectancy
                .cmp(&evaluations[a].train.metrics.net_expectancy)
        })
        .then_with(|| {
            evaluations[a]
                .config
                .label()
                .cmp(&evaluations[b].config.label())
        })
    });
    let mut pf_rank = vec![0usize; evaluations.len()];
    for (rank, &index) in pf_order.iter().enumerate() {
        pf_rank[index] = rank + 1;
    }

    let mut expectancy_order: Vec<usize> = (0..evaluations.len()).collect();
    expectancy_order.sort_by(|&a, &b| {
        evaluations[b]
            .train
            .metrics
            .net_expectancy
            .cmp(&evaluations[a].train.metrics.net_expectancy)
            .then_with(|| {
                compare_profit_factor(
                    evaluations[a].train.metrics.net_profit_factor,
                    evaluations[b].train.metrics.net_profit_factor,
                )
            })
            .then_with(|| {
                evaluations[a]
                    .config
                    .label()
                    .cmp(&evaluations[b].config.label())
            })
    });
    let mut expectancy_rank = vec![0usize; evaluations.len()];
    for (rank, &index) in expectancy_order.iter().enumerate() {
        expectancy_rank[index] = rank + 1;
    }

    let mut selected: Vec<usize> = (0..evaluations.len()).collect();
    selected.sort_by(|&a, &b| {
        (pf_rank[a] + expectancy_rank[a])
            .cmp(&(pf_rank[b] + expectancy_rank[b]))
            .then_with(|| pf_rank[a].cmp(&pf_rank[b]))
            .then_with(|| expectancy_rank[a].cmp(&expectancy_rank[b]))
            .then_with(|| {
                evaluations[a]
                    .config
                    .label()
                    .cmp(&evaluations[b].config.label())
            })
    });
    selected.truncate(TOP_CONFIGS);
    selected
        .into_iter()
        .map(|index| &evaluations[index])
        .collect()
}

fn neighbor_stability(evaluations: &[DonchianEvaluation], config: &DonchianConfig) -> bool {
    let Some(entry_index) = ENTRY_GRID.iter().position(|&entry| entry == config.entry) else {
        return false;
    };
    let Some(atr_index) = ATR_MULTIPLIERS
        .iter()
        .position(|&multiplier| Decimal::from(multiplier) == config.atr_multiplier)
    else {
        return false;
    };

    let mut neighbors = 0usize;
    for evaluation in evaluations {
        let other = &evaluation.config;
        if other.exit != config.exit
            || other.ema_period != config.ema_period
            || other.atr_period != config.atr_period
        {
            continue;
        }
        let Some(other_entry) = ENTRY_GRID.iter().position(|&entry| entry == other.entry) else {
            continue;
        };
        let Some(other_atr) = ATR_MULTIPLIERS
            .iter()
            .position(|&multiplier| Decimal::from(multiplier) == other.atr_multiplier)
        else {
            continue;
        };
        if other_entry == entry_index && other_atr == atr_index {
            continue;
        }
        if other_entry.abs_diff(entry_index) <= 1 && other_atr.abs_diff(atr_index) <= 1 {
            neighbors += 1;
            if evaluation.train.metrics.net_expectancy <= Decimal::ZERO {
                return false;
            }
        }
    }
    neighbors > 0
}

fn evaluate_window(
    dataset: &Dataset,
    window: &DonchianWindow,
    params: &DonchianParams,
    base: &CostScenario,
    stress: &CostScenario,
) -> Vec<DonchianRun> {
    let warmup = config_grid(params.ema_period, params.atr_period)
        .first()
        .map(DonchianConfig::warmup)
        .unwrap_or(DEFAULT_EMA_PERIOD);
    let evaluations: Vec<DonchianEvaluation> = config_grid(params.ema_period, params.atr_period)
        .into_iter()
        .map(|config| DonchianEvaluation {
            train: evaluate_donchian_slice(
                dataset,
                &config,
                window.train,
                warmup,
                params.init_fund,
                base,
            ),
            train_stress: evaluate_donchian_slice(
                dataset,
                &config,
                window.train,
                warmup,
                params.init_fund,
                stress,
            )
            .metrics,
            config,
        })
        .collect();
    let oos_candles = &dataset.candles[window.oos.start..window.oos.end];
    let benchmark_base = buy_and_hold(oos_candles, params.init_fund, base);
    let benchmark_stress = buy_and_hold(oos_candles, params.init_fund, stress);
    select_top_three(&evaluations)
        .into_iter()
        .map(|evaluation| {
            let oos = evaluate_donchian_slice(
                dataset,
                &evaluation.config,
                window.oos,
                warmup,
                params.init_fund,
                base,
            );
            let oos_stress = evaluate_donchian_slice(
                dataset,
                &evaluation.config,
                window.oos,
                warmup,
                params.init_fund,
                stress,
            )
            .metrics;
            let parameter_stable = neighbor_stability(&evaluations, &evaluation.config);
            let success = run_success(&oos_stress, &benchmark_stress, params.min_oos_trades)
                && parameter_stable;
            DonchianRun {
                config: evaluation.config.clone(),
                window_months: window.months,
                train: evaluation.train.clone(),
                train_stress: evaluation.train_stress.clone(),
                parameter_stable,
                success,
                oos,
                oos_stress,
                benchmark_base,
                benchmark_stress,
            }
        })
        .collect()
}

fn decision(
    successful: &[DonchianRun],
    windows: &[WindowResult],
    min_oos_trades: usize,
) -> DecisionReport {
    if successful.is_empty() {
        let mut reasons = vec![format!(
            "No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of {} OOS trades, and the parameter-stability rule.",
            min_oos_trades
        )];
        for window in windows {
            if let Some(run) = window.top.iter().max_by(|a, b| {
                a.oos_stress
                    .net_expectancy
                    .cmp(&b.oos_stress.net_expectancy)
                    .then_with(|| {
                        compare_profit_factor_desc(
                            a.oos_stress.net_profit_factor,
                            b.oos_stress.net_profit_factor,
                        )
                    })
            }) {
                reasons.push(format!(
                    "M1-M{}: best top-3 stress run {} had {} OOS trades (minimum {}), net expectancy {}, PF {}, net PnL {} vs benchmark {}; parameter stability {}.",
                    window.window.months,
                    run.config.label(),
                    run.oos_stress.trades,
                    min_oos_trades,
                    run.oos_stress.net_expectancy,
                    run.oos_stress
                        .net_profit_factor
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "infinite".to_string()),
                    run.oos_stress.net_pnl,
                    run.benchmark_stress.net_pnl,
                    if run.parameter_stable { "pass" } else { "fail" }
                ));
            }
        }
        return DecisionReport {
            verdict: Decision::Fail,
            reasons,
        };
    }
    let reasons = successful
        .iter()
        .map(|run| {
            format!(
                "M1-M{}: {} passed OOS stress with {} trades (minimum {}), net expectancy {}, PF {}, net PnL {} > benchmark {}, and parameter stability.",
                run.window_months,
                run.config.label(),
                run.oos_stress.trades,
                min_oos_trades,
                run.oos_stress.net_expectancy,
                run.oos_stress
                    .net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "infinite".to_string()),
                run.oos_stress.net_pnl,
                run.benchmark_stress.net_pnl
            )
        })
        .collect();
    DecisionReport {
        verdict: Decision::Pass,
        reasons,
    }
}

pub fn evaluate_symbol(
    dataset: &Dataset,
    base: &CostScenario,
    stress: &CostScenario,
) -> SymbolDonchian {
    evaluate_symbol_with(dataset, base, stress, &DonchianParams::default())
}

pub fn evaluate_symbol_with(
    dataset: &Dataset,
    base: &CostScenario,
    stress: &CostScenario,
    params: &DonchianParams,
) -> SymbolDonchian {
    let windows: Vec<WindowResult> = split_windows(dataset.candles.len())
        .into_iter()
        .map(|window| WindowResult {
            top: evaluate_window(dataset, &window, params, base, stress),
            window,
        })
        .collect();
    let successful_runs: Vec<DonchianRun> = windows
        .iter()
        .flat_map(|window| window.top.iter())
        .filter(|run| run.success)
        .cloned()
        .collect();
    let winner = successful_runs
        .iter()
        .max_by(|a, b| {
            a.oos_stress
                .net_expectancy
                .cmp(&b.oos_stress.net_expectancy)
                .then_with(|| {
                    compare_profit_factor_desc(
                        a.oos_stress.net_profit_factor,
                        b.oos_stress.net_profit_factor,
                    )
                })
                .then_with(|| {
                    b.oos_stress
                        .max_drawdown_pct
                        .cmp(&a.oos_stress.max_drawdown_pct)
                })
                .then_with(|| b.window_months.cmp(&a.window_months))
                .then_with(|| a.config.label().cmp(&b.config.label()))
        })
        .cloned();
    let decision = decision(&successful_runs, &windows, params.min_oos_trades);
    SymbolDonchian {
        symbol: dataset.symbol.clone(),
        dataset: dataset.clone(),
        params: *params,
        windows,
        successful_runs,
        winner,
        decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::sim::SimOutcome;

    fn d(value: i64) -> Decimal {
        Decimal::from(value)
    }

    fn candle(open: &str, high: &str, low: &str, close: &str) -> Candle {
        Candle {
            open_time_ms: 0,
            open: open.parse().unwrap(),
            high: high.parse().unwrap(),
            low: low.parse().unwrap(),
            close: close.parse().unwrap(),
            volume: Decimal::ZERO,
        }
    }

    fn config(
        entry: usize,
        exit: usize,
        ema_period: usize,
        atr_period: usize,
        multiplier: i64,
    ) -> DonchianConfig {
        DonchianConfig {
            entry,
            exit,
            ema_period,
            atr_period,
            atr_multiplier: d(multiplier),
        }
    }

    fn evaluation(
        entry: usize,
        trades: usize,
        expectancy: i64,
        pf: Option<i64>,
    ) -> DonchianEvaluation {
        let metrics = PartitionMetrics {
            trades,
            net_expectancy: d(expectancy),
            net_profit_factor: pf.map(d),
            ..Default::default()
        };
        DonchianEvaluation {
            config: DonchianConfig {
                entry,
                exit: 55,
                ema_period: 200,
                atr_period: 14,
                atr_multiplier: d(2),
            },
            train: SliceOutcome {
                metrics,
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            train_stress: PartitionMetrics::default(),
        }
    }

    #[test]
    fn ema_uses_sma_seed_and_alpha() {
        let values: Vec<Decimal> = (1..=5).map(d).collect();
        let values: Vec<Decimal> = values;
        let out = ema(&values, 3);
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        assert_eq!(out[2], Some(d(2)));
        assert_eq!(out[3], Some(d(3)));
        assert_eq!(out[4], Some(d(4)));
    }

    #[test]
    fn atr_uses_wilder_smoothing() {
        let candles = vec![
            candle("9", "10", "9", "9"),
            candle("9", "11", "9", "11"),
            candle("11", "13", "10", "12"),
        ];
        let out = atr(&candles, 2);
        assert_eq!(out[0], None);
        assert_eq!(out[1], Some(Decimal::new(15, 1)));
        let third_range = Decimal::from(3);
        let expected = (Decimal::new(15, 1) + third_range) / Decimal::from(2);
        assert_eq!(out[2], Some(expected));
    }

    #[test]
    fn donchian_high_uses_prior_candles_only() {
        let candles = vec![
            candle("1", "1", "1", "1"),
            candle("1", "3", "1", "1"),
            candle("1", "2", "1", "1"),
            candle("1", "5", "1", "1"),
        ];
        let out = donchian_high(&candles, 2);
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        assert_eq!(out[2], Some(d(3)));
        assert_eq!(out[3], Some(d(3)));
    }

    #[test]
    fn simulation_enters_next_open_and_exits_on_trailing_stop() {
        let candles = vec![
            candle("10", "10", "9", "10"),
            candle("10", "10", "9", "10"),
            candle("10", "10.5", "9.5", "10.2"),
            candle("10.5", "11", "10.4", "10.8"),
            candle("10.8", "11", "10", "10.5"),
        ];
        let cfg = config(2, 3, 2, 1, 1);
        let outcome = simulate_donchian(&candles, &cfg, 1000, 2);
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.entry_index, 3);
        assert_eq!(trade.fill, Decimal::new(105, 1));
        assert_eq!(trade.reason, "trailing_stop");
        assert_eq!(trade.exit, Decimal::new(102, 1));
        assert_eq!(outcome.open_positions.len(), 0);
        assert_eq!(outcome.account.len(), 3);
    }

    #[test]
    fn simulation_exits_ema_cross_at_next_open() {
        let candles = vec![
            candle("10", "10", "9", "10"),
            candle("10", "10", "9", "10"),
            candle("10", "10.5", "9.5", "10.2"),
            candle("10.5", "11", "10.4", "10.8"),
            candle("10.8", "10.9", "8.8", "9.0"),
            candle("9.2", "9.3", "9.0", "9.1"),
        ];
        let cfg = config(2, 3, 2, 1, 100);
        let outcome = simulate_donchian(&candles, &cfg, 1000, 2);
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.reason, "ema_exit");
        assert_eq!(trade.exit, Decimal::new(92, 1));
    }

    #[test]
    fn simulation_gap_below_stop_exits_at_open() {
        let candles = vec![
            candle("10", "10", "9", "10"),
            candle("10", "10", "9", "10"),
            candle("10", "10.5", "9.5", "10.2"),
            candle("10.5", "11", "10.4", "10.8"),
            candle("9.0", "9.1", "8.8", "9.0"),
        ];
        let cfg = config(2, 3, 2, 1, 1);
        let outcome = simulate_donchian(&candles, &cfg, 1000, 2);
        assert_eq!(outcome.trades.len(), 1);
        assert_eq!(outcome.trades[0].reason, "trailing_stop");
        assert_eq!(outcome.trades[0].exit, d(9));
    }

    #[test]
    fn top_three_uses_profit_factor_and_expectancy_rank_sum() {
        let items = vec![
            evaluation(20, 10, 50, Some(2)),
            evaluation(21, 10, 10, Some(4)),
            evaluation(22, 10, 40, None),
            evaluation(23, 10, 5, Some(3)),
            evaluation(24, 10, 100, Some(1)),
            evaluation(25, 10, 0, Some(5)),
        ];
        let selected = select_top_three(&items);
        let labels: Vec<String> = selected.iter().map(|item| item.config.label()).collect();
        assert_eq!(selected.len(), 3);
        assert_eq!(labels[0], "entry22-exit55-ema200-atr14x2");
        assert_eq!(labels[1], "entry21-exit55-ema200-atr14x2");
        assert_eq!(labels[2], "entry20-exit55-ema200-atr14x2");
    }

    fn evaluation_config(entry: usize, multiplier: i64, expectancy: i64) -> DonchianEvaluation {
        DonchianEvaluation {
            config: config(
                entry,
                DONCHIAN_EXIT,
                DEFAULT_EMA_PERIOD,
                DEFAULT_ATR_PERIOD,
                multiplier,
            ),
            train: SliceOutcome {
                metrics: PartitionMetrics {
                    net_expectancy: d(expectancy),
                    ..Default::default()
                },
                outcome: SimOutcome::default(),
                candles: Vec::new(),
            },
            train_stress: PartitionMetrics::default(),
        }
    }

    #[test]
    fn success_gate_requires_positive_expectancy_pf_and_benchmark_beat() {
        let benchmark = Benchmark {
            net_pnl: d(10),
            return_pct: d(2),
        };
        let passing = PartitionMetrics {
            trades: 10,
            net_expectancy: d(1),
            net_profit_factor: Some(d(2)),
            net_pnl: d(11),
            ..Default::default()
        };
        let losing_pf = PartitionMetrics {
            net_profit_factor: Some(Decimal::ONE),
            ..passing.clone()
        };
        let below_benchmark = PartitionMetrics {
            net_pnl: d(10),
            ..passing.clone()
        };
        assert!(run_success(&passing, &benchmark, 10));
        assert!(!run_success(&losing_pf, &benchmark, 10));
        assert!(!run_success(&below_benchmark, &benchmark, 10));
    }

    #[test]
    fn success_gate_requires_minimum_oos_trades() {
        let benchmark = Benchmark {
            net_pnl: d(10),
            return_pct: d(2),
        };
        let enough = PartitionMetrics {
            trades: 10,
            net_expectancy: d(1),
            net_profit_factor: Some(d(2)),
            net_pnl: d(11),
            ..Default::default()
        };
        let too_few = PartitionMetrics {
            trades: 9,
            ..enough.clone()
        };
        assert!(run_success(&enough, &benchmark, 10));
        assert!(!run_success(&too_few, &benchmark, 10));
    }

    #[test]
    fn parameter_stability_requires_all_neighbors_positive() {
        let selected = evaluation_config(20, 2, 10);
        let stable = vec![
            evaluation_config(20, 2, 10),
            evaluation_config(55, 2, 5),
            evaluation_config(20, 3, 1),
            evaluation_config(55, 3, 2),
            evaluation_config(55, 4, -5),
        ];
        assert!(neighbor_stability(&stable, &selected.config));

        let unstable = vec![
            evaluation_config(20, 2, 10),
            evaluation_config(55, 2, -1),
            evaluation_config(20, 3, 1),
        ];
        assert!(!neighbor_stability(&unstable, &selected.config));
        assert!(!neighbor_stability(&[selected], &stable[0].config));
    }

    #[test]
    fn benchmark_uses_half_budget_and_both_sides_of_stress_costs() {
        let candles = vec![
            candle("100", "101", "99", "100"),
            candle("100", "111", "99", "110"),
        ];
        let benchmark = buy_and_hold(&candles, 1000, &CostScenario::stress());
        let rate = Decimal::new(30, 4);
        let entry_cost = rate * d(500);
        let exit_notional = d(500) * d(110) / d(100);
        let expected = (exit_notional - d(500)) - entry_cost - rate * exit_notional;
        assert_eq!(benchmark.net_pnl, expected);
        assert_eq!(benchmark.return_pct, expected / d(500) * d(100));
    }

    #[test]
    fn split_windows_cover_the_expected_months() {
        let windows = split_windows(6000);
        assert_eq!(windows.len(), 8);
        assert_eq!(windows[0].months, 12);
        assert_eq!(
            windows[0].train,
            Slice {
                start: 0,
                end: 1200
            }
        );
        assert_eq!(
            windows[0].oos,
            Slice {
                start: 1200,
                end: 1800
            }
        );
        assert_eq!(windows[7].months, 54);
        assert_eq!(
            windows[7].train,
            Slice {
                start: 0,
                end: 5400
            }
        );
        assert_eq!(
            windows[7].oos,
            Slice {
                start: 5400,
                end: 6000
            }
        );
        for pair in windows.windows(2) {
            assert_eq!(pair[0].oos.end, pair[1].oos.start);
        }
    }

    #[test]
    fn short_history_produces_no_signals_without_error() {
        let dataset = Dataset {
            symbol: "TEST-USDT".into(),
            candles: (0..150)
                .map(|index| Candle {
                    open_time_ms: index,
                    open: d(10),
                    high: d(11),
                    low: d(9),
                    close: d(10),
                    volume: Decimal::ZERO,
                })
                .collect(),
            provenance: crate::harness::dataset::Provenance {
                source: "test".into(),
                requested: crate::harness::dataset::RequestedRange {
                    limit: 150,
                    timeframe: "1day".into(),
                    as_of_ms: 0,
                },
                effective_range: crate::harness::dataset::EffectiveRange {
                    first_open_time_ms: 0,
                    last_open_time_ms: 149,
                    candles: 150,
                },
                content_sha256: String::new(),
                fetched_at_ms: None,
                gaps: 0,
            },
        };
        let result = evaluate_symbol(&dataset, &CostScenario::base(), &CostScenario::stress());
        assert_eq!(result.windows.len(), 8);
        assert!(result
            .windows
            .iter()
            .flat_map(|window| window.top.iter())
            .all(|run| run.oos.metrics.trades == 0));
        assert_eq!(result.decision.verdict, Decision::Fail);
    }

    #[test]
    fn top_three_prefers_infinite_profit_factor() {
        let items = vec![evaluation(20, 5, 10, Some(2)), evaluation(21, 5, 5, None)];
        let selected = select_top_three(&items);
        assert_eq!(selected[0].config.label(), "entry21-exit55-ema200-atr14x2");
    }
}
