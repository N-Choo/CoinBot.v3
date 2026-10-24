use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, read_cache, retain_range, write_cache};
use analyzer::harness::walkforward::{
    evaluate_symbol_walkforward, overall_decision, SymbolWalkForward, WalkForwardParams,
};
use analyzer::harness::walkforward_report::{write_combined_index, write_symbol_report};
use analyzer::identity;
use analyzer::strategy::types::Candle;
use clap::Parser;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MIN_CANDLES: usize = 2700;
const REGIME_SMA_PERIOD: usize = 200;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-walkforward",
    about = "Expanding-window RSI walk-forward experiment with per-symbol reports"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,ETH-USDT,SOL-USDT,XRP-USDT,BNB-USDT")]
    symbols: String,

    /// Timeframe; this experiment supports 4hour only.
    #[arg(long, default_value = "4hour")]
    timeframe: String,

    /// History window in days (15 months).
    #[arg(long, default_value_t = 456)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/rsi-walkforward")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/walkforward")]
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

    /// Minimum training trades.
    #[arg(long, default_value_t = 10)]
    min_train_trades: usize,

    /// Minimum OOS trades before a run can succeed.
    #[arg(long, default_value_t = 10)]
    min_oos_trades: usize,

    /// Maximum training drawdown percentage.
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
        match read_cache(&path) {
            Ok(mut candles) => {
                retain_range(&mut candles, start_ms, as_of);
                if candles.len() >= MIN_CANDLES {
                    write_cache(&path, &candles)?;
                    match dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms, window) {
                        Ok(dataset) => {
                            println!(
                                "{}: reusing cache, {} closed candles",
                                symbol,
                                dataset.candles.len()
                            );
                            return Ok(dataset);
                        }
                        Err(error) => {
                            println!("{}: cache unusable ({error}); refetching", symbol);
                        }
                    }
                } else {
                    println!(
                        "{}: cache has only {} closed candles in range; refetching",
                        symbol,
                        candles.len()
                    );
                }
            }
            Err(error) => {
                println!("{}: cache unreadable ({error}); refetching", symbol);
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

fn print_summary(
    results: &[SymbolWalkForward],
    errors: &[String],
    generated_at: &str,
    overall: &str,
    expected_symbols: usize,
) {
    println!();
    println!(
        "walk-forward completed for {} of {} symbol(s); generated_at={generated_at}",
        results.len(),
        expected_symbols
    );
    for result in results {
        let winner = result
            .winner
            .as_ref()
            .map(|run| {
                format!(
                    "{} @ M1-M{} (OOS trades={}, expectancy={}, PF={})",
                    run.candidate.label,
                    run.window_months,
                    run.oos.metrics.trades,
                    run.oos.metrics.net_expectancy,
                    run.oos
                        .metrics
                        .net_profit_factor
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "inf".to_string())
                )
            })
            .unwrap_or_else(|| "none".to_string());
        println!(
            "{}: candles={} winner={} successful_configs={} verdict={}",
            result.symbol,
            result.dataset.candles.len(),
            winner,
            result.successful_runs.len(),
            result.decision.verdict.as_str()
        );
    }
    println!("overall verdict: {overall}");
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
    let params = WalkForwardParams {
        init_fund: cli.init_fund,
        min_train_trades: cli.min_train_trades,
        min_oos_trades: cli.min_oos_trades,
        max_dd_pct: Decimal::from(cli.max_dd_pct),
        ..WalkForwardParams::default()
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
                } else {
                    datasets.insert(symbol.clone(), dataset);
                }
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

    let mut results = Vec::new();
    for symbol in &symbols {
        let Some(dataset) = datasets.get(symbol) else {
            continue;
        };
        if dataset.candles.len() < MIN_CANDLES {
            continue;
        }
        println!("{symbol}: evaluating 5 expanding windows...");
        match evaluate_symbol_walkforward(dataset, &params, &base, &stress) {
            Ok(mut result) => {
                println!(
                    "{symbol}: evaluation complete ({} successful run(s))",
                    result.successful_runs.len()
                );
                result.btc_regime = btc_regime.clone();
                results.push(result);
            }
            Err(error) => errors.push(format!("{symbol}: evaluation failed: {error}")),
        }
    }

    let verdicts: Vec<_> = results
        .iter()
        .map(|result| result.decision.verdict)
        .collect();
    let overall = overall_decision(&verdicts, symbols.len());
    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for result in &results {
        let dir = cli.out_dir.join(&result.symbol);
        write_symbol_report(&dir, result, &params, &base, &stress, &symbols, overall)?;
    }
    write_combined_index(&cli.out_dir, &results, &errors, symbols.len())?;
    print_summary(
        &results,
        &errors,
        &generated_at,
        overall.as_str(),
        symbols.len(),
    );

    if results.is_empty() {
        return Err("no symbol produced a walk-forward result".into());
    }
    Ok(())
}
