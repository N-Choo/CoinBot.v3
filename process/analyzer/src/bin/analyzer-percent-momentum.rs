use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::percent_momentum::{
    buy_and_hold, simulate, PercentMomentumConfig, PercentMomentumMetrics, PercentMomentumParams,
};
use analyzer::harness::sweep::data::{fetch_range, read_cache, retain_range, write_cache};
use analyzer::identity;
use analyzer::strategy::types::Candle;
use clap::Parser;
use rust_decimal::Decimal;
use serde_json::json;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MIN_CANDLES: usize = 700;
const LOOKBACKS: [usize; 3] = [5, 6, 7];

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-percent-momentum",
    about = "Percentage mean-reversion train/test experiment on daily candles"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,XRP-USDT,SOL-USDT")]
    symbols: String,

    /// Timeframe; this experiment supports 1day only.
    #[arg(long, default_value = "1day")]
    timeframe: String,

    /// History window in days.
    #[arg(long, default_value_t = 730)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/percent-momentum")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/percent-momentum")]
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

    /// Mean-reversion threshold in percent.
    #[arg(long, default_value_t = 5)]
    threshold_pct: u32,
}

struct TrainRow {
    lookback: usize,
    metrics: PercentMomentumMetrics,
    selected: bool,
}

struct SymbolResult {
    symbol: String,
    candles: usize,
    first_open_ms: i64,
    last_open_ms: i64,
    train_candles: usize,
    test_candles: usize,
    train_first_ms: i64,
    train_last_ms: i64,
    test_first_ms: i64,
    test_last_ms: i64,
    train: Vec<TrainRow>,
    selected_lookback: usize,
    test_base: PercentMomentumMetrics,
    test_stress: PercentMomentumMetrics,
    bench_base: PercentMomentumMetrics,
    bench_stress: PercentMomentumMetrics,
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
) -> Result<Dataset, String> {
    load_csv(symbol, path, timeframe, as_of, settle_ms, 0, 0, 0).map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn load_symbol(
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    as_of: i64,
    settle_ms: i64,
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
                    match dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms) {
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
    dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms)
}

fn tune(
    candles: &[Candle],
    train_end: usize,
    threshold: Decimal,
    params: &PercentMomentumParams,
) -> Result<(Vec<TrainRow>, usize), String> {
    let mut rows = Vec::with_capacity(LOOKBACKS.len());
    for lookback in LOOKBACKS {
        let cfg = PercentMomentumConfig {
            lookback,
            threshold,
        };
        let outcome = simulate(candles, 0, train_end, &cfg, params)?;
        rows.push(TrainRow {
            lookback,
            metrics: outcome.metrics,
            selected: false,
        });
    }

    let mut best = 0usize;
    for index in 1..rows.len() {
        let candidate = &rows[index].metrics;
        let current = &rows[best].metrics;
        if candidate.sharpe > current.sharpe
            || (candidate.sharpe == current.sharpe
                && candidate.total_return_pct > current.total_return_pct)
        {
            best = index;
        }
    }
    rows[best].selected = true;
    let selected_lookback = rows[best].lookback;
    Ok((rows, selected_lookback))
}

fn evaluate_symbol(
    dataset: &Dataset,
    threshold: Decimal,
    base: &PercentMomentumParams,
    stress: &PercentMomentumParams,
) -> Result<SymbolResult, String> {
    let candles = &dataset.candles;
    let count = candles.len();
    let split = count / 2;
    if split < 2 || count - split < 2 {
        return Err(format!(
            "{}: only {count} candles; need at least 4 for a train/test split",
            dataset.symbol
        ));
    }

    let (train, selected_lookback) = tune(candles, split, threshold, base)?;
    let selected_cfg = PercentMomentumConfig {
        lookback: selected_lookback,
        threshold,
    };
    let test_base = simulate(candles, split, count, &selected_cfg, base)?.metrics;
    let test_stress = simulate(candles, split, count, &selected_cfg, stress)?.metrics;
    let bench_base = buy_and_hold(candles, split, count, base)?.metrics;
    let bench_stress = buy_and_hold(candles, split, count, stress)?.metrics;

    Ok(SymbolResult {
        symbol: dataset.symbol.clone(),
        candles: count,
        first_open_ms: candles[0].open_time_ms,
        last_open_ms: candles[count - 1].open_time_ms,
        train_candles: split,
        test_candles: count - split,
        train_first_ms: candles[0].open_time_ms,
        train_last_ms: candles[split - 1].open_time_ms,
        test_first_ms: candles[split].open_time_ms,
        test_last_ms: candles[count - 1].open_time_ms,
        train,
        selected_lookback,
        test_base,
        test_stress,
        bench_base,
        bench_stress,
    })
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn dec(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn write_metrics_row(out: &mut String, label: &str, metrics: &PercentMomentumMetrics) {
    writeln!(
        out,
        "| {label} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        dec(metrics.total_return_pct),
        dec(metrics.cagr_pct),
        dec(metrics.max_drawdown_pct),
        dec(metrics.sharpe),
        dec(metrics.sortino),
        metrics.trades,
        dec(metrics.win_rate_pct),
        dec(metrics.avg_holding_days),
        dec(metrics.turnover),
        dec(metrics.costs),
        dec(metrics.avg_abs_exposure)
    )
    .expect("writing to String cannot fail");
}

fn write_metrics_header(out: &mut String, first: &str) {
    writeln!(
        out,
        "| {first} | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    .expect("writing to String cannot fail");
}

fn write_symbol_report(
    dir: &Path,
    result: &SymbolResult,
    threshold: Decimal,
    base: &PercentMomentumParams,
    stress: &PercentMomentumParams,
    generated_at: &str,
    as_of: i64,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;

    let threshold_pct = threshold * Decimal::from(100);
    let mut out = String::new();
    writeln!(
        out,
        "# Percentage Mean Reversion — {} (1day)",
        result.symbol
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Research artifact generated at {generated_at}. This is not a profitability claim."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Assumptions").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Universe: BTC-USDT, XRP-USDT, SOL-USDT (this report: {}).",
        result.symbol
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- Timeframe: 1day.").expect("writing to String cannot fail");
    writeln!(
        out,
        "- Data: 2 years ending {} (as_of {}), {} closed candles retained.",
        iso(as_of),
        as_of,
        result.candles
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Split: first {} candles for training, last {} for testing. Tuning never reads the test half.",
        result.train_candles, result.test_candles
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Rule (mean reversion): C2 is the close on day t and C1 is the close L days earlier (C1 = close[t - L]). If C1 / C2 - 1 > +{}% (the earlier price was above C2, so price fell) -> target +1 unit (buy); if C1 / C2 - 1 < -{}% (price rose) -> target -1 unit (sell); otherwise hold the current position.",
        dec(threshold_pct),
        dec(threshold_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Position: one unit is 100% of strategy equity, long or short; no volatility targeting and no leverage beyond 1x. Short selling is a backtest simulation only."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Execution: C2 is read at the close of day t-1 and filled at the open of day t; no look-ahead."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Costs: base {} bps fee + {} bps slippage per side; stress {} bps fee + {} bps slippage per side; borrow cost 0.",
        dec(base.fee_bps),
        dec(base.slip_bps),
        dec(stress.fee_bps),
        dec(stress.slip_bps)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Tuning: L in 5, 6, 7 on the training half; select the highest Sharpe, breaking ties by total return; freeze L before evaluating the test half."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Metrics: turnover is the sum of absolute position changes in units; avg abs exposure is a fraction (1.0 = fully invested); costs are in quote currency."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", result.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | 1day |").expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", result.candles).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Full range | {} .. {} |",
        iso(result.first_open_ms),
        iso(result.last_open_ms)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Train | {} .. {} ({} candles) |",
        iso(result.train_first_ms),
        iso(result.train_last_ms),
        result.train_candles
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Test | {} .. {} ({} candles) |",
        iso(result.test_first_ms),
        iso(result.test_last_ms),
        result.test_candles
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Train tuning (base costs)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Selection rule: highest Sharpe; ties break on total return. The selected row is marked with **selected**."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_metrics_header(&mut out, "Lookback");
    for row in &result.train {
        let label = if row.selected {
            format!("L={} **selected**", row.lookback)
        } else {
            format!("L={}", row.lookback)
        };
        write_metrics_row(&mut out, &label, &row.metrics);
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Selected lookback: **L={}**.",
        result.selected_lookback
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Test (frozen L={})", result.selected_lookback)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_metrics_header(&mut out, "Costs");
    write_metrics_row(&mut out, "Base", &result.test_base);
    write_metrics_row(&mut out, "Stress", &result.test_stress);
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Buy-and-hold benchmark (test year)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "The benchmark buys at the first test open and sells at the last test close, paying one entry and one exit cost."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_metrics_header(&mut out, "Costs");
    write_metrics_row(&mut out, "Base", &result.bench_base);
    write_metrics_row(&mut out, "Stress", &result.bench_stress);
    writeln!(out).expect("writing to String cannot fail");

    let excess = result.test_base.total_return_pct - result.bench_base.total_return_pct;
    writeln!(
        out,
        "Base-cost comparison: strategy {}% vs buy-and-hold {}% (excess {} percentage points); Sharpe {} vs {}.",
        dec(result.test_base.total_return_pct),
        dec(result.bench_base.total_return_pct),
        dec(excess),
        dec(result.test_base.sharpe),
        dec(result.bench_base.sharpe)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Limitations").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- One fixed threshold ({}) and three lookbacks; the threshold is not tuned and the selection sample is small.",
        dec(threshold_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- One test year per symbol; results are sensitive to the chosen split and market regime."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Daily closes only; intraday stops, funding, borrow availability, and market impact are not modelled."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Costs are a flat per-side rate on turnover; the short leg is a paper simulation and has no borrow cost."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Sharpe and Sortino are annualized with 365 daily bars; CAGR uses the simulated bar count."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;

    let value = json!({
        "symbol": &result.symbol,
        "timeframe": "1day",
        "generated_at": generated_at,
        "as_of_ms": as_of,
        "threshold": threshold.to_string(),
        "selection_rule": "highest Sharpe; tie broken by total return",
        "dataset": {
            "candles": result.candles,
            "first_open_time": iso(result.first_open_ms),
            "last_open_time": iso(result.last_open_ms),
            "train": {
                "first_open_time": iso(result.train_first_ms),
                "last_open_time": iso(result.train_last_ms),
                "candles": result.train_candles,
            },
            "test": {
                "first_open_time": iso(result.test_first_ms),
                "last_open_time": iso(result.test_last_ms),
                "candles": result.test_candles,
            },
        },
        "costs": {
            "base": {
                "fee_bps": base.fee_bps.to_string(),
                "slip_bps": base.slip_bps.to_string(),
            },
            "stress": {
                "fee_bps": stress.fee_bps.to_string(),
                "slip_bps": stress.slip_bps.to_string(),
            },
        },
        "train_results": result
            .train
            .iter()
            .map(|row| json!({
                "lookback": row.lookback,
                "metrics": &row.metrics,
                "selected": row.selected,
            }))
            .collect::<Vec<_>>(),
        "selected_lookback": result.selected_lookback,
        "test_results": {
            "base": &result.test_base,
            "stress": &result.test_stress,
        },
        "benchmark": {
            "base": &result.bench_base,
            "stress": &result.bench_stress,
        },
    });
    let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), text).map_err(|error| error.to_string())
}

fn write_combined_index(
    dir: &Path,
    results: &[SymbolResult],
    errors: &[String],
    generated_at: &str,
    as_of: i64,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut out = String::new();
    writeln!(out, "# Percentage Mean Reversion — 1day, 2-year train/test")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Generated at {generated_at}; data as of {} ({} ms).",
        iso(as_of),
        as_of
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Assumptions: symbols BTC-USDT, XRP-USDT, SOL-USDT; 1day candles; first half of the retained history is training and the second half is the untouched test year. The mean-reversion rule sets C2 to the current close and C1 to the close L days earlier: if C1 / C2 - 1 is above +5% (the earlier price was above C2, so price fell) the target is +1 unit, below -5% (price rose) the target is -1 unit, otherwise the position is held. Signals fill at the next open. Base costs are 10 bps fee + 5 bps slippage per side, stress costs are 20 + 10, and borrow is free. One unit is 100% of strategy equity with no leverage beyond 1x; shorts are a backtest simulation only."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Selection: run L=5, 6, 7 on the training half, pick the highest Sharpe (tie: total return), freeze it, then evaluate the test half once."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Candles | Train L=5 Sharpe | Train L=6 Sharpe | Train L=7 Sharpe | Selected L | Test return % (base) | Test Sharpe (base) | Test return % (stress) | B&H return % (base) | Excess (pp) |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    .expect("writing to String cannot fail");
    for result in results {
        let sharpe_for = |lookback: usize| {
            result
                .train
                .iter()
                .find(|row| row.lookback == lookback)
                .map(|row| dec(row.metrics.sharpe))
                .unwrap_or_else(|| "—".to_string())
        };
        let excess = result.test_base.total_return_pct - result.bench_base.total_return_pct;
        writeln!(
            out,
            "| [{}]({}/report.md) | {} | {} | {} | {} | L={} | {} | {} | {} | {} | {} |",
            result.symbol,
            result.symbol,
            result.candles,
            sharpe_for(5),
            sharpe_for(6),
            sharpe_for(7),
            result.selected_lookback,
            dec(result.test_base.total_return_pct),
            dec(result.test_base.sharpe),
            dec(result.test_stress.total_return_pct),
            dec(result.bench_base.total_return_pct),
            dec(excess)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    if !errors.is_empty() {
        writeln!(out, "## Data issues").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        for error in errors {
            writeln!(out, "- {error}").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }
    writeln!(
        out,
        "Per-symbol artifacts: `report.md` and `results.json`. Turnover is the sum of absolute position changes; avg abs exposure is a fraction (1.0 = fully invested). Sharpe and Sortino are annualized with 365 daily bars. This is a research artifact, not a profitability claim."
    )
    .expect("writing to String cannot fail");
    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())
}

fn print_summary(results: &[SymbolResult], errors: &[String], generated_at: &str) {
    println!();
    println!(
        "percent-momentum completed for {} symbol(s); generated_at={generated_at}",
        results.len()
    );
    for result in results {
        let excess = result.test_base.total_return_pct - result.bench_base.total_return_pct;
        println!(
            "{}: candles={} selected=L{} test_base_return={}% test_base_sharpe={} test_stress_return={}% bench_base_return={}% excess={}pp",
            result.symbol,
            result.candles,
            result.selected_lookback,
            dec(result.test_base.total_return_pct),
            dec(result.test_base.sharpe),
            dec(result.test_stress.total_return_pct),
            dec(result.bench_base.total_return_pct),
            dec(excess)
        );
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
    if timeframe != Timeframe::D1 {
        return Err("--timeframe must be 1day for this experiment".into());
    }
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.threshold_pct == 0 || cli.threshold_pct > 100 {
        return Err("--threshold-pct must be between 1 and 100".into());
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

    let threshold = Decimal::from(cli.threshold_pct) / Decimal::from(100);
    let base = PercentMomentumParams {
        init_fund: Decimal::from(cli.init_fund),
        fee_bps: Decimal::from(cli.fee_bps),
        slip_bps: Decimal::from(cli.slip_bps),
    };
    let stress = PercentMomentumParams {
        init_fund: Decimal::from(cli.init_fund),
        fee_bps: Decimal::from(cli.stress_fee_bps),
        slip_bps: Decimal::from(cli.stress_slip_bps),
    };
    let start_ms = cli
        .as_of
        .saturating_sub(i64::from(cli.days).saturating_mul(DAY_MS));
    let expected = i64::from(cli.days);

    let mut errors = Vec::new();
    let mut results = Vec::new();
    for symbol in &symbols {
        let dataset = match load_symbol(
            symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            &cli.data_dir,
            &base_url,
            cli.refresh,
        ) {
            Ok(dataset) => dataset,
            Err(error) => {
                errors.push(format!("{symbol}: fetch failed: {error}"));
                continue;
            }
        };
        let count = dataset.candles.len();
        println!(
            "{}: {} closed candles, expected ~{}",
            symbol, count, expected
        );
        if count < MIN_CANDLES {
            errors.push(format!(
                "{symbol}: only {count} closed candles (need >= {MIN_CANDLES})"
            ));
            continue;
        }
        if count as i64 + 2 < expected {
            println!(
                "{}: only {} closed candles for a {}-day window; continuing",
                symbol, count, cli.days
            );
        }
        match evaluate_symbol(&dataset, threshold, &base, &stress) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(format!("{symbol}: evaluation failed: {error}")),
        }
    }

    if results.is_empty() {
        print_summary(&results, &errors, &generated_at);
        return Err("no symbol produced a result".into());
    }

    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for result in &results {
        let dir = cli.out_dir.join(&result.symbol);
        write_symbol_report(
            &dir,
            result,
            threshold,
            &base,
            &stress,
            &generated_at,
            cli.as_of,
        )?;
    }
    write_combined_index(&cli.out_dir, &results, &errors, &generated_at, cli.as_of)?;
    print_summary(&results, &errors, &generated_at);
    Ok(())
}
