use analyzer_rs::config_env::load_dotenv;
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, Dataset};
use analyzer_rs::harness::sweep::cost_ladder::frozen_candidates;
use analyzer_rs::harness::sweep::execution::{
    evaluate_execution_config, pool_exec, EntryMode, ExecModel, ExecPartition,
};
use analyzer_rs::harness::sweep::partition::chronological_splits;
use analyzer_rs::harness::sweep::sampling::Candidate;
use clap::Parser;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const WINDOW: usize = 100;
const DEFAULT_AS_OF: i64 = 1_792_661_878_840;
const DEFAULT_SETTLE_MS: i64 = 5_000;
const PARTITIONS: [&str; 2] = ["validation", "holdout"];

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-execution",
    about = "Compare market and limit execution models on the focused 4hour set"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,ETH-USDT,SOL-USDT")]
    symbols: String,

    /// Candle cache directory.
    #[arg(long, default_value = "data/sweep")]
    data_dir: PathBuf,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer-rs/experiments/execution")]
    out_dir: PathBuf,

    /// Reference time in milliseconds.
    #[arg(long, default_value_t = DEFAULT_AS_OF)]
    as_of: i64,

    /// Candle settle time in milliseconds.
    #[arg(long, default_value_t = DEFAULT_SETTLE_MS)]
    settle_ms: i64,

    /// Initial account fund per symbol.
    #[arg(long, default_value_t = 1000)]
    init_fund: i64,

    /// Results timestamp in RFC 3339 form.
    #[arg(long, default_value = "2026-10-22T00:00:00Z")]
    generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionRow {
    candidate: String,
    model: String,
    symbol_or_pooled: String,
    partition: String,
    signals: usize,
    filled: usize,
    capital_fill_rate: Decimal,
    execution_fill_rate: Decimal,
    expired: usize,
    skipped_funds: usize,
    invalid_levels: usize,
    trades: usize,
    net_expectancy: Decimal,
    net_profit_factor: Option<Decimal>,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    expectancy_stderr: Decimal,
    net_pnl: Decimal,
}

impl ExecutionRow {
    fn new(
        candidate: &str,
        model: &str,
        symbol_or_pooled: &str,
        partition: &str,
        part: &ExecPartition,
    ) -> Self {
        Self {
            candidate: candidate.to_string(),
            model: model.to_string(),
            symbol_or_pooled: symbol_or_pooled.to_string(),
            partition: partition.to_string(),
            signals: part.signals,
            filled: part.filled,
            capital_fill_rate: part.capital_fill_rate,
            execution_fill_rate: part.execution_fill_rate,
            expired: part.expired,
            skipped_funds: part.skipped_funds,
            invalid_levels: part.invalid_levels,
            trades: part.metrics.trades,
            net_expectancy: part.metrics.net_expectancy,
            net_profit_factor: part.metrics.net_profit_factor,
            win_rate: part.metrics.win_rate,
            max_drawdown_pct: part.metrics.max_drawdown_pct,
            expectancy_stderr: part.metrics.expectancy_stderr,
            net_pnl: part.metrics.net_pnl,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DatasetSummary {
    candles: usize,
    content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateDecision {
    candidate: String,
    selected_model: Option<String>,
    validation_execution_fill_rate: Option<Decimal>,
    validation_capital_fill_rate: Option<Decimal>,
    validation_skipped_funds: Option<usize>,
    validation_net_expectancy: Option<Decimal>,
    validation_expectancy_stderr: Option<Decimal>,
    market_validation_net_expectancy: Option<Decimal>,
    holdout_net_expectancy: Option<Decimal>,
    holdout_expectancy_stderr: Option<Decimal>,
    holdout_profit_factor: Option<Decimal>,
    holdout_positive_symbols: usize,
    holdout_symbols: usize,
    rescues: bool,
    rationale: String,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionSummary {
    decision: String,
    rescues: bool,
    selected_candidate: Option<String>,
    selected_model: Option<String>,
    candidates: Vec<CandidateDecision>,
    caveat: String,
}

#[derive(Debug, Serialize)]
struct ExecutionResults {
    generated_at: String,
    as_of_ms: i64,
    symbols: Vec<String>,
    data_dir: String,
    settle_ms: i64,
    init_fund: i64,
    window: usize,
    models: Vec<ExecModel>,
    datasets: BTreeMap<String, DatasetSummary>,
    rows: Vec<ExecutionRow>,
    decisions: Vec<CandidateDecision>,
    decision: DecisionSummary,
    selection_note: String,
}

struct CandidateRun {
    label: String,
    /// symbol -> model -> partition
    per_symbol: BTreeMap<String, BTreeMap<String, BTreeMap<String, ExecPartition>>>,
    /// model -> partition
    pooled: BTreeMap<String, BTreeMap<String, ExecPartition>>,
}

fn parse_list(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in value.split(',') {
        let item = item.trim();
        if item.is_empty() || out.iter().any(|existing| existing == item) {
            continue;
        }
        out.push(item.to_string());
    }
    out
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Sanitizes the data directory for committed artifacts.
fn data_dir_display(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.trim_end_matches('/').ends_with("data/sweep") {
        "data/sweep".to_string()
    } else {
        text.to_string()
    }
}

fn focused_candidates() -> Vec<Candidate> {
    frozen_candidates()
        .into_iter()
        .filter(|candidate| {
            matches!(
                candidate.label.as_str(),
                "baseline" | "joint-13" | "max_age=4"
            )
        })
        .collect()
}

fn load_datasets(
    symbols: &[String],
    data_dir: &Path,
    as_of: i64,
    settle_ms: i64,
) -> Result<BTreeMap<String, Dataset>, String> {
    let mut datasets = BTreeMap::new();
    for symbol in symbols {
        let path = data_dir.join(format!("{symbol}_4hour.csv"));
        let dataset = load_csv(symbol, &path, Timeframe::H4, as_of, settle_ms, 0, WINDOW, 0)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        datasets.insert(symbol.clone(), dataset);
    }
    Ok(datasets)
}

fn dataset_summaries(datasets: &BTreeMap<String, Dataset>) -> BTreeMap<String, DatasetSummary> {
    datasets
        .iter()
        .map(|(symbol, dataset)| {
            (
                symbol.clone(),
                DatasetSummary {
                    candles: dataset.provenance.effective_range.candles,
                    content_sha256: dataset.provenance.content_sha256.clone(),
                },
            )
        })
        .collect()
}

fn evaluate_candidates(
    candidates: &[Candidate],
    symbols: &[String],
    datasets: &BTreeMap<String, Dataset>,
    models: &[ExecModel],
    init_fund: i64,
) -> Result<Vec<CandidateRun>, String> {
    let mut runs = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut per_symbol: BTreeMap<String, BTreeMap<String, BTreeMap<String, ExecPartition>>> =
            BTreeMap::new();
        for symbol in symbols {
            let dataset = datasets
                .get(symbol)
                .ok_or_else(|| format!("missing dataset for {symbol}"))?;
            let splits = chronological_splits(dataset.candles.len());
            let result = evaluate_execution_config(
                dataset,
                &candidate.to_config(),
                candidate.max_age,
                &splits,
                WINDOW,
                init_fund,
                models,
            )
            .map_err(|error| format!("{} {symbol}: {error}", candidate.label))?;
            per_symbol.insert(symbol.clone(), result);
        }

        let mut pooled: BTreeMap<String, BTreeMap<String, ExecPartition>> = BTreeMap::new();
        for model in models {
            for partition in PARTITIONS {
                let mut parts = Vec::with_capacity(symbols.len());
                for symbol in symbols {
                    let part = per_symbol
                        .get(symbol)
                        .and_then(|models| models.get(&model.label))
                        .and_then(|partitions| partitions.get(partition))
                        .cloned()
                        .ok_or_else(|| {
                            format!(
                                "{} {symbol}: missing {} {partition}",
                                candidate.label, model.label
                            )
                        })?;
                    parts.push(part);
                }
                pooled
                    .entry(model.label.clone())
                    .or_default()
                    .insert(partition.to_string(), pool_exec(&parts));
            }
        }

        runs.push(CandidateRun {
            label: candidate.label.clone(),
            per_symbol,
            pooled,
        });
    }
    Ok(runs)
}

fn build_rows(
    runs: &[CandidateRun],
    models: &[ExecModel],
    symbols: &[String],
) -> Vec<ExecutionRow> {
    let mut rows = Vec::new();
    for run in runs {
        for model in models {
            for partition in PARTITIONS {
                let Some(pooled) = run
                    .pooled
                    .get(&model.label)
                    .and_then(|partitions| partitions.get(partition))
                else {
                    continue;
                };
                rows.push(ExecutionRow::new(
                    &run.label,
                    &model.label,
                    "pooled",
                    partition,
                    pooled,
                ));
                for symbol in symbols {
                    let Some(part) = run
                        .per_symbol
                        .get(symbol)
                        .and_then(|models| models.get(&model.label))
                        .and_then(|partitions| partitions.get(partition))
                    else {
                        continue;
                    };
                    rows.push(ExecutionRow::new(
                        &run.label,
                        &model.label,
                        symbol,
                        partition,
                        part,
                    ));
                }
            }
        }
    }
    rows
}

fn profit_factor_above_one(part: &ExecPartition) -> bool {
    match part.metrics.net_profit_factor {
        Some(value) => value > Decimal::ONE,
        None => part.metrics.trades > 0 && part.metrics.gross_profit > Decimal::ZERO,
    }
}

fn part<'a>(run: &'a CandidateRun, model: &str, partition: &str) -> Option<&'a ExecPartition> {
    run.pooled
        .get(model)
        .and_then(|partitions| partitions.get(partition))
}

fn decide(runs: &[CandidateRun], models: &[ExecModel], symbols: &[String]) -> DecisionSummary {
    let mut decisions = Vec::with_capacity(runs.len());
    for run in runs {
        let market_validation = part(run, "market", "validation")
            .map(|part| part.metrics.net_expectancy)
            .unwrap_or(Decimal::ZERO);

        let mut best: Option<(&ExecModel, &ExecPartition)> = None;
        let mut rejected: Vec<String> = Vec::new();
        for model in models {
            if model.label == "market" {
                continue;
            }
            let Some(validation) = part(run, &model.label, "validation") else {
                continue;
            };
            let fill_ok = validation.execution_fill_rate >= Decimal::new(5, 1);
            let expectancy_positive = validation.metrics.net_expectancy > Decimal::ZERO;
            let beats_market = validation.metrics.net_expectancy > market_validation;
            if fill_ok && expectancy_positive && beats_market {
                if best.is_none_or(|(_, current)| {
                    validation.metrics.net_expectancy > current.metrics.net_expectancy
                }) {
                    best = Some((model, validation));
                }
            } else {
                let mut failures = Vec::new();
                if !fill_ok {
                    failures.push(format!(
                        "execution fill rate {} was below 50%",
                        display_pct(validation.execution_fill_rate)
                    ));
                }
                if !expectancy_positive {
                    failures.push(format!(
                        "net expectancy {} was not positive",
                        display_decimal(validation.metrics.net_expectancy)
                    ));
                } else if !beats_market {
                    failures.push(format!(
                        "net expectancy {} did not beat market {}",
                        display_decimal(validation.metrics.net_expectancy),
                        display_decimal(market_validation)
                    ));
                }
                rejected.push(format!("{} ({})", model.label, failures.join("; ")));
            }
        }

        let Some((model, validation)) = best else {
            let rationale = if rejected.is_empty() {
                "no limit model had a validation partition to evaluate".to_string()
            } else {
                format!(
                    "no limit model met the validation criteria: {}. Execution fill rate excludes orders skipped for capital; capital fill rate and skipped funds are reported separately.",
                    rejected.join("; ")
                )
            };
            decisions.push(CandidateDecision {
                candidate: run.label.clone(),
                selected_model: None,
                validation_execution_fill_rate: None,
                validation_capital_fill_rate: None,
                validation_skipped_funds: None,
                validation_net_expectancy: None,
                validation_expectancy_stderr: None,
                market_validation_net_expectancy: Some(market_validation),
                holdout_net_expectancy: None,
                holdout_expectancy_stderr: None,
                holdout_profit_factor: None,
                holdout_positive_symbols: 0,
                holdout_symbols: symbols.len(),
                rescues: false,
                rationale,
            });
            continue;
        };

        let holdout = part(run, &model.label, "holdout");
        let holdout_net_expectancy = holdout
            .map(|part| part.metrics.net_expectancy)
            .unwrap_or(Decimal::ZERO);
        let holdout_profit_factor = holdout.and_then(|part| part.metrics.net_profit_factor);
        let holdout_positive_symbols = symbols
            .iter()
            .filter(|symbol| {
                run.per_symbol
                    .get(*symbol)
                    .and_then(|models| models.get(&model.label))
                    .and_then(|partitions| partitions.get("holdout"))
                    .is_some_and(|part| part.metrics.net_expectancy > Decimal::ZERO)
            })
            .count();
        let holdout_pf_ok = holdout.is_some_and(profit_factor_above_one);
        let holdout_symbols_ok = holdout_positive_symbols * 3 >= symbols.len().saturating_mul(2);
        let rescues = holdout_net_expectancy > Decimal::ZERO && holdout_pf_ok && holdout_symbols_ok;

        let rationale = if rescues {
            format!(
                "{} passed validation (execution fill rate {}, capital fill rate {}, net expectancy {}) and the holdout held up (net expectancy {}, PF {}, {} of {} symbols positive).",
                model.label,
                display_pct(validation.execution_fill_rate),
                display_pct(validation.capital_fill_rate),
                display_decimal(validation.metrics.net_expectancy),
                display_decimal(holdout_net_expectancy),
                display_profit_factor(holdout_profit_factor),
                holdout_positive_symbols,
                symbols.len()
            )
        } else {
            let mut reasons = Vec::new();
            if holdout_net_expectancy <= Decimal::ZERO {
                reasons.push(format!(
                    "holdout net expectancy {} was not positive",
                    display_decimal(holdout_net_expectancy)
                ));
            }
            if !holdout_pf_ok {
                reasons.push(format!(
                    "holdout PF {} was not above 1",
                    display_profit_factor(holdout_profit_factor)
                ));
            }
            if !holdout_symbols_ok {
                reasons.push(format!(
                    "only {} of {} symbols were positive on the holdout",
                    holdout_positive_symbols,
                    symbols.len()
                ));
            }
            format!(
                "{} passed validation (execution fill rate {}, capital fill rate {}, net expectancy {}) but did not rescue execution: {}.",
                model.label,
                display_pct(validation.execution_fill_rate),
                display_pct(validation.capital_fill_rate),
                display_decimal(validation.metrics.net_expectancy),
                reasons.join("; ")
            )
        };

        decisions.push(CandidateDecision {
            candidate: run.label.clone(),
            selected_model: Some(model.label.clone()),
            validation_execution_fill_rate: Some(validation.execution_fill_rate),
            validation_capital_fill_rate: Some(validation.capital_fill_rate),
            validation_skipped_funds: Some(validation.skipped_funds),
            validation_net_expectancy: Some(validation.metrics.net_expectancy),
            validation_expectancy_stderr: Some(validation.metrics.expectancy_stderr),
            market_validation_net_expectancy: Some(market_validation),
            holdout_net_expectancy: Some(holdout_net_expectancy),
            holdout_expectancy_stderr: holdout.map(|part| part.metrics.expectancy_stderr),
            holdout_profit_factor,
            holdout_positive_symbols,
            holdout_symbols: symbols.len(),
            rescues,
            rationale,
        });
    }

    let rescuer = decisions.iter().find(|decision| decision.rescues);
    let rescues = rescuer.is_some();
    let decision = if rescues {
        "execution rescues".to_string()
    } else {
        "execution does not rescue".to_string()
    };

    DecisionSummary {
        decision,
        rescues,
        selected_candidate: rescuer.map(|decision| decision.candidate.clone()),
        selected_model: rescuer.and_then(|decision| decision.selected_model.clone()),
        candidates: decisions,
        caveat: "The holdout has been scored in several earlier experiments, so it is no longer a clean out-of-sample test. Treat these holdout results as cautionary, and prefer the validation result for model selection.".to_string(),
    }
}

fn format_entry(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => "next candle open".to_string(),
        EntryMode::Limit { offset_bps, .. } if offset_bps.is_zero() => {
            "limit at signal entry".to_string()
        }
        EntryMode::Limit { offset_bps, .. } => {
            format!("limit {offset_bps} bps better than signal entry")
        }
    }
}

fn format_validity(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => "1 candle".to_string(),
        EntryMode::Limit {
            valid_candles: 1, ..
        } => "1 candle".to_string(),
        EntryMode::Limit { valid_candles, .. } => format!("{valid_candles} candles"),
    }
}

fn format_buffer(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => "-".to_string(),
        EntryMode::Limit { buffer_bps, .. } => buffer_bps.to_string(),
    }
}

fn format_offset(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => "-".to_string(),
        EntryMode::Limit { offset_bps, .. } => offset_bps.to_string(),
    }
}

fn maker_label(model: &ExecModel) -> String {
    format!("maker {} bps", model.maker_bps)
}

fn taker_label(model: &ExecModel) -> String {
    format!("taker {} bps", model.taker_bps + model.slip_bps)
}

fn format_entry_cost(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => taker_label(model),
        EntryMode::Limit { .. } => maker_label(model),
    }
}

fn format_tp_cost(model: &ExecModel) -> String {
    match &model.entry {
        EntryMode::Market => taker_label(model),
        EntryMode::Limit { .. } => maker_label(model),
    }
}

fn format_sl_cost(model: &ExecModel) -> String {
    taker_label(model)
}

fn display_decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn display_pct(value: Decimal) -> String {
    format!("{}%", (value * Decimal::from(100)).round_dp(2))
}

/// 95% confidence interval for a mean, using the normal approximation (±1.96 standard errors).
fn display_ci(expectancy: Decimal, stderr: Decimal) -> String {
    let margin = stderr * Decimal::new(196, 2);
    format!(
        "{}..{}",
        display_decimal(expectancy - margin),
        display_decimal(expectancy + margin)
    )
}

fn display_profit_factor(value: Option<Decimal>) -> String {
    value
        .map(|value| value.round_dp(2).to_string())
        .unwrap_or_else(|| "-".to_string())
}

const CSV_COLUMNS: [&str; 18] = [
    "candidate",
    "model",
    "symbol_or_pooled",
    "partition",
    "signals",
    "filled",
    "capital_fill_rate",
    "execution_fill_rate",
    "expired",
    "skipped_funds",
    "invalid_levels",
    "trades",
    "net_expectancy",
    "net_profit_factor",
    "win_rate",
    "max_drawdown_pct",
    "expectancy_stderr",
    "net_pnl",
];

fn write_csv(path: &Path, rows: &[ExecutionRow]) -> Result<(), String> {
    ensure_parent(path)?;
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(CSV_COLUMNS)
        .map_err(|error| error.to_string())?;
    for row in rows {
        let record = [
            row.candidate.clone(),
            row.model.clone(),
            row.symbol_or_pooled.clone(),
            row.partition.clone(),
            row.signals.to_string(),
            row.filled.to_string(),
            row.capital_fill_rate.to_string(),
            row.execution_fill_rate.to_string(),
            row.expired.to_string(),
            row.skipped_funds.to_string(),
            row.invalid_levels.to_string(),
            row.trades.to_string(),
            row.net_expectancy.to_string(),
            row.net_profit_factor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.win_rate.to_string(),
            row.max_drawdown_pct.to_string(),
            row.expectancy_stderr.to_string(),
            row.net_pnl.to_string(),
        ];
        writer
            .write_record(record)
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_json(path: &Path, results: &ExecutionResults) -> Result<(), String> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn write_partition_table(
    out: &mut String,
    title: &str,
    partition: &str,
    runs: &[CandidateRun],
    models: &[ExecModel],
) {
    writeln!(out, "## {title}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Candidate | Model | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Skipped funds | Net expectancy | SE | 95% CI | PF | Trades | Max DD % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        .expect("writing to String cannot fail");
    for run in runs {
        for model in models {
            let Some(part) = part(run, &model.label, partition) else {
                continue;
            };
            writeln!(
                out,
                "| {} | {} | {} | {} | {}/{} | {} | {} | {} | {} | {} | {} | {} | {} |",
                run.label,
                model.label,
                display_pct(part.execution_fill_rate),
                display_pct(part.capital_fill_rate),
                part.filled,
                part.signals,
                part.expired,
                part.skipped_funds,
                part.metrics.net_expectancy.round_dp(4),
                part.metrics.expectancy_stderr.round_dp(4),
                display_ci(part.metrics.net_expectancy, part.metrics.expectancy_stderr),
                display_profit_factor(part.metrics.net_profit_factor),
                part.metrics.trades,
                part.metrics.max_drawdown_pct.round_dp(2)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_per_symbol_table(
    out: &mut String,
    title: &str,
    partition: &str,
    runs: &[CandidateRun],
    models: &[ExecModel],
    symbols: &[String],
) {
    writeln!(out, "## {title}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Candidate | Model | Symbol | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Net expectancy | PF | Trades |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|---|---|---|")
        .expect("writing to String cannot fail");
    for run in runs {
        for model in models {
            for symbol in symbols {
                let Some(part) = run
                    .per_symbol
                    .get(symbol)
                    .and_then(|models| models.get(&model.label))
                    .and_then(|partitions| partitions.get(partition))
                else {
                    continue;
                };
                writeln!(
                    out,
                    "| {} | {} | {symbol} | {} | {} | {}/{} | {} | {} | {} | {} |",
                    run.label,
                    model.label,
                    display_pct(part.execution_fill_rate),
                    display_pct(part.capital_fill_rate),
                    part.filled,
                    part.signals,
                    part.expired,
                    part.metrics.net_expectancy.round_dp(4),
                    display_profit_factor(part.metrics.net_profit_factor),
                    part.metrics.trades
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

/// Writes per-symbol validation and holdout expectancy for the selected model against `market`.
///
/// The table is only populated when the pre-registered rule selects a model; otherwise the section
/// states that there is nothing to compare.
fn write_relative_to_market(
    out: &mut String,
    decision: &DecisionSummary,
    runs: &[CandidateRun],
    symbols: &[String],
) {
    writeln!(out, "## 8. Relative to market (selected model)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let (Some(candidate_label), Some(model_label)) =
        (&decision.selected_candidate, &decision.selected_model)
    else {
        writeln!(
            out,
            "No model met the pre-registered rescue rule, so there is no selected model to compare against `market`."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    };
    let Some(run) = runs.iter().find(|run| &run.label == candidate_label) else {
        writeln!(
            out,
            "Selected candidate `{candidate_label}` was not present in the evaluated runs."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    };

    writeln!(
        out,
        "Selected model: `{candidate_label}` / `{model_label}`. Deltas are selected minus `market` on the same execution simulator."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Partition | Symbol | Market net expectancy | {model_label} net expectancy | Delta |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|").expect("writing to String cannot fail");
    for partition in PARTITIONS {
        for symbol in symbols {
            let market = run
                .per_symbol
                .get(symbol)
                .and_then(|models| models.get("market"))
                .and_then(|partitions| partitions.get(partition));
            let selected = run
                .per_symbol
                .get(symbol)
                .and_then(|models| models.get(model_label))
                .and_then(|partitions| partitions.get(partition));
            let (Some(market), Some(selected)) = (market, selected) else {
                continue;
            };
            let delta = selected.metrics.net_expectancy - market.metrics.net_expectancy;
            writeln!(
                out,
                "| {partition} | {symbol} | {} | {} | {} |",
                display_decimal(market.metrics.net_expectancy),
                display_decimal(selected.metrics.net_expectancy),
                display_decimal(delta)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_markdown(
    path: &Path,
    cli: &Cli,
    data_dir: &str,
    symbols: &[String],
    models: &[ExecModel],
    runs: &[CandidateRun],
    decision: &DecisionSummary,
) -> Result<(), String> {
    ensure_parent(path)?;
    let mut out = String::new();
    writeln!(out, "# Analyzer Execution Modelling (Experiment 4)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Provisional research result. This report does not claim profitability."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 1. Run metadata").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Generated at | {} |", cli.generated_at)
        .expect("writing to String cannot fail");
    writeln!(out, "| As of (ms) | {} |", cli.as_of).expect("writing to String cannot fail");
    writeln!(out, "| Symbols | {} |", symbols.join(", ")).expect("writing to String cannot fail");
    writeln!(out, "| Data dir | {data_dir} |").expect("writing to String cannot fail");
    writeln!(out, "| Settle (ms) | {} |", cli.settle_ms).expect("writing to String cannot fail");
    writeln!(out, "| Init fund | {} |", cli.init_fund).expect("writing to String cannot fail");
    writeln!(out, "| Warm-up window | {WINDOW} |").expect("writing to String cannot fail");
    writeln!(out, "| Partitions | validation, holdout |").expect("writing to String cannot fail");
    let labels = runs
        .iter()
        .map(|run| run.label.clone())
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "| Candidates | {labels} |").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 2. Models").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Model | Entry | Validity | Buffer (bps) | Offset (bps) | Entry cost | TP exit | SL/gap/EOD exit |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|---|").expect("writing to String cannot fail");
    for model in models {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            model.label,
            format_entry(model),
            format_validity(model),
            format_buffer(model),
            format_offset(model),
            format_entry_cost(model),
            format_tp_cost(model),
            format_sl_cost(model)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Costs apply once per side on the actual notional; the metrics are already net of them."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    write_partition_table(
        &mut out,
        "3. Validation (model selection, pooled across symbols)",
        "validation",
        runs,
        models,
    );
    write_partition_table(
        &mut out,
        "4. Holdout (cautionary, pooled across symbols)",
        "holdout",
        runs,
        models,
    );
    write_per_symbol_table(
        &mut out,
        "5. Per-symbol net expectancy (validation)",
        "validation",
        runs,
        models,
        symbols,
    );
    write_per_symbol_table(
        &mut out,
        "6. Per-symbol net expectancy (holdout)",
        "holdout",
        runs,
        models,
        symbols,
    );

    writeln!(out, "## 7. Decision").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if decision.rescues {
        writeln!(out, "**nominal rescue — holdout edge is within noise**")
            .expect("writing to String cannot fail");
    } else {
        writeln!(out, "**execution does not rescue**").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    match (&decision.selected_candidate, &decision.selected_model) {
        (Some(candidate), Some(model)) if decision.rescues => {
            writeln!(
                out,
                "Pre-registered rule result: **execution rescues**. Selected model: `{candidate}` / `{model}`."
            )
            .expect("writing to String cannot fail");
        }
        _ => {
            writeln!(
                out,
                "Pre-registered rule result: **execution does not rescue**; retain the baseline."
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "A limit model is a candidate only if pooled validation net expectancy is positive, the validation execution fill rate is at least 50%, and it beats `market` on validation net expectancy. The execution fill rate divides by executable orders (signals minus skipped funds and invalid levels); the capital fill rate still divides by all signals. It rescues execution only if pooled holdout net expectancy is positive, holdout PF is above 1, and at least two of three symbols are positive on the holdout."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Candidate | Selected model | Validation net expectancy | Val SE | Val 95% CI | Validation exec fill rate | Validation capital fill rate | Validation skipped funds | Holdout net expectancy | Holdout SE | Holdout 95% CI | Holdout PF | Positive symbols | Rescues | Rationale |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    )
    .expect("writing to String cannot fail");
    for candidate in &decision.candidates {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {} | {} |",
            candidate.candidate,
            candidate
                .selected_model
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .validation_net_expectancy
                .map(display_decimal)
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .validation_expectancy_stderr
                .map(display_decimal)
                .unwrap_or_else(|| "-".to_string()),
            match (
                candidate.validation_net_expectancy,
                candidate.validation_expectancy_stderr
            ) {
                (Some(mean), Some(stderr)) => display_ci(mean, stderr),
                _ => "-".to_string(),
            },
            candidate
                .validation_execution_fill_rate
                .map(display_pct)
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .validation_capital_fill_rate
                .map(display_pct)
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .validation_skipped_funds
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .holdout_net_expectancy
                .map(display_decimal)
                .unwrap_or_else(|| "-".to_string()),
            candidate
                .holdout_expectancy_stderr
                .map(display_decimal)
                .unwrap_or_else(|| "-".to_string()),
            match (
                candidate.holdout_net_expectancy,
                candidate.holdout_expectancy_stderr
            ) {
                (Some(mean), Some(stderr)) => display_ci(mean, stderr),
                _ => "-".to_string(),
            },
            display_profit_factor(candidate.holdout_profit_factor),
            candidate.holdout_positive_symbols,
            candidate.holdout_symbols,
            if candidate.rescues { "yes" } else { "no" },
            candidate.rationale
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Both 95% CIs include zero; the validation→holdout decay is ~10×. This is not evidence of a durable edge."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Multiple comparisons: 3 candidates × 3 limit models were tested, and the holdout has been reused across experiments; the nominal rescue should be read against that search."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "**Holdout caveat:** {}", decision.caveat)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    write_relative_to_market(&mut out, decision, runs, symbols);

    writeln!(out, "## 9. Capacity / capital constraint").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if let Some(validation) = runs
        .iter()
        .find(|run| run.label == "joint-13")
        .and_then(|run| part(run, "market", "validation"))
    {
        writeln!(
            out,
            "- `joint-13` produced {} validation signals but only {} were funded (`E = init_fund / 2`, max two concurrent positions); {} were skipped for funds.",
            validation.signals, validation.filled, validation.skipped_funds
        )
        .expect("writing to String cannot fail");
    }
    writeln!(
        out,
        "- This is a strategy-capacity issue, not an execution-model failure."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- A larger `--init-fund` or smaller `E` would change the comparison; not tested here."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 10. Market control").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The execution simulator closes open positions at partition end (`eod`, taker), while the cost ladder marks them; absolute `market` metrics can differ slightly from the cost-ladder report. Model comparisons are within the same execution simulator."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 11. Limitations").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Candle-level fill model; no order-book or queue simulation."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Maker fills assume the limit is resting and filled when touched; adverse selection is approximated by the fill buffer."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Open-through limit fills use the open price (price improvement) with no placement latency or queue risk; this is optimistic on gap opens."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Costs are deducted from PnL but not from available capital for sizing; capacity-constrained configs are slightly optimistic."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Multiple uses of the same holdout across experiments reduce its evidential value."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- Costs are provisional, not verified KuCoin fees.")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn print_summary(runs: &[CandidateRun], models: &[ExecModel], decision: &DecisionSummary) {
    println!(
        "execution modelling completed for {} candidate(s) across {} model(s)",
        runs.len(),
        models.len()
    );
    for run in runs {
        for model in models {
            let validation = part(run, &model.label, "validation");
            let holdout = part(run, &model.label, "holdout");
            if let (Some(validation), Some(holdout)) = (validation, holdout) {
                println!(
                    "  {} / {}: validation net={} exec_fill={} capital_fill={} holdout net={} exec_fill={} capital_fill={}",
                    run.label,
                    model.label,
                    validation.metrics.net_expectancy.round_dp(4),
                    display_pct(validation.execution_fill_rate),
                    display_pct(validation.capital_fill_rate),
                    holdout.metrics.net_expectancy.round_dp(4),
                    display_pct(holdout.execution_fill_rate),
                    display_pct(holdout.capital_fill_rate)
                );
            }
        }
    }
    println!("  decision: {}", decision.decision);
}

fn main() -> Result<(), String> {
    load_dotenv();
    let cli = Cli::parse();

    let symbols = parse_list(&cli.symbols);
    if symbols.is_empty() {
        return Err("--symbols resolved to an empty list".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.settle_ms < 0 {
        return Err("--settle-ms must be non-negative".into());
    }
    chrono::DateTime::parse_from_rfc3339(&cli.generated_at).map_err(|_| {
        format!(
            "invalid --generated-at {:?}; expected an RFC 3339 timestamp",
            cli.generated_at
        )
    })?;

    let models = ExecModel::focused();
    let candidates = focused_candidates();
    if candidates.is_empty() {
        return Err("the focused candidate filter matched no frozen candidates".into());
    }
    let datasets = load_datasets(&symbols, &cli.data_dir, cli.as_of, cli.settle_ms)?;
    let runs = evaluate_candidates(&candidates, &symbols, &datasets, &models, cli.init_fund)?;
    let rows = build_rows(&runs, &models, &symbols);
    let decision = decide(&runs, &models, &symbols);
    let data_dir = data_dir_display(&cli.data_dir);

    let results = ExecutionResults {
        generated_at: cli.generated_at.clone(),
        as_of_ms: cli.as_of,
        symbols: symbols.clone(),
        data_dir: data_dir.clone(),
        settle_ms: cli.settle_ms,
        init_fund: cli.init_fund,
        window: WINDOW,
        models: models.clone(),
        datasets: dataset_summaries(&datasets),
        rows: rows.clone(),
        decisions: decision.candidates.clone(),
        decision: decision.clone(),
        selection_note: "Validation is the model-selection partition. The holdout has been used multiple times across experiments, so it is a cautionary check rather than clean out-of-sample evidence.".to_string(),
    };

    let csv_path = cli.out_dir.join("execution.csv");
    let json_path = cli.out_dir.join("results.json");
    let report_path = cli.out_dir.join("report.md");
    write_csv(&csv_path, &rows)?;
    write_json(&json_path, &results)?;
    write_markdown(
        &report_path,
        &cli,
        &data_dir,
        &symbols,
        &models,
        &runs,
        &decision,
    )?;

    println!("wrote {}", csv_path.display());
    println!("wrote {}", json_path.display());
    println!("wrote {}", report_path.display());
    print_summary(&runs, &models, &decision);

    Ok(())
}
