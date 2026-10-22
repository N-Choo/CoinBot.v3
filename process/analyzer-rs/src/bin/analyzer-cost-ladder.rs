use analyzer_rs::config_env::load_dotenv;
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, Dataset};
use analyzer_rs::harness::sweep::cost_ladder::{break_even_bps, frozen_candidates, scenarios};
use analyzer_rs::harness::sweep::costs::CostScenario;
use analyzer_rs::harness::sweep::evaluate::{evaluate_config, pool, PartitionMetrics};
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
    #[arg(long, default_value = "out/cost-ladder")]
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
struct CostLadderResults {
    generated_at: String,
    as_of_ms: i64,
    symbols: Vec<String>,
    data_dir: String,
    settle_ms: i64,
    init_fund: i64,
    levels_bps_per_side: Vec<Decimal>,
    rows: Vec<CostRow>,
    configs: Vec<ConfigCostSummary>,
}

struct CandidateRun {
    label: String,
    /// scenario label -> symbol -> metrics
    validation: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
    holdout: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
}

struct CandidateReport {
    label: String,
    /// scenario label -> pooled metrics
    validation_pooled: BTreeMap<String, PartitionMetrics>,
    holdout_pooled: BTreeMap<String, PartitionMetrics>,
    /// scenario label -> symbol -> metrics
    per_symbol_validation: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
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
        for symbol in symbols {
            let dataset = datasets
                .get(symbol)
                .ok_or_else(|| format!("missing dataset for {symbol}"))?;
            let splits = chronological_splits(dataset.candles.len());
            let result = evaluate_config(
                dataset,
                &candidate.to_config(),
                candidate.max_age,
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
            }
        }
        runs.push(CandidateRun {
            label: candidate.label.clone(),
            validation,
            holdout,
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
                let metrics = run
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
                    metrics,
                ));
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
            per_symbol_validation: run.validation.clone(),
            summary,
        });
    }
    Ok((rows, reports))
}

const CSV_COLUMNS: [&str; 14] = [
    "label",
    "cost_bps_per_side",
    "partition",
    "symbol_or_pooled",
    "trades",
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
    writeln!(out, "| Data dir | {} |", cli.data_dir.display())
        .expect("writing to String cannot fail");
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
    writeln!(out).expect("writing to String cannot fail");

    write_partition_table(
        &mut out,
        "2. Validation (pooled across symbols)",
        "validation",
        scenarios,
        reports,
    );
    write_partition_table(
        &mut out,
        "3. Holdout (pooled across symbols)",
        "holdout",
        scenarios,
        reports,
    );
    write_break_even_table(&mut out, reports);
    write_per_symbol_table(&mut out, symbols, scenarios, reports);
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

fn write_per_symbol_table(
    out: &mut String,
    symbols: &[String],
    scenarios: &[CostScenario],
    reports: &[CandidateReport],
) {
    writeln!(out, "## 5. Per-symbol validation net expectancy")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let levels = [
        Decimal::ZERO,
        Decimal::from(5),
        Decimal::from(10),
        Decimal::from(15),
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
    writeln!(out, "| Config | Symbol | 0bps | 5bps | 10bps | 15bps |")
        .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|").expect("writing to String cannot fail");
    for report in reports {
        for symbol in symbols {
            write!(out, "| {} | {symbol} |", report.label).expect("writing to String cannot fail");
            for label in &scenario_labels {
                let value = label
                    .and_then(|label| report.per_symbol_validation.get(label))
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

fn write_interpretation(out: &mut String, reports: &[CandidateReport]) {
    writeln!(out, "## 6. Interpretation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    let (label, rationale) = interpretation(reports);
    writeln!(out, "**{label}**").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "{rationale}").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn interpretation(reports: &[CandidateReport]) -> (&'static str, String) {
    if reports.is_empty() {
        return (
            "insufficient evidence",
            "No configs were evaluated, so no conclusion can be drawn.".to_string(),
        );
    }

    let mut best: Option<Decimal> = None;
    let mut still_positive_at_top = false;
    for report in reports {
        if report.summary.gross_expectancy_validation_bps0 <= Decimal::ZERO {
            continue;
        }
        match report.summary.break_even_validation_bps {
            Some(value) => best = Some(best.map_or(value, |current| current.max(value))),
            None => still_positive_at_top = true,
        }
    }

    if best.is_none() && !still_positive_at_top {
        return (
            "no gross edge",
            "Every frozen config has non-positive 0bps validation net expectancy, so there is no gross edge to preserve. Stop tuning this signal and move to signal/regime redesign."
                .to_string(),
        );
    }

    if !still_positive_at_top && best.is_some_and(|value| value < Decimal::from(10)) {
        return (
            "gross edge exists but likely below realistic execution costs",
            "At least one frozen config is gross-positive, but its validation break-even is below 10 bps per side. Without execution changes this is unlikely to produce a viable net edge."
                .to_string(),
        );
    }

    (
        "gross edge may survive realistic costs; proceed to excursion/execution tests",
        "At least one frozen config is gross-positive with a validation break-even of 10 bps per side or higher (or still positive at 30 bps). This warrants the excursion and execution experiments before drawing any profitability conclusion."
            .to_string(),
    )
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

    let scenarios = scenarios();
    let candidates = frozen_candidates();
    let datasets = load_datasets(&symbols, &cli.data_dir, cli.as_of, cli.settle_ms)?;
    let runs = evaluate_candidates(&candidates, &symbols, &datasets, &scenarios, cli.init_fund)?;
    let (rows, reports) = build_report(&runs, &scenarios, &symbols)?;

    let results = CostLadderResults {
        generated_at: cli.generated_at.clone(),
        as_of_ms: cli.as_of,
        symbols: symbols.clone(),
        data_dir: cli.data_dir.display().to_string(),
        settle_ms: cli.settle_ms,
        init_fund: cli.init_fund,
        levels_bps_per_side: scenarios.iter().map(scenario_cost).collect(),
        rows,
        configs: reports
            .iter()
            .map(|report| report.summary.clone())
            .collect(),
    };

    let csv_path = cli.out_dir.join("cost_ladder.csv");
    let json_path = cli.out_dir.join("results.json");
    let report_path = cli.out_dir.join("report.md");
    write_csv(&csv_path, &results.rows)?;
    write_json(&json_path, &results)?;
    write_markdown(&report_path, &cli, &symbols, &scenarios, &reports)?;

    println!("wrote {}", csv_path.display());
    println!("wrote {}", json_path.display());
    println!("wrote {}", report_path.display());
    print_summary(&reports);

    Ok(())
}
