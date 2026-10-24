use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::atr_experiment::{atr_variants, evaluate_atr_slice};
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::experiment::{evaluate_slice, side_metrics, SideMetrics};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, write_cache};
use analyzer::harness::sweep::evaluate::PartitionMetrics;
use analyzer::harness::sweep::partition::Slice;
use analyzer::identity;
use analyzer::strategy::config::Config;
use analyzer::strategy::types::{Candle, Direction};
use clap::Parser;
use rust_decimal::Decimal;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MIN_CANDLES: usize = 1000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-fixed",
    about = "One fixed RSI configuration with no tuning, compared with ATR exits"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,BNB-USDT,SOL-USDT,XRP-USDT")]
    symbols: String,

    /// Timeframe.
    #[arg(long, default_value = "1day")]
    timeframe: String,

    /// History window in days.
    #[arg(long, default_value_t = 1095)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/rsi-fixed-3y")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/fixed")]
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

    /// RSI lookback period.
    #[arg(long, default_value_t = 21)]
    rsi_period: usize,

    /// Long entry threshold.
    #[arg(long = "long", default_value_t = 60)]
    long_threshold: u32,

    /// Short entry threshold.
    #[arg(long = "short", default_value_t = 40)]
    short_threshold: u32,
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

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn candle_time(candles: &[Candle], index: usize) -> String {
    candles
        .get(index)
        .map(|candle| iso(candle.open_time_ms))
        .unwrap_or_else(|| "—".to_string())
}

fn decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn profit_factor(value: Option<Decimal>, trades: usize) -> String {
    match value {
        Some(value) => decimal(value),
        None if trades == 0 => "—".to_string(),
        None => "∞".to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
struct MetricsView {
    trades: usize,
    net_expectancy: Decimal,
    net_profit_factor: Option<Decimal>,
    max_drawdown_pct: Decimal,
    expectancy_stderr: Decimal,
    closed_net_pnl: Decimal,
    open_net_pnl: Decimal,
    net_pnl: Decimal,
    win_rate: Decimal,
}

impl MetricsView {
    fn from_metrics(metrics: &PartitionMetrics) -> Self {
        Self {
            trades: metrics.trades,
            net_expectancy: metrics.net_expectancy,
            net_profit_factor: metrics.net_profit_factor,
            max_drawdown_pct: metrics.max_drawdown_pct,
            expectancy_stderr: metrics.expectancy_stderr,
            closed_net_pnl: metrics.closed_net_pnl,
            open_net_pnl: metrics.open_net_pnl,
            net_pnl: metrics.net_pnl,
            win_rate: metrics.win_rate,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DatasetView {
    first_open_time: String,
    last_open_time: String,
    candles: usize,
    content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct StrategyView {
    rsi_period: usize,
    rsi_long_threshold: Decimal,
    rsi_short_threshold: Decimal,
    atr: bool,
    stop_loss: bool,
    take_profit: bool,
    tuning: bool,
    window: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CostView {
    label: String,
    fee_bps: Decimal,
    slip_bps: Decimal,
}

impl CostView {
    fn from_scenario(scenario: &CostScenario) -> Self {
        Self {
            label: scenario.label.clone(),
            fee_bps: scenario.fee_bps,
            slip_bps: scenario.slip_bps,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PeriodView {
    base: MetricsView,
    stress: MetricsView,
}

#[derive(Debug, Clone, Serialize)]
struct VariantResults {
    label: String,
    enabled: bool,
    period: usize,
    sl_multiplier: Decimal,
    tp_multiplier: Decimal,
    pct_high: Decimal,
    high_adjust: Decimal,
    sl_max_pct: Decimal,
    base: MetricsView,
    stress: MetricsView,
}

#[derive(Debug, Clone, Serialize)]
struct YearView {
    label: String,
    start_index: usize,
    end_index: usize,
    start_time: String,
    end_time: String,
    base: MetricsView,
    stress: MetricsView,
}

#[derive(Debug, Clone, Serialize)]
struct SideView {
    long: SideMetrics,
    short: SideMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct SidesView {
    base: SideView,
    stress: SideView,
}

#[derive(Debug, Clone, Serialize)]
struct SymbolResults {
    symbol: String,
    timeframe: String,
    generated_at: String,
    as_of_ms: i64,
    days: u32,
    dataset: DatasetView,
    strategy: StrategyView,
    base_costs: CostView,
    stress_costs: CostView,
    full_period: PeriodView,
    variants: Vec<VariantResults>,
    yearly: Vec<YearView>,
    sides: SidesView,
}

/// Three equal candle-count thirds, used as the yearly breakdown.
fn yearly_slices(n: usize) -> [(&'static str, Slice); 3] {
    let first = n / 3;
    let second = n * 2 / 3;
    [
        (
            "Year 1",
            Slice {
                start: 0,
                end: first,
            },
        ),
        (
            "Year 2",
            Slice {
                start: first,
                end: second,
            },
        ),
        (
            "Year 3",
            Slice {
                start: second,
                end: n,
            },
        ),
    ]
}

fn year_view(
    dataset: &Dataset,
    cfg: &Config,
    label: &str,
    slice: Slice,
    init_fund: i64,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<YearView, String> {
    let base_outcome = evaluate_slice(dataset, cfg, slice, cfg.window, init_fund, base)?;
    let stress_outcome = evaluate_slice(dataset, cfg, slice, cfg.window, init_fund, stress)?;
    Ok(YearView {
        label: label.to_string(),
        start_index: slice.start,
        end_index: slice.end,
        start_time: candle_time(&dataset.candles, slice.start),
        end_time: candle_time(&dataset.candles, slice.end.saturating_sub(1)),
        base: MetricsView::from_metrics(&base_outcome.metrics),
        stress: MetricsView::from_metrics(&stress_outcome.metrics),
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_symbol(
    dataset: &Dataset,
    cfg: &Config,
    generated_at: &str,
    as_of: i64,
    days: u32,
    init_fund: i64,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<SymbolResults, String> {
    let n = dataset.candles.len();
    let full = evaluate_slice(
        dataset,
        cfg,
        Slice { start: 0, end: n },
        cfg.window,
        init_fund,
        base,
    )?;
    let full_stress = evaluate_slice(
        dataset,
        cfg,
        Slice { start: 0, end: n },
        cfg.window,
        init_fund,
        stress,
    )?;
    let last_close = dataset
        .candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or(Decimal::ZERO);
    let sides = SidesView {
        base: SideView {
            long: side_metrics(&full.outcome, base, Direction::Long, last_close),
            short: side_metrics(&full.outcome, base, Direction::Short, last_close),
        },
        stress: SideView {
            long: side_metrics(&full_stress.outcome, stress, Direction::Long, last_close),
            short: side_metrics(&full_stress.outcome, stress, Direction::Short, last_close),
        },
    };

    let full_slice = Slice { start: 0, end: n };
    let mut variants = Vec::with_capacity(5);
    for variant in atr_variants() {
        let (base_metrics, stress_metrics) = if variant.enabled {
            let base_outcome = evaluate_atr_slice(
                dataset, cfg, &variant, full_slice, cfg.window, init_fund, base,
            )?;
            let stress_outcome = evaluate_atr_slice(
                dataset, cfg, &variant, full_slice, cfg.window, init_fund, stress,
            )?;
            (base_outcome.metrics, stress_outcome.metrics)
        } else {
            (full.metrics.clone(), full_stress.metrics.clone())
        };
        variants.push(VariantResults {
            label: variant.label,
            enabled: variant.enabled,
            period: variant.period,
            sl_multiplier: variant.sl_multiplier,
            tp_multiplier: variant.tp_multiplier,
            pct_high: variant.pct_high,
            high_adjust: variant.high_adjust,
            sl_max_pct: variant.sl_max_pct,
            base: MetricsView::from_metrics(&base_metrics),
            stress: MetricsView::from_metrics(&stress_metrics),
        });
    }

    let mut yearly = Vec::with_capacity(3);
    for (label, slice) in yearly_slices(n) {
        yearly.push(year_view(
            dataset, cfg, label, slice, init_fund, base, stress,
        )?);
    }

    Ok(SymbolResults {
        symbol: dataset.symbol.clone(),
        timeframe: dataset.provenance.requested.timeframe.clone(),
        generated_at: generated_at.to_string(),
        as_of_ms: as_of,
        days,
        dataset: DatasetView {
            first_open_time: candle_time(&dataset.candles, 0),
            last_open_time: candle_time(&dataset.candles, n.saturating_sub(1)),
            candles: n,
            content_sha256: dataset.provenance.content_sha256.clone(),
        },
        strategy: StrategyView {
            rsi_period: cfg.rsi_period,
            rsi_long_threshold: cfg.rsi_long_threshold,
            rsi_short_threshold: cfg.rsi_short_threshold,
            atr: false,
            stop_loss: false,
            take_profit: false,
            tuning: false,
            window: cfg.window,
        },
        base_costs: CostView::from_scenario(base),
        stress_costs: CostView::from_scenario(stress),
        full_period: PeriodView {
            base: MetricsView::from_metrics(&full.metrics),
            stress: MetricsView::from_metrics(&full_stress.metrics),
        },
        variants,
        yearly,
        sides,
    })
}

fn write_performance_rows(out: &mut String, base: &MetricsView, stress: &MetricsView) {
    writeln!(
        out,
        "| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for (label, metrics) in [("Base", base), ("Stress", stress)] {
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} | {} | {} |",
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            decimal(metrics.closed_net_pnl),
            decimal(metrics.open_net_pnl),
            decimal(metrics.win_rate)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_atr_rows(out: &mut String, variants: &[VariantResults]) {
    writeln!(
        out,
        "B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    .expect("writing to String cannot fail");
    for variant in variants {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            variant.label,
            variant.base.trades,
            variant.stress.trades,
            decimal(variant.base.net_expectancy),
            decimal(variant.stress.net_expectancy),
            profit_factor(variant.base.net_profit_factor, variant.base.trades),
            profit_factor(variant.stress.net_profit_factor, variant.stress.trades),
            decimal(variant.base.max_drawdown_pct),
            decimal(variant.stress.max_drawdown_pct),
            decimal(variant.base.expectancy_stderr),
            decimal(variant.stress.expectancy_stderr),
            decimal(variant.base.closed_net_pnl),
            decimal(variant.stress.closed_net_pnl),
            decimal(variant.base.open_net_pnl),
            decimal(variant.stress.open_net_pnl),
            decimal(variant.base.win_rate),
            decimal(variant.stress.win_rate)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_side_rows(out: &mut String, title: &str, sides: &SideView) {
    writeln!(out, "**{title}**").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for (label, metrics) in [("Long", &sides.long), ("Short", &sides.short)] {
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            metrics.trades,
            decimal(metrics.net_pnl),
            decimal(metrics.open_mark),
            decimal(metrics.net_expectancy),
            profit_factor(metrics.profit_factor, metrics.trades),
            decimal(metrics.win_rate)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_symbol_report(dir: &Path, results: &SymbolResults) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut out = String::new();
    writeln!(
        out,
        "# Fixed RSI {} Backtest — {} ({} days, no tuning)",
        results.timeframe, results.symbol, results.days
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 1. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", results.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", results.timeframe).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Date range | {} .. {} |",
        results.dataset.first_open_time, results.dataset.last_open_time
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", results.dataset.candles)
        .expect("writing to String cannot fail");
    writeln!(out, "| SHA-256 | {} |", results.dataset.content_sha256)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 2. Fixed strategy").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy version | rsi-mom-v1 |").expect("writing to String cannot fail");
    writeln!(out, "| RSI period | {} |", results.strategy.rsi_period)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Long threshold | {} |",
        decimal(results.strategy.rsi_long_threshold)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Short threshold | {} |",
        decimal(results.strategy.rsi_short_threshold)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| ATR / stop-loss / take-profit | off in the baseline; variants compared below |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Tuning | none |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Warm-up window | {} candles |",
        results.strategy.window
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 3. Full-period performance (ATR off)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_performance_rows(
        &mut out,
        &results.full_period.base,
        &results.full_period.stress,
    );

    writeln!(out, "## 4. ATR SL/TP comparison").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_atr_rows(&mut out, &results.variants);

    writeln!(out, "## 5. Yearly breakdown (ATR off)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Three equal candle-count thirds of the full period.")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for year in &results.yearly {
        for (label, metrics) in [("Base", &year.base), ("Stress", &year.stress)] {
            writeln!(
                out,
                "| {} | {} .. {} | {label} | {} | {} | {} | {} | {} | {} | {} | {} |",
                year.label,
                year.start_time,
                year.end_time,
                metrics.trades,
                decimal(metrics.net_expectancy),
                profit_factor(metrics.net_profit_factor, metrics.trades),
                decimal(metrics.max_drawdown_pct),
                decimal(metrics.expectancy_stderr),
                decimal(metrics.closed_net_pnl),
                decimal(metrics.open_net_pnl),
                decimal(metrics.win_rate)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 6. Long vs short (ATR off)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_side_rows(&mut out, "Full period, base costs", &results.sides.base);
    write_side_rows(&mut out, "Full period, stress costs", &results.sides.stress);

    writeln!(out, "## 7. Execution assumptions and costs").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- A signal produced at a candle close fills at the next candle open."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- One position per symbol; opposite signals reverse at the fill open."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- ATR off exits only on opposite signals. ATR variants can also exit on stop-loss or take-profit, including on the entry candle."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Base costs: {} bps fee + {} bps slippage per side.",
        decimal(results.base_costs.fee_bps),
        decimal(results.base_costs.slip_bps)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Stress costs: {} bps fee + {} bps slippage per side.",
        decimal(results.stress_costs.fee_bps),
        decimal(results.stress_costs.slip_bps)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 7. Notes").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- The requested window spans {} days ending at as_of = {} ms.",
        results.days, results.as_of_ms
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The configuration is fixed at RSI {}/{}/{} and is not tuned on this data.",
        results.strategy.rsi_period,
        decimal(results.strategy.rsi_long_threshold),
        decimal(results.strategy.rsi_short_threshold)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- This report is not a profitability claim.")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}

fn verdict(metrics: &MetricsView) -> &'static str {
    if metrics.trades == 0 {
        "no trades"
    } else if metrics.net_expectancy > Decimal::ZERO
        && metrics
            .net_profit_factor
            .map(|value| value > Decimal::ONE)
            .unwrap_or(true)
    {
        "positive base expectancy"
    } else {
        "no base edge"
    }
}

fn write_combined_variant_table(out: &mut String, results: &[SymbolResults], stress: bool) {
    let title = if stress {
        "Stress costs (20 bps fee + 10 bps slippage per side)"
    } else {
        "Base costs (10 bps fee + 5 bps slippage per side)"
    };
    writeln!(out, "**{title}**").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Variant | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for result in results {
        for variant in &result.variants {
            let metrics = if stress {
                &variant.stress
            } else {
                &variant.base
            };
            writeln!(
                out,
                "| [{}]({}/report.md) | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                result.symbol,
                result.symbol,
                variant.label,
                metrics.trades,
                decimal(metrics.net_expectancy),
                profit_factor(metrics.net_profit_factor, metrics.trades),
                decimal(metrics.max_drawdown_pct),
                decimal(metrics.expectancy_stderr),
                decimal(metrics.closed_net_pnl),
                decimal(metrics.open_net_pnl),
                decimal(metrics.win_rate)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_combined_index(
    dir: &Path,
    results: &[SymbolResults],
    errors: &[String],
    generated_at: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let timeframe = results
        .first()
        .map(|result| result.timeframe.as_str())
        .unwrap_or("—");
    let days = results.first().map(|result| result.days).unwrap_or(0);
    let strategy = results.first().map(|result| &result.strategy);
    let mut out = String::new();
    writeln!(
        out,
        "# Fixed RSI {timeframe} Backtest — {days} Days, No Tuning"
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if let Some(strategy) = strategy {
        writeln!(
            out,
            "One fixed configuration across all symbols: RSI {}, long {}, short {}, no tuning. The baseline has no ATR/SL/TP; the ATR variants use the same signals and add stop-loss/take-profit exits. Generated at `{generated_at}`.",
            strategy.rsi_period,
            decimal(strategy.rsi_long_threshold),
            decimal(strategy.rsi_short_threshold)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Candles | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % | Verdict | Notes |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|"
    )
    .expect("writing to String cannot fail");
    for result in results {
        let metrics = &result.full_period.base;
        let notes = format!(
            "stress expectancy {}",
            decimal(result.full_period.stress.net_expectancy)
        );
        writeln!(
            out,
            "| [{}]({}/report.md) | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            result.symbol,
            result.symbol,
            result.dataset.candles,
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            decimal(metrics.closed_net_pnl),
            decimal(metrics.open_net_pnl),
            decimal(metrics.win_rate),
            verdict(metrics),
            notes
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## ATR SL/TP comparison").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Signal-at-close, fill-at-next-open. ATR exits can trigger on the entry candle; gap exits use the open, and SL wins when both levels are touched. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap. ATR variants are full-period only."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_combined_variant_table(&mut out, results, false);
    write_combined_variant_table(&mut out, results, true);

    if !errors.is_empty() {
        writeln!(out, "## Data issues").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        for error in errors {
            writeln!(out, "- {error}").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }

    writeln!(out, "## Notes").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Full-period metrics are net of base costs (10 bps fee + 5 bps slippage per side); stress costs are 20 + 10."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The requested window spans {days} days ending at as_of."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The configuration is fixed and is not tuned on this data."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Yearly breakdowns cover the ATR-off baseline; ATR variants are reported over the full period only."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- This report is not a profitability claim.")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Per-symbol artifacts: `report.md`, `results.json`.")
        .expect("writing to String cannot fail");

    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())
}

fn print_summary(results: &[SymbolResults], errors: &[String], generated_at: &str) {
    println!();
    println!(
        "fixed backtest completed for {} symbol(s); generated_at={generated_at}",
        results.len()
    );
    for result in results {
        let base = &result.full_period.base;
        println!(
            "{}: candles={} trades={} expectancy={} pf={} max_dd={}% stress_expectancy={}",
            result.symbol,
            result.dataset.candles,
            base.trades,
            decimal(base.net_expectancy),
            profit_factor(base.net_profit_factor, base.trades),
            decimal(base.max_drawdown_pct),
            decimal(result.full_period.stress.net_expectancy)
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
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
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

    let cfg = Config {
        rsi_period: cli.rsi_period,
        rsi_long_threshold: Decimal::from(cli.long_threshold),
        rsi_short_threshold: Decimal::from(cli.short_threshold),
        ..Config::default()
    };
    cfg.validate().map_err(|error| error.to_string())?;

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

    let mut errors = Vec::new();
    let mut results = Vec::new();
    for symbol in &symbols {
        match load_symbol(
            symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            cfg.window,
            &cli.data_dir,
            &base_url,
            cli.refresh,
        ) {
            Ok(dataset) => {
                if dataset.candles.len() < MIN_CANDLES {
                    errors.push(format!(
                        "{}: only {} closed candles (need >= {MIN_CANDLES})",
                        symbol,
                        dataset.candles.len()
                    ));
                    continue;
                }
                match evaluate_symbol(
                    &dataset,
                    &cfg,
                    &generated_at,
                    cli.as_of,
                    cli.days,
                    cli.init_fund,
                    &base,
                    &stress,
                ) {
                    Ok(result) => results.push(result),
                    Err(error) => errors.push(format!("{symbol}: evaluation failed: {error}")),
                }
            }
            Err(error) => errors.push(format!("{symbol}: fetch failed: {error}")),
        }
    }

    if results.is_empty() {
        print_summary(&results, &errors, &generated_at);
        return Err("no symbol produced a backtest result".into());
    }

    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for result in &results {
        write_symbol_report(&cli.out_dir.join(&result.symbol), result)?;
    }
    write_combined_index(&cli.out_dir, &results, &errors, &generated_at)?;
    print_summary(&results, &errors, &generated_at);
    Ok(())
}
