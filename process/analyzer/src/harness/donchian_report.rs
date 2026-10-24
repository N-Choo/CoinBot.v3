use crate::harness::donchian::{
    Benchmark, DonchianConfig, DonchianRun, SymbolDonchian, WindowResult, ATR_MULTIPLIERS,
    DONCHIAN_EXIT, ENTRY_GRID, OOS_MONTHS, STRATEGY_VERSION, TOP_CONFIGS, TOTAL_MONTHS,
    TRAIN_MONTHS,
};
use crate::harness::experiment::{Decision, DecisionReport};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::partition::Slice;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
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

fn window_label(months: usize) -> String {
    format!("M1-M{months}")
}

fn oos_label(months: usize) -> String {
    format!("M{}-M{}", months + 1, months + OOS_MONTHS)
}

fn walk_forward_summary() -> String {
    let first = TRAIN_MONTHS[0];
    let last = TRAIN_MONTHS[TRAIN_MONTHS.len() - 1];
    format!(
        "5 years ({} months); expanding training windows M1-M{} through M1-M{}, each tested on the next {} months (OOS M{}-M{} through M{}-M{})",
        TOTAL_MONTHS,
        first,
        last,
        OOS_MONTHS,
        first + 1,
        first + OOS_MONTHS,
        last + 1,
        last + OOS_MONTHS
    )
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

fn write_dataset_section(out: &mut String, result: &SymbolDonchian) {
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
        "| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---|---|---:|---|---|---|---|")
        .expect("writing to String cannot fail");
    for window in &result.windows {
        let train = range_view(candles, window.window.train);
        let oos = range_view(candles, window.window.oos);
        writeln!(
            out,
            "| {} | {}..{} | {} | {} | {}..{} | {} | {} | {} | {} | {} |",
            window_label(window.window.months),
            train.start_index,
            train.end_index,
            train.end_index - train.start_index,
            oos_label(window.window.months),
            oos.start_index,
            oos.end_index,
            oos.end_index - oos.start_index,
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
    result: &SymbolDonchian,
    base: &CostScenario,
    stress: &CostScenario,
    symbols: &[String],
) {
    writeln!(out, "## 2. Experiment").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy | {STRATEGY_VERSION} |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbols | {} ({}) |",
        symbols.join(", "),
        symbols.len()
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Entry | Close > prior Donchian high (N ∈ {:?}) and close > EMA({}) |",
        ENTRY_GRID, result.params.ema_period
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Exit | ATR({}) trailing stop (highest high since entry − multiplier × ATR) or close < EMA({}) at the next open |",
        result.params.atr_period, result.params.ema_period
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Grid | entry N {:?} × Donchian exit N {} × ATR multiplier {:?} × EMA {} × ATR {} = {} configs (Donchian exit N is fixed metadata and is not an active exit in this variant) |",
        ENTRY_GRID,
        DONCHIAN_EXIT,
        ATR_MULTIPLIERS,
        result.params.ema_period,
        result.params.atr_period,
        ENTRY_GRID.len() * ATR_MULTIPLIERS.len()
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Walk-forward | {} |", walk_forward_summary())
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
    writeln!(out, "| Initial fund | {} |", result.params.init_fund)
        .expect("writing to String cannot fail");
    writeln!(out, "| Budget per position | init_fund / 2 |")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Execution assumptions:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Long-only; one position per symbol; no shorts and no pyramiding."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- A signal produced at a candle close fills at the next candle open."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The trailing stop is checked intrabar using the highest high and ATR known at the prior close. A gap below the stop exits at the open; otherwise the exit price is the stop level."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- An EMA cross below is acted on at the next open; the stop takes precedence when both trigger in the same bar."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Costs are charged on the actual entry and exit notional, not on the initial budget."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Selection and pass policy:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Each window ranks all {} configs on base-cost training metrics by the sum of the profit-factor rank (∞ first) and net-expectancy rank. The top {} advance to the holdout.",
        ENTRY_GRID.len() * ATR_MULTIPLIERS.len(),
        TOP_CONFIGS
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- A run requires at least {} OOS trades.",
        result.params.min_oos_trades
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- A run passes only when stress OOS net expectancy > 0, stress OOS PF > 1, stress OOS net PnL > the stress buy-and-hold benchmark, stress OOS trades >= {}, and the parameter-stability rule holds.",
        result.params.min_oos_trades
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Parameter stability requires every available grid neighbor to have positive base-cost train net expectancy. Neighbors share the same exit, EMA, and ATR period and sit within one grid step on both entry N and ATR multiplier; each selected config needs at least one neighbor."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- A symbol passes when any top-3 run in any window passes. The experiment passes only when all {} symbols pass.",
        symbols.len()
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The benchmark buys at the first OOS open and sells at the last OOS close with the same half-budget and cost scenario."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_validation_section(out: &mut String, result: &SymbolDonchian) {
    writeln!(out, "## 3. Validation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Top {} configs per expanding training window, selected on base-cost training metrics. Stability is checked against every available grid neighbor.",
        TOP_CONFIGS
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for window in &result.windows {
        writeln!(
            out,
            "### {} training ({})",
            window_label(window.window.months),
            oos_label(window.window.months)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        if window.top.is_empty() {
            writeln!(out, "No config was evaluated.").expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }
        writeln!(
            out,
            "| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---:|---|---:|---:|---:|---:|---:|---:|---:|---|")
            .expect("writing to String cannot fail");
        for (index, run) in window.top.iter().enumerate() {
            let metrics = &run.train.metrics;
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                index + 1,
                run.config.label(),
                metrics.trades,
                decimal(metrics.net_expectancy),
                profit_factor(metrics.net_profit_factor, metrics.trades),
                decimal(metrics.max_drawdown_pct),
                decimal(metrics.expectancy_stderr),
                decimal(run.train_stress.net_expectancy),
                profit_factor(run.train_stress.net_profit_factor, run.train_stress.trades),
                if run.parameter_stable { "yes" } else { "no" }
            )
            .expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_benchmark_line(out: &mut String, base: &Benchmark, stress: &Benchmark) {
    writeln!(
        out,
        "Buy & hold benchmark (half budget): base net PnL {} ({}%), stress net PnL {} ({}%).",
        decimal(base.net_pnl),
        decimal(base.return_pct),
        decimal(stress.net_pnl),
        decimal(stress.return_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_holdout_rows(out: &mut String, run: &DonchianRun) {
    for (label, metrics, benchmark, success) in [
        ("Base", &run.oos.metrics, run.benchmark_base, false),
        ("Stress", &run.oos_stress, run.benchmark_stress, run.success),
    ] {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            run.config.label(),
            label,
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.net_pnl),
            decimal(benchmark.net_pnl),
            if metrics.net_pnl > benchmark.net_pnl {
                "yes"
            } else {
                "no"
            },
            if run.parameter_stable { "yes" } else { "no" },
            if label == "Stress" {
                if success {
                    "Pass"
                } else {
                    "Fail"
                }
            } else {
                "—"
            }
        )
        .expect("writing to String cannot fail");
    }
}

fn write_holdout_section(out: &mut String, result: &SymbolDonchian) {
    writeln!(out, "## 4. Holdout").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Top {} configs per window on the following {} months, with buy-and-hold comparison.",
        TOP_CONFIGS, OOS_MONTHS
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if result.windows.iter().all(|window| window.top.is_empty()) {
        writeln!(out, "No config was tested out of sample.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    for window in &result.windows {
        writeln!(
            out,
            "### {} holdout ({})",
            oos_label(window.window.months),
            window_label(window.window.months)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        if let Some(first) = window.top.first() {
            write_benchmark_line(out, &first.benchmark_base, &first.benchmark_stress);
        }
        writeln!(
            out,
            "| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---:|---:|---:|---:|---:|---:|---|---|---|")
            .expect("writing to String cannot fail");
        for run in &window.top {
            write_holdout_rows(out, run);
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn tested_labels(windows: &[WindowResult]) -> Vec<String> {
    let mut seen = BTreeMap::new();
    for window in windows {
        for run in &window.top {
            seen.entry(run.config.label()).or_insert(());
        }
    }
    seen.into_keys().collect()
}

fn write_success_matrix(out: &mut String, result: &SymbolDonchian) {
    let labels = tested_labels(&result.windows);
    writeln!(
        out,
        "**Success matrix (window × tested config; stress OOS gates + parameter stability)**"
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
        write!(out, "| {} |", window_label(window.window.months))
            .expect("writing to String cannot fail");
        for label in &labels {
            let cell = window
                .top
                .iter()
                .find(|run| &run.config.label() == label)
                .map(|run| {
                    if run.success {
                        "Pass".to_string()
                    } else if !run.parameter_stable {
                        "Fail (stability)".to_string()
                    } else {
                        "Fail".to_string()
                    }
                })
                .unwrap_or_else(|| "—".to_string());
            write!(out, " {cell} |").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_config_consistency(out: &mut String, result: &SymbolDonchian) {
    #[derive(Default)]
    struct Row {
        selected: usize,
        passed: usize,
        best_stress_expectancy: Option<Decimal>,
    }

    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for window in &result.windows {
        for run in &window.top {
            let row = rows.entry(run.config.label()).or_default();
            row.selected += 1;
            if run.success {
                row.passed += 1;
            }
            let better = row
                .best_stress_expectancy
                .map(|current| run.oos_stress.net_expectancy > current)
                .unwrap_or(true);
            if better {
                row.best_stress_expectancy = Some(run.oos_stress.net_expectancy);
            }
        }
    }
    let mut rows: Vec<(String, Row)> = rows.into_iter().collect();
    rows.sort_by(|a, b| {
        b.1.passed
            .cmp(&a.1.passed)
            .then_with(|| b.1.selected.cmp(&a.1.selected))
            .then_with(|| a.0.cmp(&b.0))
    });

    writeln!(
        out,
        "**Config consistency (across the {} expanding windows)**",
        result.windows.len()
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if rows.is_empty() {
        writeln!(out, "No configs were selected.").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    writeln!(
        out,
        "| Config | Windows selected | Windows passed | Best OOS stress expectancy |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|").expect("writing to String cannot fail");
    for (label, row) in rows {
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            label,
            row.selected,
            row.passed,
            row.best_stress_expectancy
                .map(decimal)
                .unwrap_or_else(|| "—".to_string())
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_benchmark_comparison(out: &mut String, result: &SymbolDonchian) {
    writeln!(out, "**Benchmark comparison by window (stress costs)**")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---|").expect("writing to String cannot fail");
    for window in &result.windows {
        let best = window.top.iter().max_by(|a, b| {
            a.oos_stress
                .net_pnl
                .cmp(&b.oos_stress.net_pnl)
                .then_with(|| {
                    a.oos_stress
                        .net_expectancy
                        .cmp(&b.oos_stress.net_expectancy)
                })
        });
        match best {
            Some(run) => {
                let difference = run.oos_stress.net_pnl - run.benchmark_stress.net_pnl;
                writeln!(
                    out,
                    "| {} | {} | {} | {} | {} | {} |",
                    window_label(window.window.months),
                    run.config.label(),
                    decimal(run.oos_stress.net_pnl),
                    decimal(run.benchmark_stress.net_pnl),
                    decimal(difference),
                    if difference > Decimal::ZERO {
                        "Beat"
                    } else {
                        "Underperform"
                    }
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    out,
                    "| {} | — | — | — | — | — |",
                    window_label(window.window.months)
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_robustness_section(out: &mut String, result: &SymbolDonchian) {
    writeln!(out, "## 5. Robustness").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_success_matrix(out, result);
    write_config_consistency(out, result);
    write_benchmark_comparison(out, result);
}

fn write_decision_section(
    out: &mut String,
    result: &SymbolDonchian,
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
        writeln!(out, "Successful runs: none.").expect("writing to String cannot fail");
    } else {
        writeln!(out, "Successful runs:").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "| Window | Config | OOS trades | Stress expectancy | Stress PF | Stress net PnL | Benchmark stress net PnL |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
        for run in &result.successful_runs {
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} |",
                window_label(run.window_months),
                run.config.label(),
                run.oos_stress.trades,
                decimal(run.oos_stress.net_expectancy),
                profit_factor(run.oos_stress.net_profit_factor, run.oos_stress.trades),
                decimal(run.oos_stress.net_pnl),
                decimal(run.benchmark_stress.net_pnl)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

#[derive(Debug, Clone, Serialize)]
struct RunView {
    config: DonchianConfig,
    window_months: usize,
    train: MetricsView,
    train_stress: MetricsView,
    oos: MetricsView,
    oos_stress: MetricsView,
    benchmark_base: Benchmark,
    benchmark_stress: Benchmark,
    parameter_stable: bool,
    success: bool,
}

impl RunView {
    fn from_run(run: &DonchianRun) -> Self {
        Self {
            config: run.config.clone(),
            window_months: run.window_months,
            train: MetricsView::from_metrics(&run.train.metrics),
            train_stress: MetricsView::from_metrics(&run.train_stress),
            oos: MetricsView::from_metrics(&run.oos.metrics),
            oos_stress: MetricsView::from_metrics(&run.oos_stress),
            benchmark_base: run.benchmark_base,
            benchmark_stress: run.benchmark_stress,
            parameter_stable: run.parameter_stable,
            success: run.success,
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
    init_fund: i64,
    ema_period: usize,
    atr_period: usize,
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
    result: &SymbolDonchian,
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
        init_fund: result.params.init_fund,
        ema_period: result.params.ema_period,
        atr_period: result.params.atr_period,
        base_costs: CostView::from_scenario(base),
        stress_costs: CostView::from_scenario(stress),
        windows: result
            .windows
            .iter()
            .map(|window| WindowView {
                months: window.window.months,
                train: range_view(candles, window.window.train),
                oos: range_view(candles, window.window.oos),
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

#[allow(clippy::too_many_arguments)]
pub fn write_symbol_report(
    dir: &Path,
    result: &SymbolDonchian,
    base: &CostScenario,
    stress: &CostScenario,
    symbols: &[String],
    overall: Decision,
) -> Result<(), String> {
    ensure_dir(dir)?;

    let mut out = String::new();
    writeln!(
        out,
        "# Donchian Trend-Following — {} ({}, 5 years)",
        result.symbol, result.dataset.provenance.requested.timeframe
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Research artifact for `{STRATEGY_VERSION}`. This is not a profitability claim."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_dataset_section(&mut out, result);
    write_experiment_section(&mut out, result, base, stress, symbols);
    write_validation_section(&mut out, result);
    write_holdout_section(&mut out, result);
    write_robustness_section(&mut out, result);
    write_decision_section(&mut out, result, overall, symbols.len());

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;

    let results = symbol_results(result, base, stress, overall, symbols.len());
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
    generated_at: String,
    expected_symbols: usize,
    symbols: Vec<SymbolSummary>,
    overall_verdict: Decision,
    data_issues: Vec<String>,
}

fn combined_results(
    results: &[SymbolDonchian],
    expected_symbols: usize,
    errors: &[String],
    generated_at: &str,
) -> CombinedResults {
    let verdicts: Vec<Decision> = results
        .iter()
        .map(|result| result.decision.verdict)
        .collect();
    let overall = if verdicts.is_empty()
        || verdicts.len() != expected_symbols
        || verdicts.iter().any(|verdict| *verdict != Decision::Pass)
    {
        Decision::Fail
    } else {
        Decision::Pass
    };
    let timeframe = results
        .first()
        .map(|result| result.dataset.provenance.requested.timeframe.clone())
        .unwrap_or_else(|| "unknown".to_string());
    CombinedResults {
        strategy_version: STRATEGY_VERSION.to_string(),
        timeframe,
        generated_at: generated_at.to_string(),
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

pub fn write_combined_index(
    dir: &Path,
    results: &[SymbolDonchian],
    errors: &[String],
    expected_symbols: usize,
    generated_at: &str,
) -> Result<(), String> {
    ensure_dir(dir)?;
    let combined = combined_results(results, expected_symbols, errors, generated_at);
    let timeframe = combined.timeframe.clone();

    let mut out = String::new();
    writeln!(out, "# Donchian Trend-Following — 5 Years, {timeframe}")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let min_oos_trades = results
        .first()
        .map(|result| result.params.min_oos_trades)
        .unwrap_or(10);
    writeln!(
        out,
        "Data: {}. A symbol passes when any top-3 config in any window clears the stress OOS gates (net expectancy > 0, PF > 1, net PnL above buy-and-hold, at least {} OOS trades) and the parameter-stability rule; the overall experiment passes only when all {} symbols pass.",
        walk_forward_summary(),
        min_oos_trades,
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
    writeln!(out, "Generated at: {generated_at}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Candles | Winner | Window | OOS stress trades | OOS stress expectancy | OOS stress PF | OOS stress net PnL | Benchmark stress net PnL | Successful configs | Verdict |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---|---|---:|---:|---:|---:|---:|---:|---|")
        .expect("writing to String cannot fail");
    for result in results {
        let winner_label = result
            .winner
            .as_ref()
            .map(|run| run.config.label())
            .unwrap_or_else(|| "none".to_string());
        let winner_window = result
            .winner
            .as_ref()
            .map(|run| window_label(run.window_months))
            .unwrap_or_else(|| "—".to_string());
        let oos = result.winner.as_ref().map(|run| &run.oos_stress);
        let benchmark = result
            .winner
            .as_ref()
            .map(|run| run.benchmark_stress.net_pnl)
            .map(decimal)
            .unwrap_or_else(|| "—".to_string());
        writeln!(
            out,
            "| [{}]({}/report.md) | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
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
            oos.map(|metrics| decimal(metrics.net_pnl))
                .unwrap_or_else(|| "—".to_string()),
            benchmark,
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
    writeln!(out, "Per-symbol artifacts: `report.md` and `results.json`.")
        .expect("writing to String cannot fail");
    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())?;

    let json = serde_json::to_string_pretty(&combined).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}
