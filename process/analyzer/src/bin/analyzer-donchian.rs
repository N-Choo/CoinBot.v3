use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::donchian::{
    evaluate_symbol_with, DonchianParams, SymbolDonchian, DEFAULT_ATR_PERIOD, DEFAULT_EMA_PERIOD,
};
use analyzer::harness::donchian_report::{write_combined_index, write_symbol_report};
use analyzer::harness::experiment::Decision;
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, read_cache, retain_range, write_cache};
use analyzer::identity;
use clap::Parser;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-donchian",
    about = "Donchian breakout walk-forward experiment with per-symbol reports"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,ETH-USDT,SOL-USDT,BNB-USDT,XRP-USDT")]
    symbols: String,

    /// Timeframe; supports 4hour and 1day.
    #[arg(long, default_value = "4hour")]
    timeframe: String,

    /// History window in days (5 years).
    #[arg(long, default_value_t = 1825)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/donchian-v2-4h")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/donchian")]
    data_dir: PathBuf,

    /// Refetch every symbol instead of reusing the cache.
    #[arg(long)]
    refresh: bool,

    /// Minimum OOS trades required for a run to pass.
    #[arg(long, default_value_t = 10)]
    min_oos_trades: usize,

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

    /// EMA period used for the trend filter and exit.
    #[arg(long, default_value_t = DEFAULT_EMA_PERIOD)]
    ema_period: usize,

    /// ATR period used for the trailing stop.
    #[arg(long, default_value_t = DEFAULT_ATR_PERIOD)]
    atr_period: usize,
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
    min_candles: usize,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
) -> Result<Dataset, String> {
    let path = cache_path(data_dir, symbol, timeframe);
    if !refresh && path.is_file() {
        match read_cache(&path) {
            Ok(mut candles) => {
                retain_range(&mut candles, start_ms, as_of);
                if candles.len() >= min_candles {
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

fn expected_candles(days: u32, timeframe: Timeframe) -> i64 {
    i64::from(days) * DAY_MS / timeframe.millis()
}

fn overall_decision(verdicts: &[Decision], expected_symbols: usize) -> Decision {
    if verdicts.is_empty()
        || verdicts.len() != expected_symbols
        || verdicts.iter().any(|verdict| *verdict != Decision::Pass)
    {
        Decision::Fail
    } else {
        Decision::Pass
    }
}

fn print_summary(
    results: &[SymbolDonchian],
    errors: &[String],
    generated_at: &str,
    overall: Decision,
    expected_symbols: usize,
) {
    println!();
    println!(
        "donchian 5-year experiment completed for {} of {} symbol(s); generated_at={generated_at}",
        results.len(),
        expected_symbols
    );
    for result in results {
        let winner = result
            .winner
            .as_ref()
            .map(|run| {
                format!(
                    "{} @ M1-M{} (stress trades={}, expectancy={}, PF={}, net={}, benchmark={})",
                    run.config.label(),
                    run.window_months,
                    run.oos_stress.trades,
                    run.oos_stress.net_expectancy,
                    run.oos_stress
                        .net_profit_factor
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "inf".to_string()),
                    run.oos_stress.net_pnl,
                    run.benchmark_stress.net_pnl
                )
            })
            .unwrap_or_else(|| "none".to_string());
        println!(
            "{}: candles={} winner={} successful_runs={} verdict={}",
            result.symbol,
            result.dataset.candles.len(),
            winner,
            result.successful_runs.len(),
            result.decision.verdict.as_str()
        );
    }
    println!("overall verdict: {}", overall.as_str());
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
    if timeframe != Timeframe::H4 && timeframe != Timeframe::D1 {
        return Err("--timeframe must be 4hour or 1day for this experiment".into());
    }
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.ema_period == 0 || cli.atr_period == 0 {
        return Err("--ema-period and --atr-period must be positive".into());
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
    let params = DonchianParams {
        init_fund: cli.init_fund,
        ema_period: cli.ema_period,
        atr_period: cli.atr_period,
        min_oos_trades: cli.min_oos_trades,
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
    let min_candles = (expected as usize) * 9 / 10;
    let warmup = params
        .ema_period
        .max(params.atr_period)
        .max(analyzer::harness::donchian::DONCHIAN_EXIT)
        .max(analyzer::harness::donchian::ENTRY_GRID[1]);

    let mut errors = Vec::new();
    let mut datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    for symbol in &symbols {
        match load_symbol(
            symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            warmup,
            min_candles,
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
                if count < min_candles {
                    errors.push(format!(
                        "{symbol}: only {count} closed candles (need >= {min_candles})"
                    ));
                } else {
                    datasets.insert(symbol.clone(), dataset);
                }
            }
            Err(error) => errors.push(format!("{symbol}: fetch failed: {error}")),
        }
    }

    let mut results = Vec::new();
    for symbol in &symbols {
        let Some(dataset) = datasets.get(symbol) else {
            continue;
        };
        println!("{symbol}: evaluating 5-year walk-forward...");
        let result = evaluate_symbol_with(dataset, &base, &stress, &params);
        println!(
            "{symbol}: evaluation complete ({} successful run(s))",
            result.successful_runs.len()
        );
        results.push(result);
    }

    let verdicts: Vec<_> = results
        .iter()
        .map(|result| result.decision.verdict)
        .collect();
    let overall = overall_decision(&verdicts, symbols.len());
    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for result in &results {
        let dir = cli.out_dir.join(&result.symbol);
        write_symbol_report(&dir, result, &base, &stress, &symbols, overall)?;
    }
    write_combined_index(
        &cli.out_dir,
        &results,
        &errors,
        symbols.len(),
        &generated_at,
    )?;
    print_summary(&results, &errors, &generated_at, overall, symbols.len());

    if results.is_empty() {
        return Err("no symbol produced a donchian result".into());
    }
    Ok(())
}
