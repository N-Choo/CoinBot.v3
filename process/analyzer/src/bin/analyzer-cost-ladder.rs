use analyzer::config_env::load_dotenv;
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::cost_ladder::{break_even_bps, frozen_candidates, scenarios};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::evaluate::{evaluate_config, pool, PartitionMetrics};
use analyzer::harness::sweep::partition::chronological_splits;
use analyzer::harness::sweep::sampling::Candidate;
use clap::Parser;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const WINDOW: usize = 100;
const DEFAULT_AS_OF: i64 = 1_792_661_878_840;
const DEFAULT_SETTLE_MS: i64 = 5_000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-cost-ladder",
    about = "Evaluate the frozen 4hour screening set across per-side cost levels"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,ETH-USDT,SOL-USDT")]
    symbols: String,

    /// Candle cache directory.
    #[arg(long, default_value = "data/sweep")]
    data_dir: PathBuf,

    /// Artifact output directory.
    #[arg(
        long,
        default_value = "process/analyzer/experiments/historical/rsi-divergence-cost-ladder"
    )]
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
struct CostRow {
    label: String,
    cost_bps_per_side: Decimal,
    partition: String,
    symbol_or_pooled: String,
    trades: usize,
    gross_expectancy: Decimal,
    net_expectancy: Decimal,
    net_expectancy_r: Decimal,
    net_profit_factor: Option<Decimal>,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    expectancy_stderr: Decimal,
    closed_net_pnl: Decimal,
    open_net_pnl: Decimal,
    net_pnl: Decimal,
}

impl CostRow {
    fn new(
        label: &str,
        cost_bps_per_side: Decimal,
        partition: &str,
        symbol_or_pooled: &str,
        metrics: &PartitionMetrics,
    ) -> Self {
        Self {
            label: label.to_string(),
            cost_bps_per_side,
            partition: partition.to_string(),
            symbol_or_pooled: symbol_or_pooled.to_string(),
            trades: metrics.trades,
            gross_expectancy: if metrics.trades == 0 {
                Decimal::ZERO
            } else {
                metrics.gross_pnl / Decimal::from(metrics.trades)
            },
            net_expectancy: metrics.net_expectancy,
            net_expectancy_r: metrics.net_expectancy_r,
            net_profit_factor: metrics.net_profit_factor,
            win_rate: metrics.win_rate,
            max_drawdown_pct: metrics.max_drawdown_pct,
            expectancy_stderr: metrics.expectancy_stderr,
            closed_net_pnl: metrics.closed_net_pnl,
            open_net_pnl: metrics.open_net_pnl,
            net_pnl: metrics.net_pnl,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConfigCostSummary {
    label: String,
    break_even_validation_bps: Option<Decimal>,
    break_even_holdout_bps: Option<Decimal>,
    gross_expectancy_validation_bps0: Decimal,
    gross_expectancy_holdout_bps0: Decimal,
}

#[derive(Debug, Serialize)]
struct DatasetSummary {
    candles: usize,
    content_sha256: String,
}

#[derive(Debug, Serialize)]
struct VerdictSummary {
    validation_verdict: String,
    validation_best_break_even_bps: Option<Decimal>,
    validation_best_break_even_display: String,
    holdout_verdict: String,
    best_holdout_break_even_bps: Option<Decimal>,
    best_holdout_break_even_display: String,
    decision: String,
}

#[derive(Debug, Serialize)]
struct CostLadderResults {
    generated_at: String,
    as_of_ms: i64,
    symbols: Vec<String>,
    data_dir: String,
    data_dir_display: String,
    settle_ms: i64,
    init_fund: i64,
    levels_bps_per_side: Vec<Decimal>,
    datasets: BTreeMap<String, DatasetSummary>,
    rows: Vec<CostRow>,
    configs: Vec<ConfigCostSummary>,
    verdicts: VerdictSummary,
    selection_note: String,
}

struct CandidateRun {
    label: String,
    /// scenario label -> symbol -> metrics
    validation: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    holdout: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    /// scenario label -> fold name -> symbol -> metrics
    folds: BTreeMap<String, BTreeMap<String, BTreeMap<String, PartitionMetrics>>>,
}

struct CandidateReport {
    label: String,
    /// scenario label -> pooled metrics
    validation_pooled: BTreeMap<String, PartitionMetrics>,
    holdout_pooled: BTreeMap<String, PartitionMetrics>,
    /// scenario label -> fold name -> pooled metrics
    folds_pooled: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    /// scenario label -> symbol -> metrics
    per_symbol_validation: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    per_symbol_holdout: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    summary: ConfigCostSummary,
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
///
/// The canonical cache lives under a user-specific absolute path; artifacts only need to record
/// the repository-relative location when the supplied directory is the standard `data/sweep`.
fn data_dir_display(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.trim_end_matches('/').ends_with("data/sweep") {
        "data/sweep".to_string()
    } else {
        text.to_string()
    }
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
    scenarios: &[CostScenario],
    init_fund: i64,
) -> Result<Vec<CandidateRun>, String> {
    let mut runs = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut validation: BTreeMap<String, BTreeMap<String, PartitionMetrics>> = BTreeMap::new();
        let mut holdout: BTreeMap<String, BTreeMap<String, PartitionMetrics>> = BTreeMap::new();
        let mut folds: BTreeMap<String, BTreeMap<String, BTreeMap<String, PartitionMetrics>>> =
            BTreeMap::new();
        for symbol in symbols {
            let dataset = datasets
                .get(symbol)
                .ok_or_else(|| format!("missing dataset for {symbol}"))?;
            let splits = chronological_splits(dataset.candles.len());
            let fold_names: Vec<String> = splits
                .folds
                .iter()
                .enumerate()
                .map(|(index, _)| format!("fold{}_val", index + 1))
                .collect();
            let result = evaluate_config(
                dataset,
                &candidate.to_config(),
                &splits,
                WINDOW,
                init_fund,
                scenarios,
                true,
            )
            .map_err(|error| format!("{} {symbol}: {error}", candidate.label))?;
            for scenario in scenarios {
                let partitions = result
                    .partitions
                    .get(&scenario.label)
                    .ok_or_else(|| format!("missing {} scenario", scenario.label))?;
                let validation_metrics = partitions
                    .get("validation")
                    .cloned()
                    .ok_or_else(|| format!("{} {symbol}: missing validation", candidate.label))?;
                let holdout_metrics = partitions
                    .get("holdout")
                    .cloned()
                    .ok_or_else(|| format!("{} {symbol}: missing holdout", candidate.label))?;
                validation
                    .entry(scenario.label.clone())
                    .or_default()
                    .insert(symbol.clone(), validation_metrics);
                holdout
                    .entry(scenario.label.clone())
                    .or_default()
                    .insert(symbol.clone(), holdout_metrics);
                for fold_name in &fold_names {
                    let fold_metrics = partitions.get(fold_name).cloned().ok_or_else(|| {
                        format!(
                            "{} {symbol}: missing {fold_name} for {}",
                            candidate.label, scenario.label
                        )
                    })?;
                    folds
                        .entry(scenario.label.clone())
                        .or_default()
                        .entry(fold_name.clone())
                        .or_default()
                        .insert(symbol.clone(), fold_metrics);
                }
            }
        }
        runs.push(CandidateRun {
            label: candidate.label.clone(),
            validation,
            holdout,
            folds,
        });
    }
    Ok(runs)
}

fn pool_by_scenario(
    metrics_by_scenario: &BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    scenarios: &[CostScenario],
    symbols: &[String],
) -> Result<BTreeMap<String, PartitionMetrics>, String> {
    let mut pooled = BTreeMap::new();
    for scenario in scenarios {
        let by_symbol = metrics_by_scenario
            .get(&scenario.label)
            .ok_or_else(|| format!("missing {} scenario", scenario.label))?;
        let mut metrics = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            metrics.push(
                by_symbol
                    .get(symbol)
                    .cloned()
                    .ok_or_else(|| format!("missing {} for {symbol}", scenario.label))?,
            );
        }
        pooled.insert(scenario.label.clone(), pool(&metrics));
    }
    Ok(pooled)
}

fn pool_folds(
    folds: &BTreeMap<String, BTreeMap<String, BTreeMap<String, PartitionMetrics>>>,
    scenarios: &[CostScenario],
    symbols: &[String],
) -> Result<BTreeMap<String, BTreeMap<String, PartitionMetrics>>, String> {
    let mut pooled = BTreeMap::new();
    for scenario in scenarios {
        let by_fold = folds
            .get(&scenario.label)
            .ok_or_else(|| format!("missing {} folds", scenario.label))?;
        let mut scenario_pooled = BTreeMap::new();
        for (fold_name, by_symbol) in by_fold {
            let mut metrics = Vec::with_capacity(symbols.len());
            for symbol in symbols {
                metrics.push(by_symbol.get(symbol).cloned().ok_or_else(|| {
                    format!("missing {} {fold_name} for {symbol}", scenario.label)
                })?);
            }
            scenario_pooled.insert(fold_name.clone(), pool(&metrics));
        }
        pooled.insert(scenario.label.clone(), scenario_pooled);
    }
    Ok(pooled)
}

fn scenario_cost(scenario: &CostScenario) -> Decimal {
    scenario.fee_bps + scenario.slip_bps
}

fn break_even_points(
    scenarios: &[CostScenario],
    pooled: &BTreeMap<String, PartitionMetrics>,
) -> Result<Vec<(Decimal, Decimal)>, String> {
    scenarios
        .iter()
        .map(|scenario| {
            let metrics = pooled
                .get(&scenario.label)
                .ok_or_else(|| format!("missing {} metrics", scenario.label))?;
            Ok((scenario_cost(scenario), metrics.net_expectancy))
        })
        .collect()
}

fn gross_at_zero(
    scenarios: &[CostScenario],
    pooled: &BTreeMap<String, PartitionMetrics>,
) -> Result<Decimal, String> {
    let first = scenarios
        .first()
        .ok_or_else(|| "no cost scenarios were defined".to_string())?;
    pooled
        .get(&first.label)
        .map(|metrics| metrics.net_expectancy)
        .ok_or_else(|| format!("missing {} metrics", first.label))
}

fn build_report(
    runs: &[CandidateRun],
    scenarios: &[CostScenario],
    symbols: &[String],
) -> Result<(Vec<CostRow>, Vec<CandidateReport>), String> {
    let mut rows = Vec::new();
    let mut reports = Vec::with_capacity(runs.len());
    for run in runs {
        let validation_pooled = pool_by_scenario(&run.validation, scenarios, symbols)?;
        let holdout_pooled = pool_by_scenario(&run.holdout, scenarios, symbols)?;
        let folds_pooled = pool_folds(&run.folds, scenarios, symbols)?;

        for scenario in scenarios {
            let cost = scenario_cost(scenario);
            let validation = validation_pooled
                .get(&scenario.label)
                .ok_or_else(|| format!("missing pooled {} validation", scenario.label))?;
            let holdout = holdout_pooled
                .get(&scenario.label)
                .ok_or_else(|| format!("missing pooled {} holdout", scenario.label))?;
            rows.push(CostRow::new(
                &run.label,
                cost,
                "validation",
                "pooled",
                validation,
            ));
            rows.push(CostRow::new(&run.label, cost, "holdout", "pooled", holdout));
            for symbol in symbols {
                let validation_metrics = run
                    .validation
                    .get(&scenario.label)
                    .and_then(|by_symbol| by_symbol.get(symbol))
                    .ok_or_else(|| {
                        format!(
                            "{} {symbol}: missing {} validation",
                            run.label, scenario.label
                        )
                    })?;
                rows.push(CostRow::new(
                    &run.label,
                    cost,
                    "validation",
                    symbol,
                    validation_metrics,
                ));
                let holdout_metrics = run
                    .holdout
                    .get(&scenario.label)
                    .and_then(|by_symbol| by_symbol.get(symbol))
                    .ok_or_else(|| {
                        format!("{} {symbol}: missing {} holdout", run.label, scenario.label)
                    })?;
                rows.push(CostRow::new(
                    &run.label,
                    cost,
                    "holdout",
                    symbol,
                    holdout_metrics,
                ));
            }
            let by_fold = folds_pooled
                .get(&scenario.label)
                .ok_or_else(|| format!("missing pooled {} folds", scenario.label))?;
            for (fold_name, fold_pooled) in by_fold {
                rows.push(CostRow::new(
                    &run.label,
                    cost,
                    fold_name,
                    "pooled",
                    fold_pooled,
                ));
                let by_symbol = run
                    .folds
                    .get(&scenario.label)
                    .and_then(|folds| folds.get(fold_name))
                    .ok_or_else(|| {
                        format!("{}: missing {} {fold_name}", run.label, scenario.label)
                    })?;
                for symbol in symbols {
                    let fold_metrics = by_symbol.get(symbol).ok_or_else(|| {
                        format!(
                            "{} {symbol}: missing {} {fold_name}",
                            run.label, scenario.label
                        )
                    })?;
                    rows.push(CostRow::new(
                        &run.label,
                        cost,
                        fold_name,
                        symbol,
                        fold_metrics,
                    ));
                }
            }
        }

        let validation_points = break_even_points(scenarios, &validation_pooled)?;
        let holdout_points = break_even_points(scenarios, &holdout_pooled)?;
        let summary = ConfigCostSummary {
            label: run.label.clone(),
            break_even_validation_bps: break_even_bps(&validation_points),
            break_even_holdout_bps: break_even_bps(&holdout_points),
            gross_expectancy_validation_bps0: gross_at_zero(scenarios, &validation_pooled)?,
            gross_expectancy_holdout_bps0: gross_at_zero(scenarios, &holdout_pooled)?,
        };
        reports.push(CandidateReport {
            label: run.label.clone(),
            validation_pooled,
            holdout_pooled,
            folds_pooled,
            per_symbol_validation: run.validation.clone(),
            per_symbol_holdout: run.holdout.clone(),
            summary,
        });
    }
    Ok((rows, reports))
}

const CSV_COLUMNS: [&str; 15] = [
    "label",
    "cost_bps_per_side",
    "partition",
    "symbol_or_pooled",
    "trades",
    "gross_expectancy",
    "net_expectancy",
    "net_expectancy_r",
    "net_profit_factor",
    "win_rate",
    "max_drawdown_pct",
    "expectancy_stderr",
    "closed_net_pnl",
    "open_net_pnl",
    "net_pnl",
];

fn write_csv(path: &Path, rows: &[CostRow]) -> Result<(), String> {
    ensure_parent(path)?;
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(CSV_COLUMNS)
        .map_err(|error| error.to_string())?;
    for row in rows {
        let record = [
            row.label.clone(),
            row.cost_bps_per_side.to_string(),
            row.partition.clone(),
            row.symbol_or_pooled.clone(),
            row.trades.to_string(),
            row.gross_expectancy.to_string(),
            row.net_expectancy.to_string(),
            row.net_expectancy_r.to_string(),
            row.net_profit_factor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.win_rate.to_string(),
            row.max_drawdown_pct.to_string(),
            row.expectancy_stderr.to_string(),
            row.closed_net_pnl.to_string(),
            row.open_net_pnl.to_string(),
            row.net_pnl.to_string(),
        ];
        writer
            .write_record(record)
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_json(path: &Path, results: &CostLadderResults) -> Result<(), String> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn format_break_even(value: Option<Decimal>) -> String {
    match value {
        None => "> 30".to_string(),
        Some(value) if value == Decimal::ZERO => "0 (no gross edge)".to_string(),
        Some(value) => value.to_string(),
    }
}

fn format_profit_factor(value: Option<Decimal>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn write_markdown(
    path: &Path,
    cli: &Cli,
    data_dir_display: &str,
    symbols: &[String],
    scenarios: &[CostScenario],
    reports: &[CandidateReport],
) -> Result<(), String> {
    ensure_parent(path)?;
    let mut out = String::new();
    writeln!(out, "# Analyzer 4hour Cost Ladder").expect("writing to String cannot fail");
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
    writeln!(out, "| Data dir | {data_dir_display} |").expect("writing to String cannot fail");
    writeln!(out, "| Settle (ms) | {} |", cli.settle_ms).expect("writing to String cannot fail");
    writeln!(out, "| Init fund | {} |", cli.init_fund).expect("writing to String cannot fail");
    let levels = scenarios
        .iter()
        .map(|scenario| scenario_cost(scenario).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "| Levels (bps per side) | {levels} |").expect("writing to String cannot fail");
    let labels = reports
        .iter()
        .map(|report| report.label.clone())
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "| Candidates | {labels} |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Selection | Winners were selected on validation, so validation is in-sample; holdout is out-of-sample. |"
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    write_partition_table(
        &mut out,
        "2. Validation (in-sample, pooled across symbols)",
        "validation",
        scenarios,
        reports,
    );
    write_partition_table(
        &mut out,
        "3. Holdout (out-of-sample, pooled across symbols)",
        "holdout",
        scenarios,
        reports,
    );
    write_break_even_table(&mut out, reports);
    write_fraction_positive_table(&mut out, symbols, scenarios, reports);
    write_per_symbol_table(
        &mut out,
        "6. Per-symbol validation net expectancy (in-sample)",
        false,
        symbols,
        scenarios,
        reports,
    );
    write_per_symbol_table(
        &mut out,
        "7. Per-symbol holdout net expectancy (out-of-sample)",
        true,
        symbols,
        scenarios,
        reports,
    );
    write_interpretation(&mut out, reports);

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn write_partition_table(
    out: &mut String,
    title: &str,
    partition: &str,
    scenarios: &[CostScenario],
    reports: &[CandidateReport],
) {
    writeln!(out, "## {title}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Config | Cost (bps/side) | Net expectancy | PF | Trades | Max DD % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|").expect("writing to String cannot fail");
    for report in reports {
        let pooled = if partition == "validation" {
            &report.validation_pooled
        } else {
            &report.holdout_pooled
        };
        for scenario in scenarios {
            let Some(metrics) = pooled.get(&scenario.label) else {
                continue;
            };
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                report.label,
                scenario_cost(scenario),
                metrics.net_expectancy,
                format_profit_factor(metrics.net_profit_factor),
                metrics.trades,
                metrics.max_drawdown_pct
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_break_even_table(out: &mut String, reports: &[CandidateReport]) {
    writeln!(out, "## 4. Break-even per-side cost").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "`> 30` means pooled net expectancy was still positive at the highest level."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Config | 0bps validation gross | Validation break-even (bps/side) | 0bps holdout gross | Holdout break-even (bps/side) |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|").expect("writing to String cannot fail");
    for report in reports {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            report.label,
            report.summary.gross_expectancy_validation_bps0,
            format_break_even(report.summary.break_even_validation_bps),
            report.summary.gross_expectancy_holdout_bps0,
            format_break_even(report.summary.break_even_holdout_bps)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_fraction_positive_table(
    out: &mut String,
    symbols: &[String],
    scenarios: &[CostScenario],
    reports: &[CandidateReport],
) {
    writeln!(out, "## 5. Fraction positive (validation, in-sample)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let levels = [
        Decimal::ZERO,
        Decimal::from(5),
        Decimal::from(10),
        Decimal::from(15),
        Decimal::from(30),
    ];
    let scenario_labels: Vec<Option<&str>> = levels
        .iter()
        .map(|level| {
            scenarios
                .iter()
                .find(|scenario| scenario_cost(scenario) == *level)
                .map(|scenario| scenario.label.as_str())
        })
        .collect();
    writeln!(
        out,
        "| Config | Cost (bps/side) | Symbols positive | Folds positive |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|").expect("writing to String cannot fail");
    for report in reports {
        for (level, label) in levels.iter().zip(&scenario_labels) {
            let Some(label) = label else {
                continue;
            };
            let symbols_positive = symbols
                .iter()
                .filter(|symbol| {
                    report
                        .per_symbol_validation
                        .get(*label)
                        .and_then(|by_symbol| by_symbol.get(*symbol))
                        .is_some_and(|metrics| metrics.net_expectancy > Decimal::ZERO)
                })
                .count();
            let folds = report.folds_pooled.get(*label);
            let folds_positive = folds
                .map(|by_fold| {
                    by_fold
                        .values()
                        .filter(|metrics| metrics.net_expectancy > Decimal::ZERO)
                        .count()
                })
                .unwrap_or(0);
            let folds_total = folds.map(|by_fold| by_fold.len()).unwrap_or(0);
            writeln!(
                out,
                "| {} | {} | {symbols_positive}/{} | {folds_positive}/{folds_total} |",
                report.label,
                level,
                symbols.len()
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_per_symbol_table(
    out: &mut String,
    title: &str,
    holdout: bool,
    symbols: &[String],
    scenarios: &[CostScenario],
    reports: &[CandidateReport],
) {
    writeln!(out, "## {title}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let levels = [
        Decimal::ZERO,
        Decimal::from(5),
        Decimal::from(10),
        Decimal::from(15),
        Decimal::from(30),
    ];
    let scenario_labels: Vec<Option<&str>> = levels
        .iter()
        .map(|level| {
            scenarios
                .iter()
                .find(|scenario| scenario_cost(scenario) == *level)
                .map(|scenario| scenario.label.as_str())
        })
        .collect();
    writeln!(
        out,
        "| Config | Symbol | 0bps | 5bps | 10bps | 15bps | 30bps |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|").expect("writing to String cannot fail");
    for report in reports {
        let by_scenario = if holdout {
            &report.per_symbol_holdout
        } else {
            &report.per_symbol_validation
        };
        for symbol in symbols {
            write!(out, "| {} | {symbol} |", report.label).expect("writing to String cannot fail");
            for label in &scenario_labels {
                let value = label
                    .and_then(|label| by_scenario.get(label))
                    .and_then(|by_symbol| by_symbol.get(symbol))
                    .map(|metrics| metrics.net_expectancy.to_string())
                    .unwrap_or_else(|| "-".to_string());
                write!(out, " {value} |").expect("writing to String cannot fail");
            }
            writeln!(out).expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

#[derive(Debug)]
struct Verdict {
    label: String,
    decision: String,
    rationale: String,
    /// Outer `None` means no config had positive gross expectancy. Inner `None` means the
    /// best break-even was still positive at the highest cost level (reported as `> 30`).
    best: Option<Option<Decimal>>,
}

fn best_break_even(reports: &[CandidateReport], holdout: bool) -> Option<Option<Decimal>> {
    let mut best: Option<Option<Decimal>> = None;
    for report in reports {
        let (gross, break_even) = if holdout {
            (
                report.summary.gross_expectancy_holdout_bps0,
                report.summary.break_even_holdout_bps,
            )
        } else {
            (
                report.summary.gross_expectancy_validation_bps0,
                report.summary.break_even_validation_bps,
            )
        };
        if gross <= Decimal::ZERO {
            continue;
        }
        best = Some(match best {
            None => break_even,
            Some(None) => None,
            Some(Some(current)) => break_even.map(|value| current.max(value)),
        });
    }
    best
}

fn format_best_break_even(best: &Option<Option<Decimal>>) -> String {
    match best {
        None => "-".to_string(),
        Some(None) => "> 30".to_string(),
        Some(Some(value)) => value.round_dp(2).to_string(),
    }
}

fn best_break_even_value(best: &Option<Option<Decimal>>) -> Option<Decimal> {
    match best {
        Some(Some(value)) => Some(*value),
        _ => None,
    }
}

fn verdict(reports: &[CandidateReport], holdout: bool) -> Verdict {
    let partition = if holdout {
        "out-of-sample"
    } else {
        "in-sample"
    };
    let best = best_break_even(reports, holdout);

    if reports.is_empty() {
        return Verdict {
            label: "insufficient evidence".to_string(),
            decision: "insufficient evidence".to_string(),
            rationale: "No configs were evaluated, so no conclusion can be drawn.".to_string(),
            best,
        };
    }

    let survival = || {
        Verdict {
        label: "gross edge may survive realistic costs; proceed to excursion/execution tests"
            .to_string(),
        decision: "proceed to excursion/execution tests".to_string(),
        rationale: format!(
            "At least one config has positive gross expectancy {partition} with a best break-even of 10 bps per side or higher (or still positive at 30 bps). This warrants the excursion and execution experiments before drawing any profitability conclusion."
        ),
        best,
    }
    };

    match &best {
        None => Verdict {
            label: format!("no gross edge {partition}"),
            decision: format!("no gross edge {partition}"),
            rationale: format!(
                "No config has positive 0 bps {partition} net expectancy, so there is no gross edge to preserve. Stop tuning this signal and move to signal/regime redesign."
            ),
            best,
        },
        Some(None) => survival(),
        Some(Some(value)) if *value < Decimal::from(10) => Verdict {
            label: format!(
                "gross edge exists {partition} but is below realistic execution costs; no viable path without execution changes"
            ),
            decision: "no viable path without execution changes".to_string(),
            rationale: format!(
                "At least one config has positive gross expectancy {partition}, but the best break-even is below 10 bps per side. Without execution changes this is unlikely to produce a viable net edge."
            ),
            best,
        },
        Some(Some(_)) => survival(),
    }
}

fn write_interpretation(out: &mut String, reports: &[CandidateReport]) {
    writeln!(out, "## 8. Interpretation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if reports.is_empty() {
        writeln!(out, "**insufficient evidence**").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "No configs were evaluated, so no conclusion can be drawn."
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        return;
    }
    let validation = verdict(reports, false);
    let holdout = verdict(reports, true);
    writeln!(
        out,
        "**Validation (in-sample): {}. Holdout (out-of-sample): {} (best break-even {} bps/side). Decision: {}.**",
        validation.label,
        holdout.label,
        format_best_break_even(&holdout.best),
        holdout.decision
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The three winners were selected on validation, so the validation result is in-sample. Only the holdout is out-of-sample evidence."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Validation best gross-positive break-even: {} bps/side (in-sample).",
        format_best_break_even(&validation.best)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Holdout best gross-positive break-even: {} bps/side (out-of-sample).",
        format_best_break_even(&holdout.best)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Validation rationale (in-sample):** {}",
        validation.rationale
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Holdout rationale (out-of-sample):** {}",
        holdout.rationale
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn print_summary(reports: &[CandidateReport]) {
    println!(
        "cost ladder completed for {} config(s) across {} cost levels",
        reports.len(),
        reports
            .first()
            .map(|report| report.validation_pooled.len())
            .unwrap_or(0)
    );
    for report in reports {
        println!(
            "  {}: validation gross@0bps={} break-even={} holdout gross@0bps={} break-even={}",
            report.label,
            report.summary.gross_expectancy_validation_bps0,
            format_break_even(report.summary.break_even_validation_bps),
            report.summary.gross_expectancy_holdout_bps0,
            format_break_even(report.summary.break_even_holdout_bps)
        );
    }
    let validation = verdict(reports, false);
    let holdout = verdict(reports, true);
    println!("  validation (in-sample): {}", validation.label);
    println!(
        "  holdout (out-of-sample): {} (best break-even {} bps/side)",
        holdout.label,
        format_best_break_even(&holdout.best)
    );
    println!("  decision: {}", holdout.decision);
}

fn main() -> Result<(), String> {
    load_dotenv();
    share::logger::init_logger();
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

    let scenarios = scenarios();
    let candidates = frozen_candidates();
    let datasets = load_datasets(&symbols, &cli.data_dir, cli.as_of, cli.settle_ms)?;
    let runs = evaluate_candidates(&candidates, &symbols, &datasets, &scenarios, cli.init_fund)?;
    let (rows, reports) = build_report(&runs, &scenarios, &symbols)?;

    let data_dir_display = data_dir_display(&cli.data_dir);
    let validation = verdict(&reports, false);
    let holdout = verdict(&reports, true);
    let results = CostLadderResults {
        generated_at: cli.generated_at.clone(),
        as_of_ms: cli.as_of,
        symbols: symbols.clone(),
        data_dir: data_dir_display.clone(),
        data_dir_display: data_dir_display.clone(),
        settle_ms: cli.settle_ms,
        init_fund: cli.init_fund,
        levels_bps_per_side: scenarios.iter().map(scenario_cost).collect(),
        datasets: dataset_summaries(&datasets),
        rows,
        configs: reports
            .iter()
            .map(|report| report.summary.clone())
            .collect(),
        verdicts: VerdictSummary {
            validation_verdict: format!("in-sample: {}", validation.label),
            validation_best_break_even_bps: best_break_even_value(&validation.best),
            validation_best_break_even_display: format_best_break_even(&validation.best),
            holdout_verdict: format!("out-of-sample: {}", holdout.label),
            best_holdout_break_even_bps: best_break_even_value(&holdout.best),
            best_holdout_break_even_display: format_best_break_even(&holdout.best),
            decision: holdout.decision.clone(),
        },
        selection_note: "Three winners were selected on validation, so validation is in-sample; only the holdout is out-of-sample.".to_string(),
    };

    let csv_path = cli.out_dir.join("cost_ladder.csv");
    let json_path = cli.out_dir.join("results.json");
    let report_path = cli.out_dir.join("report.md");
    write_csv(&csv_path, &results.rows)?;
    write_json(&json_path, &results)?;
    write_markdown(
        &report_path,
        &cli,
        &data_dir_display,
        &symbols,
        &scenarios,
        &reports,
    )?;

    println!("wrote {}", csv_path.display());
    println!("wrote {}", json_path.display());
    println!("wrote {}", report_path.display());
    print_summary(&reports);

    Ok(())
}
