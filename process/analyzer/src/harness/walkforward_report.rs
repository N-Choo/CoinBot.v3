use crate::harness::experiment::{
    neighbor_metrics, regime_metrics, side_metrics, Decision, DecisionReport, RegimeMetrics,
    SideMetrics, STRATEGY_VERSION,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::partition::Slice;
use crate::harness::sweep::sampling::Candidate;
use crate::harness::walkforward::{
    overall_decision, ConfigRun, SymbolWalkForward, WalkForwardParams, WindowResult,
};
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

fn ensure_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| error.to_string())
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn candle_time(candles: &[Candle], index: usize) -> String {
    candles
        .get(index)
        .map(|candle| iso(candle.open_time_ms))
        .unwrap_or_else(|| "—".to_string())
}

#[derive(Debug, Clone, Serialize)]
struct RangeView {
    start_index: usize,
    end_index: usize,
    start_time: String,
    end_time: String,
}

fn range_view(candles: &[Candle], slice: Slice) -> RangeView {
    RangeView {
        start_index: slice.start,
        end_index: slice.end,
        start_time: candle_time(candles, slice.start),
        end_time: candle_time(candles, slice.end.saturating_sub(1)),
    }
}

#[derive(Debug, Clone, Serialize)]
struct MetricsView {
    trades: usize,
    net_expectancy: Decimal,
    net_profit_factor: Option<Decimal>,
    max_drawdown: Decimal,
    max_drawdown_pct: Decimal,
    expectancy_stderr: Decimal,
    closed_net_pnl: Decimal,
    open_net_pnl: Decimal,
    net_pnl: Decimal,
    win_rate: Decimal,
}

impl MetricsView {
    fn from_metrics(metrics: &PartitionMetrics) -> Self {
        Self {
            trades: metrics.trades,
            net_expectancy: metrics.net_expectancy,
            net_profit_factor: metrics.net_profit_factor,
            max_drawdown: metrics.max_drawdown,
            max_drawdown_pct: metrics.max_drawdown_pct,
            expectancy_stderr: metrics.expectancy_stderr,
            closed_net_pnl: metrics.closed_net_pnl,
            open_net_pnl: metrics.open_net_pnl,
            net_pnl: metrics.net_pnl,
            win_rate: metrics.win_rate,
        }
    }
}

fn decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn profit_factor(value: Option<Decimal>, trades: usize) -> String {
    match value {
        Some(value) => decimal(value),
        None if trades == 0 => "—".to_string(),
        None => "∞".to_string(),
    }
}

fn window_label(split_months: usize) -> String {
    format!("M1-M{split_months}")
}

fn oos_label(split_months: usize, oos_months: usize) -> String {
    format!("M{}-M{}", split_months + 1, split_months + oos_months)
}

fn write_side_table(out: &mut String, title: &str, rows: &[(&str, SideMetrics)]) {
    writeln!(out, "**{title}**").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for (label, metrics) in rows {
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            metrics.trades,
            decimal(metrics.net_pnl),
            decimal(metrics.open_mark),
            decimal(metrics.net_expectancy),
            profit_factor(metrics.profit_factor, metrics.trades),
            decimal(metrics.win_rate)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_dataset_section(out: &mut String, result: &SymbolWalkForward, params: &WalkForwardParams) {
    let provenance = &result.dataset.provenance;
    let candles = &result.dataset.candles;
    writeln!(out, "## 1. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", result.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", provenance.requested.timeframe)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Date range | {} .. {} |",
        candle_time(candles, 0),
        candle_time(candles, candles.len().saturating_sub(1))
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", candles.len()).expect("writing to String cannot fail");
    writeln!(out, "| SHA-256 | {} |", provenance.content_sha256)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| Train window | Train candles | OOS window | OOS candles | Train first | Train last | OOS first | OOS last |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---|---:|---|---|---|---|").expect("writing to String cannot fail");
    for window in &result.windows {
        let train = range_view(candles, window.split.train);
        let oos = range_view(candles, window.split.oos);
        writeln!(
            out,
            "| {} | {}..{} | {} | {}..{} | {} | {} | {} | {} |",
            window_label(window.split.months),
            train.start_index,
            train.end_index,
            oos_label(window.split.months, params.oos_months),
            oos.start_index,
            oos.end_index,
            train.start_time,
            train.end_time,
            oos.start_time,
            oos.end_time
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_experiment_section(
    out: &mut String,
    params: &WalkForwardParams,
    base: &CostScenario,
    stress: &CostScenario,
    symbols: &[String],
) {
    writeln!(out, "## 2. Experiment").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy version | {STRATEGY_VERSION} |")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbols | {} ({}) |",
        symbols.join(", "),
        symbols.len()
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| RSI grid | period 14, 21 × long 55, 60, 65, 70 × short 40, 45, 50 (24 candidates) |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Base costs | {} bps fee + {} bps slippage per side |",
        base.fee_bps, base.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Stress costs | {} bps fee + {} bps slippage per side |",
        stress.fee_bps, stress.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Initial fund | {} |", params.init_fund)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Simulation window | {} warm-up candles |",
        params.window
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Execution assumptions:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- A signal produced at a candle close fills at the next candle open."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- One position per symbol, reversal-only exits.")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Train selection and OOS success use base costs; stress costs are reported for context."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Training gates:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- At least {} trades, net expectancy > 0, profit factor > 1, max drawdown <= {}%.",
        params.min_train_trades,
        decimal(params.max_dd_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Eligible candidates are ranked by train net expectancy; the best two are tested OOS."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "OOS success gates:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- At least {} trades, net expectancy > 0, profit factor > 1 (max drawdown is reported, not gated).",
        params.min_oos_trades
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Decision policy:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- A symbol passes if at least one tested config in any window is successful; the config does not need to be the same across symbols."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The overall experiment passes only if all {} symbols pass.",
        symbols.len()
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_validation_section(
    out: &mut String,
    result: &SymbolWalkForward,
    params: &WalkForwardParams,
) {
    writeln!(out, "## 3. Validation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Top two eligible candidates per expanding training window, base costs."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for window in &result.windows {
        writeln!(
            out,
            "### {} training ({})",
            window_label(window.split.months),
            oos_label(window.split.months, params.oos_months)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        if window.top.is_empty() {
            writeln!(out, "No candidate met the training gates.")
                .expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }
        writeln!(
            out,
            "| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---:|---|---:|---:|---:|---:|---:|---:|")
            .expect("writing to String cannot fail");
        for (index, run) in window.top.iter().enumerate() {
            let metrics = &run.train.metrics;
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                index + 1,
                run.candidate.label,
                metrics.trades,
                decimal(metrics.net_expectancy),
                profit_factor(metrics.net_profit_factor, metrics.trades),
                decimal(metrics.max_drawdown_pct),
                decimal(metrics.expectancy_stderr),
                decimal(run.train_stress.net_expectancy)
            )
            .expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_oos_rows(out: &mut String, run: &ConfigRun, split_months: usize) {
    let base = MetricsView::from_metrics(&run.oos.metrics);
    let stress = MetricsView::from_metrics(&run.oos_stress);
    for (label, metrics) in [("Base", &base), ("Stress", &stress)] {
        let marker = if label == "Base" {
            if run.successful {
                "yes"
            } else {
                "no"
            }
        } else {
            "—"
        };
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            window_label(split_months),
            run.candidate.label,
            label,
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            decimal(metrics.closed_net_pnl),
            decimal(metrics.open_net_pnl),
            marker
        )
        .expect("writing to String cannot fail");
    }
}

fn write_holdout_section(out: &mut String, result: &SymbolWalkForward, params: &WalkForwardParams) {
    writeln!(out, "## 4. Holdout").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if result.windows.iter().all(|window| window.top.is_empty()) {
        writeln!(out, "No candidate was tested out of sample.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    writeln!(
        out,
        "Top two candidates per window on the following five months. Success uses base costs."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|")
        .expect("writing to String cannot fail");
    for window in &result.windows {
        for run in &window.top {
            write_oos_rows(out, run, window.split.months);
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The OOS interval for a window ending at month k is M(k+1)..M(k+{}), using {} months.",
        params.oos_months, params.oos_months
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn tested_labels(windows: &[WindowResult]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut labels = Vec::new();
    for window in windows {
        for run in &window.top {
            if seen.insert(run.candidate.label.clone()) {
                labels.push(run.candidate.label.clone());
            }
        }
    }
    labels
}

fn write_matrix(out: &mut String, result: &SymbolWalkForward) {
    let labels = tested_labels(&result.windows);
    writeln!(
        out,
        "**Successful-config matrix (window × tested config; base costs)**"
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if labels.is_empty() {
        writeln!(out, "No configs were tested out of sample.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    write!(out, "| Window |").expect("writing to String cannot fail");
    for label in &labels {
        write!(out, " {label} |").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    write!(out, "|---|").expect("writing to String cannot fail");
    for _ in &labels {
        write!(out, "---|").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    for window in &result.windows {
        write!(out, "| {} |", window_label(window.split.months))
            .expect("writing to String cannot fail");
        for label in &labels {
            let cell = window
                .top
                .iter()
                .find(|run| &run.candidate.label == label)
                .map(|run| if run.successful { "Pass" } else { "Fail" })
                .unwrap_or("—");
            write!(out, " {cell} |").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn window_for_winner<'a>(
    result: &'a SymbolWalkForward,
    winner: &ConfigRun,
) -> Option<&'a WindowResult> {
    result
        .windows
        .iter()
        .find(|window| window.split.months == winner.window_months)
}

fn write_robustness_section(
    out: &mut String,
    result: &SymbolWalkForward,
    params: &WalkForwardParams,
    base: &CostScenario,
) {
    writeln!(out, "## 5. Robustness").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_matrix(out, result);

    let Some(winner) = &result.winner else {
        writeln!(
            out,
            "No successful config was found; side, regime, and neighbor robustness are unavailable."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    };

    let last_close = winner
        .oos
        .candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or(Decimal::ZERO);
    let long = side_metrics(&winner.oos.outcome, base, Direction::Long, last_close);
    let short = side_metrics(&winner.oos.outcome, base, Direction::Short, last_close);
    write_side_table(
        out,
        &format!(
            "Long vs short ({}, OOS {}, base costs)",
            winner.candidate.label,
            oos_label(winner.window_months, params.oos_months)
        ),
        &[("Long", long), ("Short", short)],
    );

    if result.btc_regime.is_empty() {
        writeln!(out, "**BTC regime (winner OOS, base costs)**")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "BTC regime data was unavailable; all entries are unknown."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
    } else {
        let RegimeMetrics {
            bull,
            bear,
            unknown,
        } = regime_metrics(
            &winner.oos.outcome,
            base,
            &winner.oos.candles,
            &result.btc_regime,
            last_close,
        );
        write_side_table(
            out,
            "BTC regime (winner OOS, base costs; bull = BTC close > 200-period SMA at entry)",
            &[("Bull", bull), ("Bear", bear), ("Unknown", unknown)],
        );
    }

    writeln!(
        out,
        "**Neighboring configs for the winner (training window, base costs)**"
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if let Some(window) = window_for_winner(result, winner) {
        let (min, median, max, count) = neighbor_metrics(&winner.candidate, &window.candidates);
        writeln!(
            out,
            "Same RSI period and long/short thresholds within ±5 points, including the winner. Training window {}.",
            window_label(window.split.months)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(out, "| Min net expectancy | Median | Max | Count |")
            .expect("writing to String cannot fail");
        writeln!(out, "|---:|---:|---:|---:|").expect("writing to String cannot fail");
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            decimal(min),
            decimal(median),
            decimal(max),
            count
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
    } else {
        writeln!(out, "The winner's training window was not found.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_decision_section(
    out: &mut String,
    result: &SymbolWalkForward,
    overall: Decision,
    expected_symbols: usize,
) {
    writeln!(out, "## 6. Decision").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Per-symbol verdict: {}**",
        result.decision.verdict.as_str()
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Overall experiment verdict: {}** (all {} symbols must pass)",
        overall.as_str(),
        expected_symbols
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Reasons:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for reason in &result.decision.reasons {
        writeln!(out, "- {reason}").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    if result.successful_runs.is_empty() {
        writeln!(out, "Successful configs: none.").expect("writing to String cannot fail");
    } else {
        writeln!(out, "Successful configs:").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "| Window | Config | OOS trades | OOS net expectancy | OOS PF |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---:|---:|---:|").expect("writing to String cannot fail");
        for run in &result.successful_runs {
            writeln!(
                out,
                "| {} | {} | {} | {} | {} |",
                window_label(run.window_months),
                run.candidate.label,
                run.oos.metrics.trades,
                decimal(run.oos.metrics.net_expectancy),
                profit_factor(run.oos.metrics.net_profit_factor, run.oos.metrics.trades)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

#[derive(Debug, Clone, Serialize)]
struct RunView {
    candidate: Candidate,
    window_months: usize,
    train: MetricsView,
    train_stress: MetricsView,
    oos: MetricsView,
    oos_stress: MetricsView,
    successful: bool,
}

impl RunView {
    fn from_run(run: &ConfigRun) -> Self {
        Self {
            candidate: run.candidate.clone(),
            window_months: run.window_months,
            train: MetricsView::from_metrics(&run.train.metrics),
            train_stress: MetricsView::from_metrics(&run.train_stress),
            oos: MetricsView::from_metrics(&run.oos.metrics),
            oos_stress: MetricsView::from_metrics(&run.oos_stress),
            successful: run.successful,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct WindowView {
    months: usize,
    train: RangeView,
    oos: RangeView,
    top: Vec<RunView>,
}

#[derive(Debug, Clone, Serialize)]
struct CostView {
    label: String,
    fee_bps: Decimal,
    slip_bps: Decimal,
}

impl CostView {
    fn from_scenario(scenario: &CostScenario) -> Self {
        Self {
            label: scenario.label.clone(),
            fee_bps: scenario.fee_bps,
            slip_bps: scenario.slip_bps,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SymbolResults {
    symbol: String,
    timeframe: String,
    strategy_version: String,
    first_open_time: String,
    last_open_time: String,
    candles: usize,
    content_sha256: String,
    params: WalkForwardParams,
    base_costs: CostView,
    stress_costs: CostView,
    windows: Vec<WindowView>,
    successful_runs: Vec<RunView>,
    winner: Option<RunView>,
    decision: DecisionReport,
    overall_verdict: Decision,
    expected_symbols: usize,
}

fn symbol_results(
    result: &SymbolWalkForward,
    params: &WalkForwardParams,
    base: &CostScenario,
    stress: &CostScenario,
    overall: Decision,
    expected_symbols: usize,
) -> SymbolResults {
    let candles = &result.dataset.candles;
    SymbolResults {
        symbol: result.symbol.clone(),
        timeframe: result.dataset.provenance.requested.timeframe.clone(),
        strategy_version: STRATEGY_VERSION.to_string(),
        first_open_time: candle_time(candles, 0),
        last_open_time: candle_time(candles, candles.len().saturating_sub(1)),
        candles: candles.len(),
        content_sha256: result.dataset.provenance.content_sha256.clone(),
        params: params.clone(),
        base_costs: CostView::from_scenario(base),
        stress_costs: CostView::from_scenario(stress),
        windows: result
            .windows
            .iter()
            .map(|window| WindowView {
                months: window.split.months,
                train: range_view(candles, window.split.train),
                oos: range_view(candles, window.split.oos),
                top: window.top.iter().map(RunView::from_run).collect(),
            })
            .collect(),
        successful_runs: result
            .successful_runs
            .iter()
            .map(RunView::from_run)
            .collect(),
        winner: result.winner.as_ref().map(RunView::from_run),
        decision: result.decision.clone(),
        overall_verdict: overall,
        expected_symbols,
    }
}

const LEADERBOARD_COLUMNS: [&str; 23] = [
    "window_months",
    "train_start",
    "train_end",
    "oos_start",
    "oos_end",
    "label",
    "rsi_period",
    "rsi_long_threshold",
    "rsi_short_threshold",
    "train_trades",
    "train_net_expectancy",
    "train_net_profit_factor",
    "train_max_drawdown_pct",
    "train_expectancy_stderr",
    "train_stress_net_expectancy",
    "oos_trades",
    "oos_net_expectancy",
    "oos_net_profit_factor",
    "oos_max_drawdown_pct",
    "oos_expectancy_stderr",
    "oos_closed_net_pnl",
    "oos_open_mark",
    "successful",
];

fn write_leaderboard(path: &Path, result: &SymbolWalkForward) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(LEADERBOARD_COLUMNS)
        .map_err(|error| error.to_string())?;
    for window in &result.windows {
        for run in &window.top {
            let train = &run.train.metrics;
            let oos = &run.oos.metrics;
            let record = [
                run.window_months.to_string(),
                window.split.train.start.to_string(),
                window.split.train.end.to_string(),
                window.split.oos.start.to_string(),
                window.split.oos.end.to_string(),
                run.candidate.label.clone(),
                run.candidate.rsi_period.to_string(),
                run.candidate.rsi_long_threshold.to_string(),
                run.candidate.rsi_short_threshold.to_string(),
                train.trades.to_string(),
                train.net_expectancy.to_string(),
                train
                    .net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                train.max_drawdown_pct.to_string(),
                train.expectancy_stderr.to_string(),
                run.train_stress.net_expectancy.to_string(),
                oos.trades.to_string(),
                oos.net_expectancy.to_string(),
                oos.net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                oos.max_drawdown_pct.to_string(),
                oos.expectancy_stderr.to_string(),
                oos.closed_net_pnl.to_string(),
                oos.open_net_pnl.to_string(),
                run.successful.to_string(),
            ];
            writer
                .write_record(record)
                .map_err(|error| error.to_string())?;
        }
    }
    writer.flush().map_err(|error| error.to_string())
}

/// Writes `report.md`, `leaderboard.csv`, and `results.json` for one symbol.
#[allow(clippy::too_many_arguments)]
pub fn write_symbol_report(
    dir: &Path,
    result: &SymbolWalkForward,
    params: &WalkForwardParams,
    base: &CostScenario,
    stress: &CostScenario,
    symbols: &[String],
    overall: Decision,
) -> Result<(), String> {
    ensure_dir(dir)?;

    let mut out = String::new();
    writeln!(
        out,
        "# RSI Walk-Forward — {} (4hour, 15 months)",
        result.symbol
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Research artifact for `{STRATEGY_VERSION}`. This is not a profitability claim."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_dataset_section(&mut out, result, params);
    write_experiment_section(&mut out, params, base, stress, symbols);
    write_validation_section(&mut out, result, params);
    write_holdout_section(&mut out, result, params);
    write_robustness_section(&mut out, result, params, base);
    write_decision_section(&mut out, result, overall, symbols.len());

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;
    write_leaderboard(&dir.join("leaderboard.csv"), result)?;

    let results = symbol_results(result, params, base, stress, overall, symbols.len());
    let json = serde_json::to_string_pretty(&results).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize)]
struct SymbolSummary {
    symbol: String,
    candles: usize,
    winner: Option<RunView>,
    successful_configs: usize,
    verdict: Decision,
}

#[derive(Debug, Clone, Serialize)]
struct CombinedResults {
    strategy_version: String,
    timeframe: String,
    expected_symbols: usize,
    symbols: Vec<SymbolSummary>,
    overall_verdict: Decision,
    data_issues: Vec<String>,
}

fn combined_results(
    results: &[SymbolWalkForward],
    expected_symbols: usize,
    errors: &[String],
) -> CombinedResults {
    let verdicts: Vec<Decision> = results
        .iter()
        .map(|result| result.decision.verdict)
        .collect();
    let overall = overall_decision(&verdicts, expected_symbols);
    CombinedResults {
        strategy_version: STRATEGY_VERSION.to_string(),
        timeframe: "4hour".to_string(),
        expected_symbols,
        symbols: results
            .iter()
            .map(|result| SymbolSummary {
                symbol: result.symbol.clone(),
                candles: result.dataset.candles.len(),
                winner: result.winner.as_ref().map(RunView::from_run),
                successful_configs: result.successful_runs.len(),
                verdict: result.decision.verdict,
            })
            .collect(),
        overall_verdict: overall,
        data_issues: errors.to_vec(),
    }
}

/// Writes the combined `README.md` and `results.json` index for a multi-symbol run.
pub fn write_combined_index(
    dir: &Path,
    results: &[SymbolWalkForward],
    errors: &[String],
    expected_symbols: usize,
) -> Result<(), String> {
    ensure_dir(dir)?;
    let combined = combined_results(results, expected_symbols, errors);

    let mut out = String::new();
    writeln!(out, "# RSI Walk-Forward — 15 Months, 4hour").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Expanding training windows M1-M6 through M1-M10, each tested on the following five months. A symbol passes when at least one tested config in any window is successful; the overall experiment passes only when all {} symbols pass.",
        expected_symbols
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Overall verdict: {}**",
        combined.overall_verdict.as_str()
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Candles | Winner | Window | OOS trades | OOS expectancy | OOS PF | Successful configs | Verdict |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---|---|---:|---:|---:|---:|---|")
        .expect("writing to String cannot fail");
    for result in results {
        let winner_label = result
            .winner
            .as_ref()
            .map(|run| run.candidate.label.clone())
            .unwrap_or_else(|| "none".to_string());
        let winner_window = result
            .winner
            .as_ref()
            .map(|run| window_label(run.window_months))
            .unwrap_or_else(|| "—".to_string());
        let oos = result.winner.as_ref().map(|run| &run.oos.metrics);
        writeln!(
            out,
            "| [{}]({}/report.md) | {} | {} | {} | {} | {} | {} | {} | {} |",
            result.symbol,
            result.symbol,
            result.dataset.candles.len(),
            winner_label,
            winner_window,
            oos.map(|metrics| metrics.trades.to_string())
                .unwrap_or_else(|| "—".to_string()),
            oos.map(|metrics| decimal(metrics.net_expectancy))
                .unwrap_or_else(|| "—".to_string()),
            oos.map(|metrics| profit_factor(metrics.net_profit_factor, metrics.trades))
                .unwrap_or_else(|| "—".to_string()),
            result.successful_runs.len(),
            result.decision.verdict.as_str()
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    if !errors.is_empty() {
        writeln!(out, "## Data issues").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        for error in errors {
            writeln!(out, "- {error}").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
    writeln!(
        out,
        "Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`."
    )
    .expect("writing to String cannot fail");
    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())?;

    let json = serde_json::to_string_pretty(&combined).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::experiment::{CandidateEvaluation, SliceOutcome};

    fn candle(open_time_ms: i64) -> Candle {
        Candle {
            open_time_ms,
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
            volume: Decimal::ZERO,
        }
    }

    #[test]
    fn ranges_use_open_times_for_boundaries() {
        let candles: Vec<Candle> = (0..10).map(|index| candle(index as i64 * 1000)).collect();
        let view = range_view(&candles, Slice { start: 2, end: 5 });
        assert_eq!(view.start_index, 2);
        assert_eq!(view.end_index, 5);
        assert_eq!(view.start_time, iso(2000));
        assert_eq!(view.end_time, iso(4000));
    }

    #[test]
    fn window_labels_follow_the_expanding_schedule() {
        assert_eq!(window_label(6), "M1-M6");
        assert_eq!(oos_label(6, 5), "M7-M11");
        assert_eq!(oos_label(10, 5), "M11-M15");
    }

    #[test]
    fn tested_labels_are_unique_in_first_seen_order() {
        let unused: Vec<CandidateEvaluation> = Vec::new();
        let window = |months: usize, labels: &[&str]| WindowResult {
            split: crate::harness::walkforward::WindowSplit {
                months,
                train: Slice {
                    start: 0,
                    end: months,
                },
                oos: Slice {
                    start: months,
                    end: months + 5,
                },
            },
            top: labels
                .iter()
                .map(|label| ConfigRun {
                    candidate: Candidate {
                        label: label.to_string(),
                        origin: "test".to_string(),
                        rsi_period: 21,
                        rsi_long_threshold: Decimal::from(60),
                        rsi_short_threshold: Decimal::from(50),
                    },
                    window_months: months,
                    train: SliceOutcome {
                        metrics: PartitionMetrics::default(),
                        outcome: crate::harness::sim::SimOutcome::default(),
                        candles: Vec::new(),
                    },
                    train_stress: PartitionMetrics::default(),
                    oos: SliceOutcome {
                        metrics: PartitionMetrics::default(),
                        outcome: crate::harness::sim::SimOutcome::default(),
                        candles: Vec::new(),
                    },
                    oos_stress: PartitionMetrics::default(),
                    successful: false,
                })
                .collect(),
            candidates: unused.clone(),
        };
        let windows = vec![window(6, &["a", "b"]), window(7, &["b", "c"])];
        assert_eq!(tested_labels(&windows), vec!["a", "b", "c"]);
    }
}
