use crate::harness::experiment::{
    neighbor_metrics, regime_metrics, side_metrics, CandidateEvaluation, ExperimentParams,
    RegimeMetrics, SideMetrics, SliceOutcome, Split, SymbolExperiment, TopCandidate,
    STRATEGY_VERSION,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::partition::Slice;
use crate::harness::sweep::sampling::Candidate;
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;
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
struct SplitView {
    dev_m1_m4: RangeView,
    first_oos_m5: RangeView,
    extended_oos_m6_m10: RangeView,
    excluded_m11_m12: RangeView,
}

fn split_view(split: Split, candles: &[Candle]) -> SplitView {
    SplitView {
        dev_m1_m4: range_view(candles, split.dev),
        first_oos_m5: range_view(candles, split.first_oos),
        extended_oos_m6_m10: range_view(candles, split.extended_oos),
        excluded_m11_m12: range_view(
            candles,
            Slice {
                start: split.extended_oos.end,
                end: candles.len(),
            },
        ),
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

fn write_dataset_section(out: &mut String, experiment: &SymbolExperiment, split: &SplitView) {
    let provenance = &experiment.dataset.provenance;
    let candles = &experiment.dataset.candles;
    writeln!(out, "## 1. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", experiment.symbol).expect("writing to String cannot fail");
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

    writeln!(out, "| Split | Candle range | First open | Last open |")
        .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|").expect("writing to String cannot fail");
    let rows = [
        ("M1-M4 development", &split.dev_m1_m4),
        ("M5 first out-of-sample", &split.first_oos_m5),
        ("M6-M10 extended OOS log", &split.extended_oos_m6_m10),
        ("M11-M12 excluded", &split.excluded_m11_m12),
    ];
    for (label, view) in rows {
        writeln!(
            out,
            "| {label} | {}..{} | {} | {} |",
            view.start_index, view.end_index, view.start_time, view.end_time
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_experiment_section(
    out: &mut String,
    experiment: &SymbolExperiment,
    params: &ExperimentParams,
    base: &CostScenario,
    stress: &CostScenario,
) {
    writeln!(out, "## 2. Experiment").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy version | {STRATEGY_VERSION} |")
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
    writeln!(out, "| Frozen top configs | {} |", params.top_n)
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
        "- Ranking uses base costs on M1-M4; stress costs are reported for context."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Development policy:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Only the max drawdown gate is applied: max drawdown <= {}%. There is no minimum trade-count gate.",
        decimal(params.max_dd_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Candidates are ranked by profit factor descending (infinite first, then net expectancy and label) and separately by net expectancy descending (then profit factor and label)."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Combined score = PF rank + expectancy rank; lower is better. The top {} are frozen before M5 is read.",
        params.top_n
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- M5 pass rule: net expectancy > 0 and PF > 1 with no trade-count gate. The symbol passes if any frozen top config passes M5; M6-M10 is logged only."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    if experiment.top.is_empty() {
        writeln!(
            out,
            "No candidate passed the max drawdown gate, so no top config was frozen."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }

    writeln!(
        out,
        "| Top | Config | Dev trades | Dev net expectancy | Dev PF | Dev max DD % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for (index, candidate) in experiment.top.iter().enumerate() {
        let metrics = dev_metrics(experiment, &candidate.candidate.label);
        writeln!(
            out,
            "| top-{} | {} | {} | {} | {} | {} |",
            index + 1,
            config_summary(&candidate.candidate),
            metrics
                .map(|metrics| metrics.trades.to_string())
                .unwrap_or_else(|| "—".to_string()),
            metrics
                .map(|metrics| decimal(metrics.net_expectancy))
                .unwrap_or_else(|| "—".to_string()),
            metrics
                .map(|metrics| profit_factor(metrics.net_profit_factor, metrics.trades))
                .unwrap_or_else(|| "—".to_string()),
            metrics
                .map(|metrics| decimal(metrics.max_drawdown_pct))
                .unwrap_or_else(|| "—".to_string())
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn top_rank(experiment: &SymbolExperiment, label: &str) -> Option<usize> {
    experiment
        .top
        .iter()
        .position(|candidate| candidate.candidate.label == label)
        .map(|index| index + 1)
}

fn dev_metrics<'a>(experiment: &'a SymbolExperiment, label: &str) -> Option<&'a PartitionMetrics> {
    experiment
        .candidates
        .iter()
        .find(|evaluation| evaluation.candidate.label == label)
        .map(|evaluation| &evaluation.dev.metrics)
}

fn primary_candidate(experiment: &SymbolExperiment) -> Option<&TopCandidate> {
    experiment
        .top
        .iter()
        .find(|candidate| candidate.m5_pass)
        .or_else(|| experiment.top.first())
}

fn config_summary(candidate: &Candidate) -> String {
    format!(
        "{} (period {}, long {}, short {})",
        candidate.label,
        candidate.rsi_period,
        candidate.rsi_long_threshold,
        candidate.rsi_short_threshold
    )
}

fn write_validation_section(out: &mut String, experiment: &SymbolExperiment) {
    writeln!(out, "## 3. Validation — M1-M4 development").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "All 24 candidates, base costs. Frozen configs are marked **top-1**, **top-2**, and **top-3**."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Label | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Top |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---:|---:|---|").expect("writing to String cannot fail");

    let mut candidates: Vec<&CandidateEvaluation> = experiment
        .candidates
        .iter()
        .filter(|evaluation| top_rank(experiment, &evaluation.candidate.label).is_none())
        .collect();
    candidates.sort_by(|a, b| {
        b.dev
            .metrics
            .net_expectancy
            .cmp(&a.dev.metrics.net_expectancy)
            .then_with(|| a.candidate.label.cmp(&b.candidate.label))
    });
    let mut ordered: Vec<&CandidateEvaluation> = experiment
        .top
        .iter()
        .filter_map(|top| {
            experiment
                .candidates
                .iter()
                .find(|evaluation| evaluation.candidate.label == top.candidate.label)
        })
        .collect();
    ordered.append(&mut candidates);

    for evaluation in ordered {
        let candidate = &evaluation.candidate;
        let metrics = &evaluation.dev.metrics;
        let mark = top_rank(experiment, &candidate.label)
            .map(|rank| format!("**top-{rank}**"))
            .unwrap_or_default();
        writeln!(
            out,
            "| {} | period {} / long {} / short {} | {} | {} | {} | {} | {} | {} |",
            candidate.label,
            candidate.rsi_period,
            candidate.rsi_long_threshold,
            candidate.rsi_short_threshold,
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            mark
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_holdout_section(out: &mut String, experiment: &SymbolExperiment) {
    writeln!(out, "## 4. Holdout").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if experiment.top.is_empty() {
        writeln!(out, "No candidate was frozen, so no holdout was evaluated.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    writeln!(
        out,
        "M5 is the decision partition; M6-M10 is logged for every frozen config and never changes the decision."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    for (index, candidate) in experiment.top.iter().enumerate() {
        let status = if candidate.m5_pass { "PASS" } else { "FAIL" };
        writeln!(
            out,
            "### Top-{}: {} — M5 **{}**",
            index + 1,
            config_summary(&candidate.candidate),
            status
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");

        writeln!(out, "**M5 — first out-of-sample (decision partition)**")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let base = MetricsView::from_metrics(&candidate.first_oos.metrics);
        let stress = MetricsView::from_metrics(&candidate.first_oos_stress);
        write_holdout_rows(out, &base, Some(&stress));

        writeln!(
            out,
            "M5 result: **{}** (net expectancy > 0 and PF > 1; no trade-count gate).",
            status
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");

        writeln!(out, "**M6-M10 — extended OOS log (context only)**")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let base = MetricsView::from_metrics(&candidate.extended_oos.metrics);
        let stress = MetricsView::from_metrics(&candidate.extended_oos_stress);
        write_holdout_rows(out, &base, Some(&stress));
    }
    writeln!(
        out,
        "M6-M10 is logged for context and does not change the decision."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_holdout_rows(out: &mut String, base: &MetricsView, stress: Option<&MetricsView>) {
    writeln!(
        out,
        "| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for (label, metrics) in [("Base", Some(base)), ("Stress", stress)] {
        let Some(metrics) = metrics else {
            continue;
        };
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} | {} |",
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            decimal(metrics.closed_net_pnl),
            decimal(metrics.open_net_pnl)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_robustness_section(out: &mut String, experiment: &SymbolExperiment, base: &CostScenario) {
    writeln!(out, "## 5. Robustness").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let Some(chosen) = primary_candidate(experiment) else {
        writeln!(out, "No candidate was frozen; robustness is unavailable.")
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    };
    writeln!(
        out,
        "Using {} (the first top config that passes M5, else top-1).",
        config_summary(&chosen.candidate)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let first = &chosen.first_oos;

    let last_close = first
        .candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or(Decimal::ZERO);
    let long = side_metrics(&first.outcome, base, Direction::Long, last_close);
    let short = side_metrics(&first.outcome, base, Direction::Short, last_close);
    write_side_table(
        out,
        "Long vs short (M5, base costs)",
        &[("Long", long), ("Short", short)],
    );

    if experiment.btc_regime.is_empty() {
        writeln!(out, "**BTC regime (M5, base costs)**").expect("writing to String cannot fail");
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
            &first.outcome,
            base,
            &first.candles,
            &experiment.btc_regime,
            last_close,
        );
        write_side_table(
            out,
            "BTC regime (M5, base costs; bull = BTC close > 200-period SMA at entry)",
            &[("Bull", bull), ("Bear", bear), ("Unknown", unknown)],
        );
    }

    let (min, median, max, count) = neighbor_metrics(&chosen.candidate, &experiment.candidates);
    writeln!(out, "**Neighboring configs (M1-M4, base costs)**")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Same RSI period and long/short thresholds within ±5 points, including the chosen config."
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
}

fn write_decision_section(out: &mut String, experiment: &SymbolExperiment) {
    writeln!(out, "## 6. Decision").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "**Verdict: {}**", experiment.decision.verdict.as_str())
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for reason in &experiment.decision.reasons {
        writeln!(out, "- {reason}").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The verdict passes if any frozen top config passes M5. M6-M10 is included above for transparency."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

#[derive(Debug, Clone, Serialize)]
struct ValidationRow {
    label: String,
    origin: String,
    rsi_period: usize,
    rsi_long_threshold: Decimal,
    rsi_short_threshold: Decimal,
    top_rank: Option<usize>,
    dev: MetricsView,
    dev_stress: MetricsView,
}

#[derive(Debug, Clone, Serialize)]
struct HoldoutView {
    partition: String,
    base: MetricsView,
    stress: Option<MetricsView>,
}

#[derive(Debug, Clone, Serialize)]
struct TopView {
    rank: usize,
    candidate: Candidate,
    m5_pass: bool,
    first_oos: HoldoutView,
    extended_oos: HoldoutView,
}

#[derive(Debug, Clone, Serialize)]
struct NeighborView {
    min: Decimal,
    median: Decimal,
    max: Decimal,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RobustnessView {
    long: SideMetrics,
    short: SideMetrics,
    regime: Option<RegimeMetrics>,
    neighbors: Option<NeighborView>,
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
    params: ExperimentParams,
    base_costs: CostView,
    stress_costs: CostView,
    split: SplitView,
    selected: Option<Candidate>,
    top: Vec<TopView>,
    validation: Vec<ValidationRow>,
    first_oos: Option<HoldoutView>,
    extended_oos: Option<HoldoutView>,
    robustness: RobustnessView,
    decision: crate::harness::experiment::DecisionReport,
}

fn side_robustness(experiment: &SymbolExperiment, base: &CostScenario) -> RobustnessView {
    let Some(chosen) = primary_candidate(experiment) else {
        return RobustnessView {
            long: SideMetrics::default(),
            short: SideMetrics::default(),
            regime: None,
            neighbors: None,
        };
    };
    let first = &chosen.first_oos;
    let last_close = first
        .candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or(Decimal::ZERO);
    let long = side_metrics(&first.outcome, base, Direction::Long, last_close);
    let short = side_metrics(&first.outcome, base, Direction::Short, last_close);
    let regime = if experiment.btc_regime.is_empty() {
        None
    } else {
        Some(regime_metrics(
            &first.outcome,
            base,
            &first.candles,
            &experiment.btc_regime,
            last_close,
        ))
    };
    let (min, median, max, count) = neighbor_metrics(&chosen.candidate, &experiment.candidates);
    let neighbors = Some(NeighborView {
        min,
        median,
        max,
        count,
    });
    RobustnessView {
        long,
        short,
        regime,
        neighbors,
    }
}

fn holdout_view(
    partition: &str,
    base: Option<&SliceOutcome>,
    stress: Option<&PartitionMetrics>,
) -> Option<HoldoutView> {
    base.map(|base| HoldoutView {
        partition: partition.to_string(),
        base: MetricsView::from_metrics(&base.metrics),
        stress: stress.map(MetricsView::from_metrics),
    })
}

fn symbol_results(
    experiment: &SymbolExperiment,
    params: &ExperimentParams,
    base: &CostScenario,
    stress: &CostScenario,
) -> SymbolResults {
    let split = split_view(experiment.split, &experiment.dataset.candles);
    let validation = experiment
        .candidates
        .iter()
        .map(|evaluation| ValidationRow {
            label: evaluation.candidate.label.clone(),
            origin: evaluation.candidate.origin.clone(),
            rsi_period: evaluation.candidate.rsi_period,
            rsi_long_threshold: evaluation.candidate.rsi_long_threshold,
            rsi_short_threshold: evaluation.candidate.rsi_short_threshold,
            top_rank: top_rank(experiment, &evaluation.candidate.label),
            dev: MetricsView::from_metrics(&evaluation.dev.metrics),
            dev_stress: MetricsView::from_metrics(&evaluation.stress),
        })
        .collect();
    let top = experiment
        .top
        .iter()
        .enumerate()
        .map(|(index, candidate)| TopView {
            rank: index + 1,
            candidate: candidate.candidate.clone(),
            m5_pass: candidate.m5_pass,
            first_oos: holdout_view(
                "M5",
                Some(&candidate.first_oos),
                Some(&candidate.first_oos_stress),
            )
            .expect("top candidate has an M5 evaluation"),
            extended_oos: holdout_view(
                "M6-M10",
                Some(&candidate.extended_oos),
                Some(&candidate.extended_oos_stress),
            )
            .expect("top candidate has an M6-M10 evaluation"),
        })
        .collect();
    SymbolResults {
        symbol: experiment.symbol.clone(),
        timeframe: experiment.dataset.provenance.requested.timeframe.clone(),
        strategy_version: STRATEGY_VERSION.to_string(),
        first_open_time: candle_time(&experiment.dataset.candles, 0),
        last_open_time: candle_time(
            &experiment.dataset.candles,
            experiment.dataset.candles.len().saturating_sub(1),
        ),
        candles: experiment.dataset.candles.len(),
        content_sha256: experiment.dataset.provenance.content_sha256.clone(),
        params: params.clone(),
        base_costs: CostView::from_scenario(base),
        stress_costs: CostView::from_scenario(stress),
        split,
        selected: experiment.selected.clone(),
        top,
        validation,
        first_oos: holdout_view(
            "M5",
            experiment.first_oos.as_ref(),
            experiment.first_oos_stress.as_ref(),
        ),
        extended_oos: holdout_view(
            "M6-M10",
            experiment.extended_oos.as_ref(),
            experiment.extended_oos_stress.as_ref(),
        ),
        robustness: side_robustness(experiment, base),
        decision: experiment.decision.clone(),
    }
}

const LEADERBOARD_COLUMNS: [&str; 14] = [
    "label",
    "origin",
    "rsi_period",
    "rsi_long_threshold",
    "rsi_short_threshold",
    "dev_trades",
    "dev_net_expectancy",
    "dev_net_profit_factor",
    "dev_max_drawdown_pct",
    "dev_expectancy_stderr",
    "dev_closed_net_pnl",
    "dev_net_pnl",
    "dev_stress_net_expectancy",
    "top_rank",
];

fn write_leaderboard(path: &Path, experiment: &SymbolExperiment) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(LEADERBOARD_COLUMNS)
        .map_err(|error| error.to_string())?;
    let mut candidates: Vec<&CandidateEvaluation> = experiment.candidates.iter().collect();
    candidates.sort_by(|a, b| {
        b.dev
            .metrics
            .net_expectancy
            .cmp(&a.dev.metrics.net_expectancy)
            .then_with(|| a.candidate.label.cmp(&b.candidate.label))
    });
    for evaluation in candidates {
        let candidate = &evaluation.candidate;
        let metrics = &evaluation.dev.metrics;
        let record = [
            candidate.label.clone(),
            candidate.origin.clone(),
            candidate.rsi_period.to_string(),
            candidate.rsi_long_threshold.to_string(),
            candidate.rsi_short_threshold.to_string(),
            metrics.trades.to_string(),
            metrics.net_expectancy.to_string(),
            metrics
                .net_profit_factor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            metrics.max_drawdown_pct.to_string(),
            metrics.expectancy_stderr.to_string(),
            metrics.closed_net_pnl.to_string(),
            metrics.net_pnl.to_string(),
            evaluation.stress.net_expectancy.to_string(),
            top_rank(experiment, &candidate.label)
                .map(|rank| format!("top-{rank}"))
                .unwrap_or_default(),
        ];
        writer
            .write_record(record)
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

/// Writes `report.md`, `leaderboard.csv`, and `results.json` for one symbol.
pub fn write_symbol_report(
    dir: &Path,
    experiment: &SymbolExperiment,
    params: &ExperimentParams,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<(), String> {
    ensure_dir(dir)?;
    let split = split_view(experiment.split, &experiment.dataset.candles);

    let mut out = String::new();
    writeln!(
        out,
        "# RSI Per-Symbol Tuning — {} (4hour, 1 year)",
        experiment.symbol
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Research artifact for `{STRATEGY_VERSION}`. This is not a profitability claim."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_dataset_section(&mut out, experiment, &split);
    write_experiment_section(&mut out, experiment, params, base, stress);
    write_validation_section(&mut out, experiment);
    write_holdout_section(&mut out, experiment);
    write_robustness_section(&mut out, experiment, base);
    write_decision_section(&mut out, experiment);

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;
    write_leaderboard(&dir.join("leaderboard.csv"), experiment)?;

    let results = symbol_results(experiment, params, base, stress);
    let json = serde_json::to_string_pretty(&results).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}

/// Writes `README.md` at the root of a multi-symbol run.
pub fn write_combined_index(dir: &Path, experiments: &[SymbolExperiment]) -> Result<(), String> {
    ensure_dir(dir)?;
    let mut out = String::new();
    writeln!(out, "# RSI Per-Symbol Tuning — 1 Year, 4hour")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Each symbol freezes the top three configs by combined PF/expectancy rank on M1-M4, then judges M5 only. A config passes when M5 net expectancy > 0 and PF > 1; the symbol passes if any top config passes. M6-M10 is logged for context and never changes the decision."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Top | Config | M1-M4 trades | M1-M4 expectancy | M5 trades | M5 expectancy | M5 PF | M6-M10 expectancy | M5 result | Symbol verdict |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---:|---:|---:|---:|---:|---:|---|---|")
        .expect("writing to String cannot fail");
    for experiment in experiments {
        if experiment.top.is_empty() {
            writeln!(
                out,
                "| [{}]({}/report.md) | — | no config (DD gate) | — | — | — | — | — | — | — | {} |",
                experiment.symbol,
                experiment.symbol,
                experiment.decision.verdict.as_str()
            )
            .expect("writing to String cannot fail");
            continue;
        }
        for (index, candidate) in experiment.top.iter().enumerate() {
            let dev = dev_metrics(experiment, &candidate.candidate.label);
            let first = &candidate.first_oos.metrics;
            let extended = &candidate.extended_oos.metrics;
            writeln!(
                out,
                "| [{}]({}/report.md) | top-{} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                experiment.symbol,
                experiment.symbol,
                index + 1,
                candidate.candidate.label,
                dev.map(|metrics| metrics.trades.to_string())
                    .unwrap_or_else(|| "—".to_string()),
                dev.map(|metrics| decimal(metrics.net_expectancy))
                    .unwrap_or_else(|| "—".to_string()),
                first.trades,
                decimal(first.net_expectancy),
                profit_factor(first.net_profit_factor, first.trades),
                decimal(extended.net_expectancy),
                if candidate.m5_pass { "PASS" } else { "FAIL" },
                experiment.decision.verdict.as_str()
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`."
    )
    .expect("writing to String cannot fail");
    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::sweep::partition::Slice;

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
    fn split_view_marks_m11_m12_as_excluded() {
        let candles: Vec<Candle> = (0..1200).map(|index| candle(index as i64)).collect();
        let view = split_view(crate::harness::experiment::split_one_year(1200), &candles);
        assert_eq!(view.dev_m1_m4.end_index, 400);
        assert_eq!(view.first_oos_m5.end_index, 500);
        assert_eq!(view.extended_oos_m6_m10.end_index, 1000);
        assert_eq!(view.excluded_m11_m12.start_index, 1000);
        assert_eq!(view.excluded_m11_m12.end_index, 1200);
    }
}
