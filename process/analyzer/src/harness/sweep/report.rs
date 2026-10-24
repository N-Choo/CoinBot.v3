use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::partition::{chronological_splits, Slice};
use crate::harness::sweep::ranking::{CandidateOutcome, Selection, Thresholds};
use crate::harness::sweep::sampling::Candidate;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DatasetSummary {
    pub symbol: String,
    pub timeframe: String,
    pub requested_first_ms: i64,
    pub requested_last_ms: i64,
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
    pub requested_candles: usize,
    pub expected_candles: usize,
    pub missing_candles: usize,
    pub duplicates_dropped: usize,
    pub pages: usize,
    pub fetched_at_ms: Option<i64>,
    pub settle_ms: i64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidatePlan {
    pub seed: u64,
    pub candidates: Vec<Candidate>,
}

/// A half-open candle-index range (`start` inclusive, `end` exclusive) with
/// ISO timestamps derived from the first and last candle in the range.
#[derive(Debug, Clone, Serialize)]
pub struct SplitBoundary {
    pub start_index: usize,
    pub end_index: usize,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FoldSummary {
    pub fold: usize,
    pub train: SplitBoundary,
    pub validation: SplitBoundary,
}

/// Chronological split boundaries for one loaded dataset. Timestamps are the
/// open times of the boundary candles, so they can be checked independently of
/// the index arithmetic.
#[derive(Debug, Clone, Serialize)]
pub struct SplitSummary {
    pub symbol: String,
    pub timeframe: String,
    pub candles: usize,
    pub train: SplitBoundary,
    pub validation: SplitBoundary,
    pub holdout: SplitBoundary,
    pub folds: Vec<FoldSummary>,
}

impl SplitSummary {
    pub fn from_candles(symbol: &str, timeframe: &str, candles: &[Candle]) -> Self {
        let splits = chronological_splits(candles.len());
        let boundary = |slice: Slice| SplitBoundary {
            start_index: slice.start,
            end_index: slice.end,
            start_time: iso_open_time(candles, slice.start),
            end_time: iso_open_time(candles, slice.end.saturating_sub(1)),
        };
        let folds = splits
            .folds
            .iter()
            .enumerate()
            .map(|(index, fold)| FoldSummary {
                fold: index + 1,
                train: boundary(fold.train),
                validation: boundary(fold.val),
            })
            .collect();
        Self {
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            candles: candles.len(),
            train: boundary(splits.train),
            validation: boundary(splits.validation),
            holdout: boundary(splits.holdout),
            folds,
        }
    }
}

fn iso_open_time(candles: &[Candle], index: usize) -> String {
    candles
        .get(index)
        .and_then(|candle| {
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(candle.open_time_ms)
        })
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Full resolved baseline configuration.
#[derive(Debug, Clone, Serialize)]
pub struct BaselineConfig {
    pub rsi_period: usize,
    pub rsi_long_threshold: Decimal,
    pub rsi_short_threshold: Decimal,
    pub window: usize,
}

impl BaselineConfig {
    pub fn from_candidate(candidate: &Candidate) -> Self {
        let config = candidate.to_config();
        Self {
            rsi_period: config.rsi_period,
            rsi_long_threshold: config.rsi_long_threshold,
            rsi_short_threshold: config.rsi_short_threshold,
            window: config.window,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SweepResults {
    pub generated_at: String,
    pub as_of_ms: i64,
    pub months: u32,
    pub seed: u64,
    pub symbols: Vec<String>,
    pub timeframes: Vec<String>,
    pub fee_bps: u32,
    pub slip_bps: u32,
    pub stress_fee_bps: u32,
    pub stress_slip_bps: u32,
    pub init_fund: i64,
    pub datasets: Vec<DatasetSummary>,
    /// Chronological split boundaries for every loaded symbol/timeframe.
    pub splits: Vec<SplitSummary>,
    /// Baseline candidate pinned at the start of the study.
    pub baseline: Candidate,
    /// Human-readable description of where the baseline came from.
    pub baseline_source: String,
    /// Fully resolved baseline configuration.
    pub baseline_config: BaselineConfig,
    /// timeframe -> candidate outcomes (sorted by label)
    pub outcomes: BTreeMap<String, Vec<CandidateOutcome>>,
    /// timeframe -> selection
    pub selections: BTreeMap<String, Selection>,
    /// non-fatal fetch/run errors, recorded rather than hidden
    pub errors: Vec<String>,
    /// Selection thresholds applied to every timeframe.
    pub thresholds: Thresholds,
    /// timeframe -> seed and the complete candidate list (every attempted config).
    pub candidate_plan: BTreeMap<String, CandidatePlan>,
}

const LEADERBOARD_COLUMNS: [&str; 25] = [
    "timeframe",
    "symbol_or_pooled",
    "label",
    "origin",
    "rsi_period",
    "rsi_long_threshold",
    "rsi_short_threshold",
    "partition",
    "trades",
    "open_at_end",
    "net_expectancy",
    "net_expectancy_r",
    "net_profit_factor",
    "win_rate",
    "max_drawdown",
    "max_drawdown_pct",
    "closed_net_pnl",
    "open_net_pnl",
    "net_pnl",
    "other_exits",
    "exposure_candles",
    "time_in_market_pct",
    "expectancy_stderr",
    "stress_validation_net_pnl",
    "stress_validation_expectancy",
];

/// Partitions every candidate is scored on. The holdout is scored only for
/// frozen candidates and is therefore optional in the artifacts.
const REQUIRED_BASE_PARTITIONS: [&str; 5] =
    ["train", "validation", "fold1_val", "fold2_val", "fold3_val"];

const HOLDOUT_PARTITION: &str = "holdout";

/// Placeholder for a partition that was not evaluated (for example a holdout
/// on a non-frozen candidate).
const MISSING_METRIC: &str = "—";

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Returns a copy with candidates, per-symbol metrics, and selection notes in deterministic order.
fn sorted_results(results: &SweepResults) -> SweepResults {
    let mut sorted = results.clone();
    for outcomes in sorted.outcomes.values_mut() {
        outcomes.sort_by(|a, b| a.label.cmp(&b.label));
        for outcome in outcomes.iter_mut() {
            outcome.per_symbol_validation.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }
    for selection in sorted.selections.values_mut() {
        selection.notes.sort();
    }
    sorted
}

/// Reports every required base partition that an outcome is missing, so CSV
/// output never silently drops rows. The holdout is optional (frozen only).
fn missing_base_partitions(results: &SweepResults) -> Vec<String> {
    let mut missing = Vec::new();
    for (timeframe, outcomes) in &results.outcomes {
        for outcome in outcomes {
            for partition in REQUIRED_BASE_PARTITIONS {
                if !outcome.base.contains_key(partition) {
                    missing.push(format!("{timeframe}/{}: {partition}", outcome.label));
                }
            }
        }
    }
    missing
}

/// Writes one row per timeframe × candidate × symbol-or-pooled × required base partition.
///
/// The holdout row is emitted only when it was evaluated (frozen candidates). Per-symbol rows
/// carry the validation partition only, because that is the per-symbol partition recorded by
/// [`CandidateOutcome`]. Stress columns are populated for the pooled validation row; they are
/// empty where no stress metrics were recorded.
pub fn write_leaderboard_csv(path: &Path, results: &SweepResults) -> Result<(), String> {
    let sorted = sorted_results(results);
    let results = &sorted;
    let missing = missing_base_partitions(results);
    if !missing.is_empty() {
        return Err(format!(
            "cannot write leaderboard; missing base partitions: {}",
            missing.join(", ")
        ));
    }

    ensure_parent(path)?;
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(LEADERBOARD_COLUMNS)
        .map_err(|error| error.to_string())?;

    for (timeframe, outcomes) in &results.outcomes {
        for outcome in outcomes {
            for partition in REQUIRED_BASE_PARTITIONS {
                // The pooled validation row is `base["validation"]`, the same partition used for
                // selection. Missing partitions are rejected above rather than silently skipped.
                let metrics = outcome.base.get(partition).ok_or_else(|| {
                    format!(
                        "cannot write leaderboard; {timeframe}/{} is missing {partition}",
                        outcome.label
                    )
                })?;
                let stress = if partition == "validation" {
                    outcome.stress.get("validation")
                } else {
                    None
                };
                write_leaderboard_row(
                    &mut writer,
                    timeframe,
                    "pooled",
                    outcome,
                    metrics,
                    partition,
                    stress,
                )?;
            }

            // The holdout is scored only for frozen candidates; when absent, no
            // row is written rather than failing the whole artifact.
            if let Some(metrics) = outcome.base.get(HOLDOUT_PARTITION) {
                write_leaderboard_row(
                    &mut writer,
                    timeframe,
                    "pooled",
                    outcome,
                    metrics,
                    HOLDOUT_PARTITION,
                    None,
                )?;
            }

            for (symbol, metrics) in &outcome.per_symbol_validation {
                write_leaderboard_row(
                    &mut writer,
                    timeframe,
                    symbol,
                    outcome,
                    metrics,
                    "validation",
                    None,
                )?;
            }
        }
    }

    writer.flush().map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn write_leaderboard_row<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    timeframe: &str,
    symbol_or_pooled: &str,
    outcome: &CandidateOutcome,
    metrics: &PartitionMetrics,
    partition: &str,
    stress_validation: Option<&PartitionMetrics>,
) -> Result<(), String> {
    let candidate = &outcome.candidate;
    let stress_net_pnl = stress_validation
        .map(|stress| stress.net_pnl.to_string())
        .unwrap_or_default();
    let stress_expectancy = stress_validation
        .map(|stress| stress.net_expectancy.to_string())
        .unwrap_or_default();
    let record = [
        timeframe.to_string(),
        symbol_or_pooled.to_string(),
        outcome.label.clone(),
        outcome.origin.clone(),
        candidate.rsi_period.to_string(),
        candidate.rsi_long_threshold.to_string(),
        candidate.rsi_short_threshold.to_string(),
        partition.to_string(),
        metrics.trades.to_string(),
        metrics.open_at_end.to_string(),
        metrics.net_expectancy.to_string(),
        metrics.net_expectancy_r.to_string(),
        metrics
            .net_profit_factor
            .map(|value| value.to_string())
            .unwrap_or_default(),
        metrics.win_rate.to_string(),
        metrics.max_drawdown.to_string(),
        metrics.max_drawdown_pct.to_string(),
        metrics.closed_net_pnl.to_string(),
        metrics.open_net_pnl.to_string(),
        metrics.net_pnl.to_string(),
        metrics.other_exits.to_string(),
        metrics.exposure_candles.to_string(),
        metrics.time_in_market_pct.to_string(),
        metrics.expectancy_stderr.to_string(),
        stress_net_pnl,
        stress_expectancy,
    ];
    writer
        .write_record(record)
        .map_err(|error| error.to_string())
}

/// Serializes the full results document, including thresholds and the per-timeframe candidate plan.
pub fn write_results_json(path: &Path, results: &SweepResults) -> Result<(), String> {
    ensure_parent(path)?;
    let sorted = sorted_results(results);
    let json = serde_json::to_string_pretty(&sorted).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

pub fn write_markdown(path: &Path, results: &SweepResults) -> Result<(), String> {
    ensure_parent(path)?;
    let sorted = sorted_results(results);
    let results = &sorted;
    let mut out = String::new();
    writeln!(out, "# Analyzer Hyperparameter Screening Report")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Provisional screening results. This report does not claim profitability."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    write_metadata(&mut out, results);
    write_baseline_config(&mut out, results);
    write_data_quality(&mut out, results);
    write_splits(&mut out, results);
    write_baseline_vs_candidates(&mut out, results);
    write_configurations(&mut out, results);
    write_selection(&mut out, results);
    write_walk_forward(&mut out, results);
    write_sensitivity(&mut out, results);
    write_costs(&mut out, results);
    write_commands(&mut out, results);
    write_limitations(&mut out);
    write_recommendation(&mut out, results);

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn write_metadata(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 1. Run metadata").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Generated at | {} |", results.generated_at)
        .expect("writing to String cannot fail");
    writeln!(out, "| As of (ms) | {} |", results.as_of_ms).expect("writing to String cannot fail");
    writeln!(out, "| Months | {} |", results.months).expect("writing to String cannot fail");
    writeln!(out, "| Symbols | {} |", join(&results.symbols))
        .expect("writing to String cannot fail");
    writeln!(out, "| Timeframes | {} |", join(&results.timeframes))
        .expect("writing to String cannot fail");
    writeln!(out, "| Seed | {} |", results.seed).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Base costs | {} bps fee + {} bps slippage per side |",
        results.fee_bps, results.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Stress costs | {} bps fee + {} bps slippage per side |",
        results.stress_fee_bps, results.stress_slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Init fund | {} |", results.init_fund).expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_baseline_config(out: &mut String, results: &SweepResults) {
    let config = &results.baseline_config;
    writeln!(out, "## 1b. Baseline configuration").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Source: {} (`{}`: {}).",
        results.baseline_source,
        results.baseline.label,
        config_summary(&results.baseline)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Parameter | Values | Role |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|").expect("writing to String cannot fail");
    let rows = [
        ("RSI period", "14, 21".to_string(), "varied"),
        ("RSI long threshold", "55, 60, 65, 70".to_string(), "varied"),
        ("RSI short threshold", "40, 45, 50".to_string(), "varied"),
        ("Window", config.window.to_string(), "held constant"),
    ];
    for (parameter, value, role) in rows {
        writeln!(out, "| {parameter} | {value} | {role} |").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_data_quality(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 2. Data quality").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if results.datasets.is_empty() {
        writeln!(out, "No datasets recorded.").expect("writing to String cannot fail");
    } else {
        writeln!(
            out,
            "| Symbol | Timeframe | Requested range (ms) | Effective range (ms) | Candles | Requested | Expected | Missing | Duplicates | Pages | SHA-256 |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|")
            .expect("writing to String cannot fail");
        for dataset in &results.datasets {
            writeln!(
                out,
                "| {} | {} | {}..{} | {}..{} | {} | {} | {} | {} | {} | {} | {} |",
                dataset.symbol,
                dataset.timeframe,
                dataset.requested_first_ms,
                dataset.requested_last_ms,
                dataset.first_open_time_ms,
                dataset.last_open_time_ms,
                dataset.candles,
                dataset.requested_candles,
                dataset.expected_candles,
                dataset.missing_candles,
                dataset.duplicates_dropped,
                dataset.pages,
                dataset.content_sha256
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");

    // Errors and warnings are always reported: an all-fetches-failed run has no datasets but the
    // captured errors are exactly what the reader needs to see.
    writeln!(out, "### Errors").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if results.errors.is_empty() {
        writeln!(out, "No errors recorded.").expect("writing to String cannot fail");
    } else {
        for error in &results.errors {
            writeln!(out, "- {error}").expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "### Warnings").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let has_warnings = results
        .datasets
        .iter()
        .any(|dataset| dataset.missing_candles > 0 || dataset.duplicates_dropped > 0);
    if has_warnings {
        for dataset in &results.datasets {
            if dataset.missing_candles > 0 {
                writeln!(
                    out,
                    "- Warning: {} {} has {} missing candle(s).",
                    dataset.symbol, dataset.timeframe, dataset.missing_candles
                )
                .expect("writing to String cannot fail");
            }
            if dataset.duplicates_dropped > 0 {
                writeln!(
                    out,
                    "- Warning: {} {} dropped {} duplicate candle(s).",
                    dataset.symbol, dataset.timeframe, dataset.duplicates_dropped
                )
                .expect("writing to String cannot fail");
            }
        }
    } else {
        writeln!(out, "No warnings recorded.").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_splits(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 2b. Chronological splits").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if results.splits.is_empty() {
        writeln!(out, "No splits recorded.").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    writeln!(
        out,
        "Each split is chronological: 60% train, 20% validation, 20% holdout, plus three expanding folds inside train+validation. Indices are half-open (`start` inclusive, `end` exclusive); timestamps are the open times of the first and last candle in each range."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Timeframe | Partition | Start index | End index | Start | End | Fold validation ends <= holdout start |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|---|").expect("writing to String cannot fail");
    // Track any fold validation that reaches into the holdout so the closing
    // sentence is only emitted when the guarantee actually holds.
    let mut overlapping: BTreeSet<(String, String)> = BTreeSet::new();
    for split in &results.splits {
        let holdout_start = split.holdout.start_index;
        let mut rows: Vec<(String, &SplitBoundary, bool)> = vec![
            ("train".to_string(), &split.train, false),
            ("validation".to_string(), &split.validation, false),
            ("holdout".to_string(), &split.holdout, false),
        ];
        for fold in &split.folds {
            rows.push((format!("fold{}_train", fold.fold), &fold.train, false));
            rows.push((format!("fold{}_val", fold.fold), &fold.validation, true));
        }
        for (name, boundary, is_fold_validation) in rows {
            let check = if is_fold_validation {
                if boundary.end_index <= holdout_start {
                    "yes"
                } else {
                    overlapping.insert((split.symbol.clone(), split.timeframe.clone()));
                    "NO"
                }
            } else {
                "—"
            };
            writeln!(
                out,
                "| {} | {} | {name} | {} | {} | {} | {} | {check} |",
                split.symbol,
                split.timeframe,
                boundary.start_index,
                boundary.end_index,
                boundary.start_time,
                boundary.end_time
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    if overlapping.is_empty() {
        writeln!(
            out,
            "Every `fold*_val` row above ends at or before the holdout start index, so no fold validation overlaps the holdout."
        )
        .expect("writing to String cannot fail");
    } else {
        let offenders = overlapping
            .iter()
            .map(|(symbol, timeframe)| format!("{symbol} {timeframe}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "- Warning: fold validation overlaps the holdout for {offenders}; see the `NO` rows above."
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_baseline_vs_candidates(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 3. Baseline vs candidates").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Validation is the selection partition; holdout is the one-shot evaluation partition, scored only for the baseline and selected winners, and is never used for selection. Metrics are pooled across symbols under base costs. A missing holdout is shown as —."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Footnote: *pooled max DD* is the maximum per-symbol drawdown, not a portfolio drawdown."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    for timeframe in &results.timeframes {
        writeln!(out, "### {timeframe}").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let outcomes = timeframe_outcomes(results, timeframe);
        if outcomes.is_empty() {
            writeln!(out, "No candidates recorded.").expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }

        writeln!(
            out,
            "| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Val stderr | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|---|")
            .expect("writing to String cannot fail");
        let selection = results.selections.get(timeframe);
        for outcome in outcomes {
            let validation = &outcome.pooled_validation;
            let holdout = outcome.base.get("holdout");
            let (holdout_trades, holdout_expectancy, holdout_pf, holdout_dd) = match holdout {
                Some(metrics) => (
                    metrics.trades.to_string(),
                    metrics.net_expectancy.to_string(),
                    format_profit_factor(metrics.net_profit_factor),
                    metrics.max_drawdown_pct.to_string(),
                ),
                None => (
                    MISSING_METRIC.to_string(),
                    MISSING_METRIC.to_string(),
                    MISSING_METRIC.to_string(),
                    MISSING_METRIC.to_string(),
                ),
            };
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                outcome.label,
                config_summary(&outcome.candidate),
                validation.trades,
                validation.net_expectancy,
                format_profit_factor(validation.net_profit_factor),
                validation.max_drawdown_pct,
                validation.expectancy_stderr,
                holdout_trades,
                holdout_expectancy,
                holdout_pf,
                holdout_dd,
                selection_status(&outcome.label, selection)
            )
            .expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_configurations(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 4. Exact configurations tested and reasons")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The `origin` column records how each candidate entered the study: `baseline` for the pinned profile, or `grid` for the full 24-point RSI grid."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    for timeframe in &results.timeframes {
        writeln!(out, "### {timeframe}").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let outcomes = timeframe_outcomes(results, timeframe);
        if outcomes.is_empty() {
            writeln!(out, "No candidates recorded.").expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }

        writeln!(
            out,
            "| Label | Origin | RSI period | Long threshold | Short threshold |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|").expect("writing to String cannot fail");
        for outcome in outcomes {
            let candidate = &outcome.candidate;
            writeln!(
                out,
                "| {} | {} | {} | {} | {} |",
                outcome.label,
                outcome.origin,
                candidate.rsi_period,
                candidate.rsi_long_threshold,
                candidate.rsi_short_threshold
            )
            .expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_selection(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 5. Selection").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Selection uses validation only under the base cost scenario. Stress results are reported as a robustness check but never used for selection."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    for timeframe in &results.timeframes {
        writeln!(out, "### {timeframe}").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        match results.selections.get(timeframe) {
            Some(selection) => {
                if selection.winners.is_empty() {
                    writeln!(
                        out,
                        "Selection: no candidate meets the criteria for this timeframe."
                    )
                    .expect("writing to String cannot fail");
                } else {
                    writeln!(
                        out,
                        "Winners (in rank order): {}.",
                        code_list(&selection.winners)
                    )
                    .expect("writing to String cannot fail");
                }
                writeln!(out).expect("writing to String cannot fail");
                if selection.notes.is_empty() {
                    writeln!(out, "No notes.").expect("writing to String cannot fail");
                } else {
                    writeln!(out, "Notes:").expect("writing to String cannot fail");
                    for note in &selection.notes {
                        writeln!(out, "- {note}").expect("writing to String cannot fail");
                    }
                }
                writeln!(out).expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    out,
                    "Selection: no candidate meets the criteria for this timeframe (no selection recorded)."
                )
                .expect("writing to String cannot fail");
                writeln!(out).expect("writing to String cannot fail");
            }
        }
    }
}

fn write_walk_forward(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 6. Walk-forward").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Expanding-fold validation metrics are pooled across symbols; per-symbol validation metrics are shown separately. Only the baseline and selected winners are shown."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    for timeframe in &results.timeframes {
        writeln!(out, "### {timeframe}").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let outcomes = timeframe_outcomes(results, timeframe);
        let mut labels = vec!["baseline".to_string()];
        if let Some(selection) = results.selections.get(timeframe) {
            for winner in &selection.winners {
                if !labels.contains(winner) {
                    labels.push(winner.clone());
                }
            }
        }
        let selected: Vec<&CandidateOutcome> = labels
            .iter()
            .filter_map(|label| outcomes.iter().find(|outcome| &outcome.label == label))
            .collect();
        if selected.is_empty() {
            writeln!(out, "No walk-forward data recorded.").expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }

        writeln!(out, "#### Pooled folds").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(out, "| Label | Fold | Net expectancy | Trades | Max DD % |")
            .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|").expect("writing to String cannot fail");
        for outcome in &selected {
            for (index, metrics) in outcome.folds.iter().enumerate() {
                writeln!(
                    out,
                    "| {} | fold{}_val | {} | {} | {} |",
                    outcome.label,
                    index + 1,
                    metrics.net_expectancy,
                    metrics.trades,
                    metrics.max_drawdown_pct
                )
                .expect("writing to String cannot fail");
            }
        }
        writeln!(out).expect("writing to String cannot fail");

        writeln!(out, "#### Per-symbol validation").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "| Label | Symbol | Net expectancy | Trades | Max DD % |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|").expect("writing to String cannot fail");
        for outcome in &selected {
            for (symbol, metrics) in &outcome.per_symbol_validation {
                writeln!(
                    out,
                    "| {} | {} | {} | {} | {} |",
                    outcome.label,
                    symbol,
                    metrics.net_expectancy,
                    metrics.trades,
                    metrics.max_drawdown_pct
                )
                .expect("writing to String cannot fail");
            }
        }
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_sensitivity(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 7. Parameter sensitivity").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Each row groups candidates by a tested parameter value and reports the validation net expectancy range (pooled across symbols, base costs). Joint draws change several parameters at once, so these ranges are descriptive rather than causal."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    type ParameterAccessor = (
        &'static str,
        fn(&Candidate) -> String,
        fn(&str, &str) -> std::cmp::Ordering,
    );
    let parameters: [ParameterAccessor; 3] = [
        (
            "RSI period",
            |candidate| candidate.rsi_period.to_string(),
            lexical_order,
        ),
        (
            "RSI long threshold",
            |candidate| candidate.rsi_long_threshold.to_string(),
            lexical_order,
        ),
        (
            "RSI short threshold",
            |candidate| candidate.rsi_short_threshold.to_string(),
            lexical_order,
        ),
    ];

    for timeframe in &results.timeframes {
        writeln!(out, "### {timeframe}").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        let outcomes = timeframe_outcomes(results, timeframe);
        if outcomes.is_empty() {
            writeln!(out, "No candidates recorded.").expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
            continue;
        }

        writeln!(
            out,
            "| Parameter | Value | Candidates | Mean validation net expectancy | Min | Max |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|---|").expect("writing to String cannot fail");
        for (parameter, accessor, order) in parameters {
            let groups = group_values(outcomes, accessor, order);
            for (value, values) in groups {
                let count = values.len();
                let min = values.iter().copied().min().unwrap_or_default();
                let max = values.iter().copied().max().unwrap_or_default();
                let mean = values.iter().copied().sum::<Decimal>() / Decimal::from(count);
                writeln!(
                    out,
                    "| {parameter} | {value} | {count} | {mean} | {min} | {max} |"
                )
                .expect("writing to String cannot fail");
            }
        }
        writeln!(out).expect("writing to String cannot fail");
    }

    writeln!(
        out,
        "Positions are reversal-only and close at end of data; there are no protective levels."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_costs(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 8. Cost assumptions").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Base scenario: {} bps fee + {} bps slippage per side ({} bps per side in total).",
        results.fee_bps,
        results.slip_bps,
        results.fee_bps + results.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Stress scenario: {} bps fee + {} bps slippage per side ({} bps per side in total).",
        results.stress_fee_bps,
        results.stress_slip_bps,
        results.stress_fee_bps + results.stress_slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Costs are applied once to each side of the actual simulated fill notional. Every reported metric is net of costs; results are gross of nothing."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The simulator caps a position's gross loss at its notional `E`; a net loss may slightly exceed `E` because costs are real."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_commands(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 9. Command (synthesized from run configuration)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The actual process argv was not captured; this invocation is synthesized from the recorded run configuration."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "```bash").expect("writing to String cannot fail");
    writeln!(
        out,
        "make analyzer-sweep ARGS=\"--symbols {} --timeframes {} --months {} --as-of {} --generated-at {} --seed {} --fee-bps {} --slip-bps {} --stress-fee-bps {} --stress-slip-bps {} --init-fund {}\"",
        join_compact(&results.symbols),
        join_compact(&results.timeframes),
        results.months,
        results.as_of_ms,
        results.generated_at,
        results.seed,
        results.fee_bps,
        results.slip_bps,
        results.stress_fee_bps,
        results.stress_slip_bps,
        results.init_fund
    )
    .expect("writing to String cannot fail");
    writeln!(out, "cargo fmt -p analyzer").expect("writing to String cannot fail");
    writeln!(out, "cargo test -p analyzer --features harness")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "cargo clippy -p analyzer --features harness --all-targets --no-deps -- -D warnings"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "cargo build -p analyzer --no-default-features")
        .expect("writing to String cannot fail");
    writeln!(out, "```").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_limitations(out: &mut String) {
    writeln!(out, "## 9b. Limitations").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The neighbor-median-within-one-standard-error stability check from the study spec is approximated by fold and symbol consistency: the full 24-point grid is evaluated, so a candidate's one-step grid neighbors are present. No bootstrap confidence interval is reported; only the pooled standard error of validation net expectancy is shown. The single chronological 60/20/20 split is regime-sensitive, and the holdout is a one-shot evaluation rather than a rolling out-of-sample test."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_recommendation(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 10. Recommendation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let (label, rationale) = recommendation(results);
    writeln!(out, "**{label}**").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "{rationale}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn recommendation(results: &SweepResults) -> (&'static str, String) {
    let mut winner_count = 0usize;
    let mut has_non_baseline_winner = false;
    for selection in results.selections.values() {
        for winner in &selection.winners {
            winner_count += 1;
            if winner != "baseline" {
                has_non_baseline_winner = true;
            }
        }
    }

    // A requested timeframe with no recorded selection (or no winners) makes the run incomplete.
    let missing_selection =
        results
            .timeframes
            .iter()
            .any(|timeframe| match results.selections.get(timeframe) {
                Some(selection) => selection.winners.is_empty(),
                None => true,
            });

    let (label, mut rationale) = if !results.errors.is_empty() {
        (
            "insufficient evidence",
            "The run recorded one or more errors, so the screening is incomplete. Resolve the listed errors and rerun before drawing any conclusion. This is a provisional screening result, not a profitability claim.".to_string(),
        )
    } else if missing_selection || winner_count == 0 {
        (
            "insufficient evidence",
            "No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the study baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising the study baseline.".to_string(),
        )
    } else if has_non_baseline_winner {
        (
            "larger experiment",
            "At least one non-baseline candidate met the validation criteria, but the selection is validation-only and provisional. Confirm the frozen holdout result and the stress scenario in a larger experiment before considering any change to the study baseline. This is not a profitability claim.".to_string(),
        )
    } else {
        (
            "retain baseline",
            "Only the baseline met the validation criteria across the screened timeframes, so there is no supported reason to change the study baseline. This is a provisional screening result, not a profitability claim.".to_string(),
        )
    };

    let negative_holdout = negative_holdout_winners(results);
    if !negative_holdout.is_empty() {
        rationale.push_str(&format!(
            " Selected candidate(s) with non-positive holdout net expectancy: {}.",
            negative_holdout.join(", ")
        ));
    }

    (label, rationale)
}

/// Lists selected candidates whose one-shot holdout net expectancy is not positive.
fn negative_holdout_winners(results: &SweepResults) -> Vec<String> {
    let mut negative = Vec::new();
    for (timeframe, selection) in &results.selections {
        for winner in &selection.winners {
            let Some(outcome) = timeframe_outcomes(results, timeframe)
                .iter()
                .find(|outcome| &outcome.label == winner)
            else {
                continue;
            };
            if let Some(holdout) = outcome.base.get("holdout") {
                if holdout.net_expectancy <= Decimal::ZERO {
                    negative.push(format!("{timeframe}/{winner} ({})", holdout.net_expectancy));
                }
            }
        }
    }
    negative.sort();
    negative
}

fn timeframe_outcomes<'a>(results: &'a SweepResults, timeframe: &str) -> &'a [CandidateOutcome] {
    results
        .outcomes
        .get(timeframe)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn join(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

/// Joins command-line values with commas and no spaces, as they would be passed to argv.
fn join_compact(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(",")
    }
}

fn code_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn config_summary(candidate: &Candidate) -> String {
    format!(
        "RSI {}/{}:{}",
        candidate.rsi_period, candidate.rsi_long_threshold, candidate.rsi_short_threshold
    )
}

fn format_profit_factor(value: Option<Decimal>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn selection_status(label: &str, selection: Option<&Selection>) -> String {
    match selection {
        Some(selection) if selection.winners.iter().any(|winner| winner == label) => {
            if label == "baseline" {
                "baseline, selected".to_string()
            } else {
                "selected".to_string()
            }
        }
        _ => "not selected".to_string(),
    }
}

fn lexical_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.cmp(b)
}

fn group_values(
    outcomes: &[CandidateOutcome],
    accessor: fn(&Candidate) -> String,
    order: fn(&str, &str) -> std::cmp::Ordering,
) -> Vec<(String, Vec<Decimal>)> {
    let mut groups: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for outcome in outcomes {
        groups
            .entry(accessor(&outcome.candidate))
            .or_default()
            .push(outcome.pooled_validation.net_expectancy);
    }
    let mut rows: Vec<(String, Vec<Decimal>)> = groups.into_iter().collect();
    rows.sort_by(|left, right| order(&left.0, &right.0));
    rows
}
