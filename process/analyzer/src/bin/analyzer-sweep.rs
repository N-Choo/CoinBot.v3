use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, load_with_cache, write_cache};
use analyzer::harness::sweep::evaluate::{evaluate_config, pool, PartitionMetrics};
use analyzer::harness::sweep::partition::chronological_splits;
use analyzer::harness::sweep::ranking::{rank, CandidateOutcome, Thresholds};
use analyzer::harness::sweep::report::{
    write_leaderboard_csv, write_markdown, write_results_json, BaselineConfig, CandidatePlan,
    DatasetSummary, SplitSummary, SweepResults,
};
use analyzer::harness::sweep::sampling::{self, Candidate};
use analyzer::identity;
use analyzer::strategy::types::Candle;
use clap::Parser;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const WINDOW: usize = 100;
const MONTH_MS: i64 = 30 * 24 * 60 * 60 * 1000;
const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-sweep",
    about = "Screen analyzer hyperparameter candidates across symbols and timeframes"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,ETH-USDT,SOL-USDT")]
    symbols: String,

    /// Comma-separated KuCoin timeframes (15min, 1hour, 4hour, 1day).
    #[arg(long, default_value = "1hour,4hour")]
    timeframes: String,

    /// History window in 30-day months.
    #[arg(long, default_value_t = 24)]
    months: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now. Reproducible runs
    /// should pass a fixed value so artifacts are byte-identical.
    #[arg(long)]
    generated_at: Option<String>,

    /// Candle cache directory.
    #[arg(long, default_value = "data/sweep")]
    data_dir: PathBuf,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/sweep")]
    out_dir: PathBuf,

    /// Deterministic candidate-sampling seed.
    #[arg(long, default_value_t = sampling::SEED_DEFAULT)]
    seed: u64,

    /// Base fee in basis points per side.
    #[arg(long, default_value_t = 10)]
    fee_bps: u32,

    /// Base slippage in basis points per side.
    #[arg(long, default_value_t = 5)]
    slip_bps: u32,

    /// Stress fee in basis points per side.
    #[arg(long, default_value_t = 20)]
    stress_fee_bps: u32,

    /// Stress slippage in basis points per side.
    #[arg(long, default_value_t = 10)]
    stress_slip_bps: u32,

    /// Initial account fund per symbol.
    #[arg(long, default_value_t = 1000)]
    init_fund: i64,

    /// Refetch every symbol/timeframe instead of reusing the cache.
    #[arg(long)]
    refresh: bool,

    /// KuCoin API base URL; defaults to KUCOIN_BASE_URL, then https://api.kucoin.com.
    #[arg(long)]
    base_url: Option<String>,

    /// Candle settle time in milliseconds; defaults to ANALYZER_CANDLE_SETTLE_MS, then 5000.
    #[arg(long)]
    settle_ms: Option<i64>,
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

/// Matches `^[A-Z0-9]+-[A-Z0-9]+$` without pulling in a regex dependency.
fn valid_symbol(symbol: &str) -> bool {
    let mut parts = symbol.split('-');
    let valid_part = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    };
    match (parts.next(), parts.next(), parts.next()) {
        (Some(base), Some(quote), None) => valid_part(base) && valid_part(quote),
        _ => false,
    }
}

/// Loads filtered in-memory candles through the dataset validator using a
/// short-lived temp file. `load_csv` takes a path, so this bridges the two.
fn load_view_dataset(
    symbol: &str,
    timeframe: Timeframe,
    as_of: i64,
    settle_ms: i64,
    candles: &[Candle],
) -> Result<Dataset, String> {
    let path = std::env::temp_dir().join(format!(
        "analyzer-sweep-view-{}-{}-{}.csv",
        std::process::id(),
        symbol,
        timeframe.as_kucoin()
    ));
    write_cache(&path, candles)?;
    let result = load_csv(
        symbol,
        &path,
        timeframe,
        as_of,
        settle_ms,
        0,
        WINDOW,
        candles.len(),
    );
    let _ = std::fs::remove_file(&path);
    result.map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn load_symbol(
    symbol: &str,
    timeframe: Timeframe,
    as_of: i64,
    start_ms: i64,
    settle_ms: i64,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
) -> Result<(Dataset, DatasetSummary), String> {
    let cache_path = data_dir.join(format!("{symbol}_{}.csv", timeframe.as_kucoin()));
    // Reuse is decided solely by provenance matching the requested range; any
    // missing/mismatched provenance refetches even without `--refresh`.
    let cached = load_with_cache(
        &cache_path,
        symbol,
        timeframe,
        start_ms,
        as_of,
        refresh,
        settle_ms,
        WINDOW,
        || {
            fetch_range(
                base_url, symbol, timeframe, start_ms, as_of, settle_ms, as_of,
            )
        },
    )?;
    let data_provenance = cached.provenance;
    let dataset = load_view_dataset(symbol, timeframe, as_of, settle_ms, &cached.candles)?;
    let summary = DatasetSummary {
        symbol: symbol.to_string(),
        timeframe: timeframe.as_kucoin().to_string(),
        requested_first_ms: data_provenance.requested_first_ms,
        requested_last_ms: data_provenance.requested_last_ms,
        first_open_time_ms: dataset.provenance.effective_range.first_open_time_ms,
        last_open_time_ms: dataset.provenance.effective_range.last_open_time_ms,
        candles: dataset.candles.len(),
        requested_candles: data_provenance.requested_candles,
        expected_candles: data_provenance.expected_candles,
        missing_candles: data_provenance.missing_candles,
        duplicates_dropped: data_provenance.duplicates_dropped,
        pages: data_provenance.pages,
        fetched_at_ms: Some(data_provenance.fetched_at_ms),
        settle_ms: data_provenance.settle_ms,
        content_sha256: dataset.provenance.content_sha256.clone(),
    };
    Ok((dataset, summary))
}

fn pool_partition(
    maps: &[BTreeMap<String, PartitionMetrics>],
    name: &str,
) -> Option<PartitionMetrics> {
    if maps.is_empty() {
        return None;
    }
    let metrics: Vec<PartitionMetrics> = maps
        .iter()
        .map(|map| map.get(name).cloned())
        .collect::<Option<Vec<_>>>()?;
    Some(pool(&metrics))
}

/// Evaluates one candidate across every loaded symbol for a timeframe.
fn evaluate_candidate(
    timeframe: &str,
    candidate: &Candidate,
    datasets: &[Dataset],
    scenarios: &[CostScenario],
    init_fund: i64,
    include_holdout: bool,
) -> Result<CandidateOutcome, String> {
    let cfg = candidate.to_config();
    let mut base_maps = Vec::with_capacity(datasets.len());
    let mut stress_maps = Vec::with_capacity(datasets.len());
    let mut per_symbol_validation = Vec::with_capacity(datasets.len());

    for dataset in datasets {
        let splits = chronological_splits(dataset.candles.len());
        let result = evaluate_config(
            dataset,
            &cfg,
            &splits,
            WINDOW,
            init_fund,
            scenarios,
            include_holdout,
        )
        .map_err(|error| {
            format!(
                "{timeframe} {} {}: {error}",
                candidate.label, dataset.symbol
            )
        })?;
        let mut partitions = result.partitions;
        let missing = |scenario: &str| {
            format!(
                "{timeframe} {} {}: missing {scenario} scenario",
                candidate.label, dataset.symbol
            )
        };
        let base = partitions.remove("base").ok_or_else(|| missing("base"))?;
        let stress = partitions
            .remove("stress")
            .ok_or_else(|| missing("stress"))?;
        let validation = base.get("validation").cloned().ok_or_else(|| {
            format!(
                "{timeframe} {} {}: missing base validation",
                candidate.label, dataset.symbol
            )
        })?;
        per_symbol_validation.push((dataset.symbol.clone(), validation));
        base_maps.push(base);
        stress_maps.push(stress);
    }

    per_symbol_validation.sort_by(|a, b| a.0.cmp(&b.0));
    let validation_metrics: Vec<PartitionMetrics> = per_symbol_validation
        .iter()
        .map(|(_, metrics)| metrics.clone())
        .collect();
    let pooled_validation = pool(&validation_metrics);

    let mut base = BTreeMap::new();
    if let Some(first) = base_maps.first() {
        for name in first.keys() {
            let metrics = pool_partition(&base_maps, name).ok_or_else(|| {
                format!(
                    "{timeframe} {}: missing {name} in base scenario",
                    candidate.label
                )
            })?;
            base.insert(name.clone(), metrics);
        }
    }

    let mut stress = BTreeMap::new();
    if let Some(first) = stress_maps.first() {
        for name in first.keys() {
            let metrics = pool_partition(&stress_maps, name).ok_or_else(|| {
                format!(
                    "{timeframe} {}: missing {name} in stress scenario",
                    candidate.label
                )
            })?;
            stress.insert(name.clone(), metrics);
        }
    }

    let mut folds = Vec::with_capacity(3);
    for index in 1..=3 {
        let name = format!("fold{index}_val");
        let metrics = pool_partition(&base_maps, &name)
            .ok_or_else(|| format!("{timeframe} {}: missing {name}", candidate.label))?;
        folds.push(metrics);
    }

    Ok(CandidateOutcome {
        label: candidate.label.clone(),
        origin: candidate.origin.clone(),
        candidate: candidate.clone(),
        pooled_validation,
        per_symbol_validation,
        folds,
        base,
        stress,
    })
}

fn print_summary(results: &SweepResults) {
    println!(
        "sweep completed for {} timeframe(s) across {} symbol(s)",
        results.timeframes.len(),
        results.symbols.len()
    );
    for timeframe in &results.timeframes {
        match results.selections.get(timeframe) {
            Some(selection) if !selection.winners.is_empty() => {
                println!("{timeframe}: winners: {}", selection.winners.join(", "));
            }
            _ => println!("{timeframe}: no candidate meets the criteria"),
        }
        let baseline = results
            .outcomes
            .get(timeframe)
            .and_then(|outcomes| outcomes.iter().find(|outcome| outcome.label == "baseline"));
        if let Some(baseline) = baseline {
            let validation = &baseline.pooled_validation;
            println!(
                "  baseline validation: trades={} net_expectancy={} net_pnl={} pf={} max_dd_pct={}",
                validation.trades,
                validation.net_expectancy,
                validation.net_pnl,
                validation
                    .net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                validation.max_drawdown_pct
            );
        }
    }
    if !results.errors.is_empty() {
        println!(
            "{} error(s) recorded; see report.md and results.json",
            results.errors.len()
        );
    }
}

fn main() -> Result<(), String> {
    load_dotenv();
    share::logger::init_logger();
    let cli = Cli::parse();

    let symbols = parse_list(&cli.symbols);
    if symbols.is_empty() {
        return Err("--symbols resolved to an empty list".into());
    }
    for symbol in &symbols {
        if !valid_symbol(symbol) {
            return Err(format!(
                "invalid symbol {symbol:?}; expected BASE-QUOTE matching ^[A-Z0-9]+-[A-Z0-9]+$"
            ));
        }
    }
    if cli.months == 0 {
        return Err("--months must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    for (name, value) in [
        ("--fee-bps", cli.fee_bps),
        ("--slip-bps", cli.slip_bps),
        ("--stress-fee-bps", cli.stress_fee_bps),
        ("--stress-slip-bps", cli.stress_slip_bps),
    ] {
        if value > 1000 {
            return Err(format!("{name} must be at most 1000"));
        }
    }
    if let Some(value) = &cli.generated_at {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
            format!("invalid --generated-at {value:?}; expected an RFC 3339 timestamp")
        })?;
    }

    let mut timeframes: Vec<(String, Timeframe)> = Vec::new();
    for name in parse_list(&cli.timeframes) {
        let timeframe = Timeframe::from_kucoin(&name).ok_or_else(|| {
            format!("unsupported timeframe {name:?}; expected 15min, 1hour, 4hour, or 1day")
        })?;
        let canonical = timeframe.as_kucoin().to_string();
        if !timeframes
            .iter()
            .any(|(existing, _)| existing == &canonical)
        {
            timeframes.push((canonical, timeframe));
        }
    }
    if timeframes.is_empty() {
        return Err("--timeframes resolved to an empty list".into());
    }

    let base_url = cli
        .base_url
        .clone()
        .or_else(|| env_string("KUCOIN_BASE_URL"))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let settle_ms = match cli.settle_ms {
        Some(value) => value,
        None => match env_string("ANALYZER_CANDLE_SETTLE_MS") {
            Some(value) => value.parse::<i64>().map_err(|_| {
                format!("invalid ANALYZER_CANDLE_SETTLE_MS={value:?}: expected milliseconds")
            })?,
            None => DEFAULT_SETTLE_MS,
        },
    };
    if settle_ms < 0 {
        return Err("--settle-ms must be non-negative".into());
    }
    let start_ms = cli
        .as_of
        .saturating_sub(i64::from(cli.months).saturating_mul(MONTH_MS));

    let scenarios = [
        CostScenario {
            label: "base".to_string(),
            fee_bps: Decimal::from(cli.fee_bps),
            slip_bps: Decimal::from(cli.slip_bps),
        },
        CostScenario {
            label: "stress".to_string(),
            fee_bps: Decimal::from(cli.stress_fee_bps),
            slip_bps: Decimal::from(cli.stress_slip_bps),
        },
    ];

    let mut loaded: BTreeMap<String, Vec<Dataset>> = BTreeMap::new();
    let mut datasets = Vec::new();
    let mut splits = Vec::new();
    let mut errors = Vec::new();
    for (timeframe_name, timeframe) in &timeframes {
        for symbol in &symbols {
            match load_symbol(
                symbol,
                *timeframe,
                cli.as_of,
                start_ms,
                settle_ms,
                &cli.data_dir,
                &base_url,
                cli.refresh,
            ) {
                Ok((dataset, summary)) => {
                    splits.push(SplitSummary::from_candles(
                        &dataset.symbol,
                        timeframe_name,
                        &dataset.candles,
                    ));
                    loaded
                        .entry(timeframe_name.clone())
                        .or_default()
                        .push(dataset);
                    datasets.push(summary);
                }
                Err(error) => errors.push(format!("{timeframe_name} {symbol}: {error}")),
            }
        }
    }
    for timeframe_datasets in loaded.values_mut() {
        timeframe_datasets.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    }

    let thresholds = Thresholds::default();
    let mut outcomes_by_timeframe = BTreeMap::new();
    let mut selections = BTreeMap::new();
    let mut candidate_plan = BTreeMap::new();
    for (timeframe_name, _) in &timeframes {
        let candidates = sampling::generate(cli.seed);
        candidate_plan.insert(
            timeframe_name.clone(),
            CandidatePlan {
                seed: cli.seed,
                candidates: candidates.clone(),
            },
        );

        let mut outcomes = Vec::new();
        if let Some(timeframe_datasets) = loaded.get(timeframe_name) {
            // First pass scores every candidate without the holdout so it cannot
            // influence the validation-based ranking.
            for candidate in &candidates {
                match evaluate_candidate(
                    timeframe_name,
                    candidate,
                    timeframe_datasets,
                    &scenarios,
                    cli.init_fund,
                    false,
                ) {
                    Ok(outcome) => outcomes.push(outcome),
                    Err(error) => {
                        errors.push(format!("{timeframe_name} {}: {error}", candidate.label))
                    }
                }
            }
        }
        outcomes.sort_by(|a, b| a.label.cmp(&b.label));
        let selection = rank(&outcomes, &thresholds);

        // Only the frozen set (baseline + selected winners) is scored on the
        // one-shot holdout partition, and those outcomes replace the first pass.
        if let Some(timeframe_datasets) = loaded.get(timeframe_name) {
            let mut frozen = vec!["baseline".to_string()];
            for winner in &selection.winners {
                if !frozen.contains(winner) {
                    frozen.push(winner.clone());
                }
            }
            for label in frozen {
                let Some(candidate) = candidates.iter().find(|candidate| candidate.label == label)
                else {
                    errors.push(format!(
                        "{timeframe_name}: frozen candidate {label} was not generated"
                    ));
                    continue;
                };
                match evaluate_candidate(
                    timeframe_name,
                    candidate,
                    timeframe_datasets,
                    &scenarios,
                    cli.init_fund,
                    true,
                ) {
                    Ok(outcome) => {
                        if let Some(slot) =
                            outcomes.iter_mut().find(|existing| existing.label == label)
                        {
                            *slot = outcome;
                        } else {
                            outcomes.push(outcome);
                        }
                    }
                    Err(error) => {
                        errors.push(format!("{timeframe_name} {label}: {error}"));
                    }
                }
            }
        }
        outcomes.sort_by(|a, b| a.label.cmp(&b.label));
        selections.insert(timeframe_name.clone(), selection);
        outcomes_by_timeframe.insert(timeframe_name.clone(), outcomes);
    }

    let baseline = sampling::baseline();
    let results = SweepResults {
        generated_at: cli
            .generated_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        as_of_ms: cli.as_of,
        months: cli.months,
        seed: cli.seed,
        symbols,
        timeframes: timeframes.iter().map(|(name, _)| name.clone()).collect(),
        fee_bps: cli.fee_bps,
        slip_bps: cli.slip_bps,
        stress_fee_bps: cli.stress_fee_bps,
        stress_slip_bps: cli.stress_slip_bps,
        init_fund: cli.init_fund,
        datasets,
        splits,
        baseline_config: BaselineConfig::from_candidate(&baseline),
        baseline,
        baseline_source: "pinned per screening design decision 5; not read from .env".to_string(),
        outcomes: outcomes_by_timeframe,
        selections,
        errors,
        thresholds,
        candidate_plan,
    };

    let leaderboard = cli.out_dir.join("leaderboard.csv");
    let results_json = cli.out_dir.join("results.json");
    let report = cli.out_dir.join("report.md");
    write_leaderboard_csv(&leaderboard, &results)?;
    write_results_json(&results_json, &results)?;
    write_markdown(&report, &results)?;

    println!("wrote {}", leaderboard.display());
    println!("wrote {}", results_json.display());
    println!("wrote {}", report.display());
    print_summary(&results);

    if results.datasets.is_empty() {
        return Err("no dataset succeeded; see the recorded errors above".into());
    }
    Ok(())
}
