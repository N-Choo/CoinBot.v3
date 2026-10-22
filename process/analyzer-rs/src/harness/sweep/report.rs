use crate::harness::sweep::evaluate::PartitionMetrics;
use crate::harness::sweep::ranking::{CandidateOutcome, Selection, Thresholds};
use crate::harness::sweep::sampling::{Candidate, SMA_PERIOD};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
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
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidatePlan {
    pub seed: u64,
    pub candidates: Vec<Candidate>,
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

const LEADERBOARD_COLUMNS: [&str; 31] = [
    "timeframe",
    "symbol_or_pooled",
    "label",
    "origin",
    "rsi_period",
    "rsi_order",
    "max_age",
    "sma_enabled",
    "sma_period",
    "atr_sl_multiplier",
    "atr_tp_multiplier",
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
    "tp_hits",
    "sl_hits",
    "other_exits",
    "exposure_candles",
    "time_in_market_pct",
    "expectancy_stderr",
    "stress_validation_net_pnl",
    "stress_validation_expectancy",
];

const BASE_PARTITIONS: [&str; 6] = [
    "train",
    "validation",
    "holdout",
    "fold1_val",
    "fold2_val",
    "fold3_val",
];

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

/// Reports every base partition that an outcome is missing, so CSV output never silently drops rows.
fn missing_base_partitions(results: &SweepResults) -> Vec<String> {
    let mut missing = Vec::new();
    for (timeframe, outcomes) in &results.outcomes {
        for outcome in outcomes {
            for partition in BASE_PARTITIONS {
                if !outcome.base.contains_key(partition) {
                    missing.push(format!("{timeframe}/{}: {partition}", outcome.label));
                }
            }
        }
    }
    missing
}

/// Writes one row per timeframe × candidate × symbol-or-pooled × base partition.
///
/// Per-symbol rows carry the validation partition only, because that is the per-symbol partition
/// recorded by [`CandidateOutcome`]. Stress columns are populated for the pooled validation row;
/// they are empty where no stress metrics were recorded.
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
            for partition in BASE_PARTITIONS {
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
    let max_age = candidate
        .max_age
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
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
        candidate.rsi_order.to_string(),
        max_age,
        candidate.sma_enabled.to_string(),
        candidate.sma_period.to_string(),
        candidate.atr_sl_multiplier.to_string(),
        candidate.atr_tp_multiplier.to_string(),
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
        metrics.tp_hits.to_string(),
        metrics.sl_hits.to_string(),
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
    write_data_quality(&mut out, results);
    write_baseline_vs_candidates(&mut out, results);
    write_configurations(&mut out, results);
    write_selection(&mut out, results);
    write_walk_forward(&mut out, results);
    write_sensitivity(&mut out, results);
    write_costs(&mut out, results);
    write_commands(&mut out, results);
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

fn write_baseline_vs_candidates(out: &mut String, results: &SweepResults) {
    writeln!(out, "## 3. Baseline vs candidates").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Validation is the selection partition; holdout is the one-shot evaluation partition and is never used for selection. Metrics are pooled across symbols under base costs."
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
            "| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|")
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
                    "-".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                ),
            };
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                outcome.label,
                config_summary(&outcome.candidate),
                validation.trades,
                validation.net_expectancy,
                format_profit_factor(validation.net_profit_factor),
                validation.max_drawdown_pct,
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
        "The `origin` column records how each candidate entered the study: `baseline`, `ofat:<parameter>=<value>` (one factor at a time), or `joint` (deterministic seeded draw)."
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
            "| Label | Origin | RSI period | RSI order | Max age | SMA enabled | SMA period | ATR SL | ATR TP |"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "|---|---|---|---|---|---|---|---|---|")
            .expect("writing to String cannot fail");
        for outcome in outcomes {
            let candidate = &outcome.candidate;
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                outcome.label,
                outcome.origin,
                candidate.rsi_period,
                candidate.rsi_order,
                age_text(candidate),
                candidate.sma_enabled,
                candidate.sma_period,
                candidate.atr_sl_multiplier,
                candidate.atr_tp_multiplier
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

    type ParameterAccessor = (&'static str, fn(&Candidate) -> String);
    let parameters: [ParameterAccessor; 6] = [
        ("RSI period", |candidate| candidate.rsi_period.to_string()),
        ("RSI order", |candidate| candidate.rsi_order.to_string()),
        ("Max age", age_text),
        ("SMA enabled", |candidate| candidate.sma_enabled.to_string()),
        ("ATR SL multiplier", |candidate| {
            candidate.atr_sl_multiplier.to_string()
        }),
        ("ATR TP multiplier", |candidate| {
            candidate.atr_tp_multiplier.to_string()
        }),
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
        for (parameter, accessor) in parameters {
            let groups = group_values(outcomes, accessor);
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
        "`SMA_PERIOD` was fixed at {SMA_PERIOD} for every candidate, so this study cannot rank 50/100/200."
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
        "make analyzer-sweep ARGS=\"--symbols {} --timeframes {} --months {} --as-of {} --seed {} --fee-bps {} --slip-bps {} --stress-fee-bps {} --stress-slip-bps {} --init-fund {}\"",
        join_compact(&results.symbols),
        join_compact(&results.timeframes),
        results.months,
        results.as_of_ms,
        results.seed,
        results.fee_bps,
        results.slip_bps,
        results.stress_fee_bps,
        results.stress_slip_bps,
        results.init_fund
    )
    .expect("writing to String cannot fail");
    writeln!(out, "cargo fmt -p analyzer-rs").expect("writing to String cannot fail");
    writeln!(out, "cargo test -p analyzer-rs --features harness")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "cargo build -p analyzer-rs --no-default-features")
        .expect("writing to String cannot fail");
    writeln!(out, "```").expect("writing to String cannot fail");
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

    if !results.errors.is_empty() {
        (
            "insufficient evidence",
            "The run recorded one or more errors, so the screening is incomplete. Resolve the listed errors and rerun before drawing any conclusion. This is a provisional screening result, not a profitability claim.".to_string(),
        )
    } else if missing_selection || winner_count == 0 {
        (
            "insufficient evidence",
            "No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising defaults.".to_string(),
        )
    } else if has_non_baseline_winner {
        (
            "larger experiment",
            "At least one non-baseline candidate met the validation criteria, but the selection is validation-only and provisional. Run a larger experiment that also evaluates the holdout and the stress scenario before considering any default change. This is not a profitability claim.".to_string(),
        )
    } else {
        (
            "retain baseline",
            "Only the baseline met the validation criteria across the screened timeframes, so there is no supported reason to change production defaults. This is a provisional screening result, not a profitability claim.".to_string(),
        )
    }
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

fn age_text(candidate: &Candidate) -> String {
    candidate
        .max_age
        .map(|value| value.to_string())
        .unwrap_or_else(|| "uncapped".to_string())
}

fn config_summary(candidate: &Candidate) -> String {
    format!(
        "RSI {}/o{} age {} SMA {} ({}) SL {}x TP {}x",
        candidate.rsi_period,
        candidate.rsi_order,
        age_text(candidate),
        if candidate.sma_enabled { "on" } else { "off" },
        candidate.sma_period,
        candidate.atr_sl_multiplier,
        candidate.atr_tp_multiplier
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

fn group_values(
    outcomes: &[CandidateOutcome],
    accessor: fn(&Candidate) -> String,
) -> BTreeMap<String, Vec<Decimal>> {
    let mut groups: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for outcome in outcomes {
        groups
            .entry(accessor(&outcome.candidate))
            .or_default()
            .push(outcome.pooled_validation.net_expectancy);
    }
    groups
}
