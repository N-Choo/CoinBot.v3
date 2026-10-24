use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::experiment::{evaluate_symbol, ExperimentParams, SymbolExperiment};
use analyzer::harness::experiment_report::{write_combined_index, write_symbol_report};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, write_cache};
use analyzer::identity;
use analyzer::strategy::types::Candle;
use clap::Parser;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MIN_CANDLES: usize = 1000;
const REGIME_SMA_PERIOD: usize = 200;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-tuning",
    about = "Per-symbol RSI tuning with an M5 holdout and a standardized report"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,BNB-USDT,XRP-USDT,SOL-USDT")]
    symbols: String,

    /// Timeframe; this experiment supports 4hour only.
    #[arg(long, default_value = "4hour")]
    timeframe: String,

    /// History window in days.
    #[arg(long, default_value_t = 365)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/rsi-tuning-1y")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/tuning")]
    data_dir: PathBuf,

    /// Refetch every symbol instead of reusing the cache.
    #[arg(long)]
    refresh: bool,

    /// Initial account fund per symbol.
    #[arg(long, default_value_t = 1000)]
    init_fund: i64,

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

    /// Number of development candidates frozen for holdout evaluation.
    #[arg(long, default_value_t = 3)]
    top_n: usize,

    /// Maximum development drawdown percentage.
    #[arg(long, default_value_t = 20)]
    max_dd_pct: u32,
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

fn cache_path(data_dir: &Path, symbol: &str, timeframe: Timeframe) -> PathBuf {
    data_dir.join(format!("{symbol}_{}.csv", timeframe.as_kucoin()))
}

fn dataset_from_cache(
    symbol: &str,
    path: &Path,
    timeframe: Timeframe,
    as_of: i64,
    settle_ms: i64,
    window: usize,
) -> Result<Dataset, String> {
    load_csv(symbol, path, timeframe, as_of, settle_ms, 0, window, 0)
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn load_symbol(
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    as_of: i64,
    settle_ms: i64,
    window: usize,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
) -> Result<Dataset, String> {
    let path = cache_path(data_dir, symbol, timeframe);
    if !refresh && path.is_file() {
        match dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms, window) {
            Ok(dataset) if dataset.candles.len() >= MIN_CANDLES => {
                println!(
                    "{}: reusing cache, {} closed candles",
                    symbol,
                    dataset.candles.len()
                );
                return Ok(dataset);
            }
            Ok(dataset) => {
                println!(
                    "{}: cache has only {} closed candles; refetching",
                    symbol,
                    dataset.candles.len()
                );
            }
            Err(error) => {
                println!("{}: cache unusable ({error}); refetching", symbol);
            }
        }
    }

    let (candles, pages, duplicates) = fetch_range(
        base_url, symbol, timeframe, start_ms, as_of, settle_ms, as_of,
    )?;
    println!(
        "{}: fetched {} closed candles in {} page(s), {} duplicates dropped",
        symbol,
        candles.len(),
        pages,
        duplicates
    );
    write_cache(&path, &candles)?;
    dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms, window)
}

/// Builds an open-time keyed map of `close > 200-period simple SMA`.
/// Times before the SMA is defined are omitted and therefore report as unknown.
fn btc_regime_map(candles: &[Candle], period: usize) -> BTreeMap<i64, bool> {
    let mut map = BTreeMap::new();
    if period == 0 || candles.len() < period {
        return map;
    }
    let closes: Vec<Decimal> = candles.iter().map(|candle| candle.close).collect();
    let mut sum: Decimal = closes[..period].iter().copied().sum();
    for index in (period - 1)..candles.len() {
        if index > period - 1 {
            sum += closes[index];
            sum -= closes[index - period];
        }
        let sma = sum / Decimal::from(period);
        map.insert(candles[index].open_time_ms, closes[index] > sma);
    }
    map
}

fn expected_candles(days: u32, timeframe: Timeframe) -> i64 {
    i64::from(days) * DAY_MS / timeframe.millis()
}

fn print_summary(experiments: &[SymbolExperiment], errors: &[String], generated_at: &str) {
    println!();
    println!(
        "tuning completed for {} symbol(s); generated_at={generated_at}",
        experiments.len()
    );
    for experiment in experiments {
        println!(
            "{}: candles={} top={} verdict={}",
            experiment.symbol,
            experiment.dataset.candles.len(),
            experiment.top.len(),
            experiment.decision.verdict.as_str()
        );
        for (index, candidate) in experiment.top.iter().enumerate() {
            let first = &candidate.first_oos.metrics;
            let extended = &candidate.extended_oos.metrics;
            println!(
                "  top-{} {}: M5_trades={} M5_expectancy={} M5_pf={} M6-M10_expectancy={} M5={}",
                index + 1,
                candidate.candidate.label,
                first.trades,
                first.net_expectancy,
                first
                    .net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "inf".to_string()),
                extended.net_expectancy,
                if candidate.m5_pass { "pass" } else { "fail" }
            );
        }
    }
    if !errors.is_empty() {
        println!("{} data issue(s):", errors.len());
        for error in errors {
            println!("  - {error}");
        }
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

    let timeframe = Timeframe::from_kucoin(&cli.timeframe)
        .ok_or_else(|| format!("unsupported timeframe {:?}", cli.timeframe))?;
    if timeframe != Timeframe::H4 {
        return Err("--timeframe must be 4hour for this experiment".into());
    }
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.max_dd_pct > 100 {
        return Err("--max-dd-pct must be at most 100".into());
    }
    if cli.top_n == 0 {
        return Err("--top-n must be at least 1".into());
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
    let generated_at = match &cli.generated_at {
        Some(value) => {
            chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
                format!("invalid --generated-at {value:?}; expected an RFC 3339 timestamp")
            })?;
            value.clone()
        }
        None => chrono::DateTime::<chrono::Utc>::from_timestamp_millis(identity::now_ms())
            .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default(),
    };

    let base_url = env_string("KUCOIN_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let settle_ms = match env_string("ANALYZER_CANDLE_SETTLE_MS") {
        Some(value) => value.parse::<i64>().map_err(|_| {
            format!("invalid ANALYZER_CANDLE_SETTLE_MS={value:?}: expected milliseconds")
        })?,
        None => DEFAULT_SETTLE_MS,
    };
    if settle_ms < 0 {
        return Err("ANALYZER_CANDLE_SETTLE_MS must be non-negative".into());
    }
    let params = ExperimentParams {
        window: analyzer::harness::experiment::DEFAULT_WINDOW,
        init_fund: cli.init_fund,
        top_n: cli.top_n,
        max_dd_pct: Decimal::from(cli.max_dd_pct),
    };
    let base = CostScenario {
        label: "base".into(),
        fee_bps: Decimal::from(cli.fee_bps),
        slip_bps: Decimal::from(cli.slip_bps),
    };
    let stress = CostScenario {
        label: "stress".into(),
        fee_bps: Decimal::from(cli.stress_fee_bps),
        slip_bps: Decimal::from(cli.stress_slip_bps),
    };
    let start_ms = cli
        .as_of
        .saturating_sub(i64::from(cli.days).saturating_mul(DAY_MS));
    let expected = expected_candles(cli.days, timeframe);

    let mut errors = Vec::new();
    let mut datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    for symbol in &symbols {
        match load_symbol(
            symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            params.window,
            &cli.data_dir,
            &base_url,
            cli.refresh,
        ) {
            Ok(dataset) => {
                let count = dataset.candles.len();
                println!(
                    "{}: {} closed candles, expected ~{}",
                    symbol, count, expected
                );
                if count < MIN_CANDLES {
                    errors.push(format!(
                        "{symbol}: only {count} closed candles (need >= {MIN_CANDLES})"
                    ));
                } else if count as i64 + 2 < expected {
                    errors.push(format!(
                        "{symbol}: only {count} closed candles for a {}-day window (expected ~{expected})",
                        cli.days
                    ));
                }
                datasets.insert(symbol.clone(), dataset);
            }
            Err(error) => errors.push(format!("{symbol}: fetch failed: {error}")),
        }
    }

    let btc_symbol = "BTC-USDT";
    if !datasets.contains_key(btc_symbol) && !symbols.iter().any(|symbol| symbol == btc_symbol) {
        match load_symbol(
            btc_symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            params.window,
            &cli.data_dir,
            &base_url,
            cli.refresh,
        ) {
            Ok(dataset) => {
                datasets.insert(btc_symbol.to_string(), dataset);
            }
            Err(error) => errors.push(format!("{btc_symbol}: BTC regime fetch failed: {error}")),
        }
    }
    let btc_regime = datasets
        .get(btc_symbol)
        .map(|dataset| btc_regime_map(&dataset.candles, REGIME_SMA_PERIOD))
        .unwrap_or_default();
    if btc_regime.is_empty() {
        errors.push(format!(
            "{btc_symbol}: no BTC regime map; regime robustness will report unknown"
        ));
    }

    let mut experiments = Vec::new();
    for symbol in &symbols {
        let Some(dataset) = datasets.get(symbol) else {
            continue;
        };
        if dataset.candles.len() < MIN_CANDLES {
            continue;
        }
        match evaluate_symbol(dataset, &params, &base, &stress) {
            Ok(mut experiment) => {
                experiment.btc_regime = btc_regime.clone();
                experiments.push(experiment);
            }
            Err(error) => errors.push(format!("{symbol}: evaluation failed: {error}")),
        }
    }

    if experiments.is_empty() {
        print_summary(&experiments, &errors, &generated_at);
        return Err("no symbol produced an experiment result".into());
    }

    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for experiment in &experiments {
        let dir = cli.out_dir.join(&experiment.symbol);
        write_symbol_report(&dir, experiment, &params, &base, &stress)?;
    }
    write_combined_index(&cli.out_dir, &experiments)?;
    print_summary(&experiments, &errors, &generated_at);
    Ok(())
}
