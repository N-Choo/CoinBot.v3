use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, load_with_cache};
use analyzer::harness::ts_momentum_backtest::{
    benchmark, run_range, splits, Mode, Split, SplitName, TsBacktestParams, Variant,
    REGIME_SMA_DAYS,
};
use analyzer::harness::ts_momentum_report::{
    write_reports, DatasetInfo, ReportContext, RunReport, ScenarioInfo, ScenarioRuns, SplitBoundary,
};
use analyzer::identity;
use analyzer::strategy::ts_momentum::{NegativeMomentumMode, TsMomentumConfig};
use clap::Parser;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 86_400_000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-ts-momentum",
    about = "Backtest and report the BTC daily time-series momentum strategy"
)]
struct Cli {
    /// KuCoin symbol.
    #[arg(long, default_value = "BTC-USDT")]
    symbol: String,

    /// History window in days.
    #[arg(long, default_value_t = 1825)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/ts-momentum-7d")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/ts-momentum")]
    data_dir: PathBuf,

    /// Refetch instead of reusing a matching cache.
    #[arg(long)]
    refresh: bool,

    /// Momentum lookback in days.
    #[arg(long, default_value_t = 7)]
    momentum_lookback: usize,

    /// Volatility lookback in days.
    #[arg(long, default_value_t = 7)]
    volatility_lookback: usize,

    /// Initial fund for every split.
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

    /// Daily borrow cost in basis points on short exposure.
    #[arg(long, default_value_t = 0)]
    borrow_bps_per_day: u32,

    /// Drawdown halt threshold in percent.
    #[arg(long, default_value_t = 20)]
    max_dd: u32,

    /// Drawdown resume threshold in percent.
    #[arg(long, default_value_t = 10)]
    drawdown_resume: u32,

    /// KuCoin API base URL; defaults to KUCOIN_BASE_URL, then https://api.kucoin.com.
    #[arg(long)]
    base_url: Option<String>,

    /// Candle settle time in milliseconds; defaults to ANALYZER_CANDLE_SETTLE_MS, then 5000.
    #[arg(long)]
    settle_ms: Option<i64>,
}

fn parse_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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

fn cache_path(data_dir: &Path, symbol: &str) -> PathBuf {
    data_dir.join(format!("{symbol}_{}.csv", Timeframe::D1.as_kucoin()))
}

#[allow(clippy::too_many_arguments)]
fn load_dataset(
    symbol: &str,
    start_ms: i64,
    as_of: i64,
    settle_ms: i64,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
) -> Result<(Dataset, DatasetInfo), String> {
    let path = cache_path(data_dir, symbol);
    let cached = load_with_cache(
        &path,
        symbol,
        Timeframe::D1,
        start_ms,
        as_of,
        refresh,
        settle_ms,
        REGIME_SMA_DAYS,
        || {
            fetch_range(
                base_url,
                symbol,
                Timeframe::D1,
                start_ms,
                as_of,
                settle_ms,
                as_of,
            )
        },
    )?;
    println!(
        "{}: {} closed candles ({}) in {} page(s), {} duplicates dropped",
        symbol,
        cached.candles.len(),
        if cached.used_cache {
            "cache"
        } else {
            "fetched"
        },
        cached.provenance.pages,
        cached.provenance.duplicates_dropped
    );
    if cached.provenance.missing_candles > 0 {
        return Err(format!(
            "{symbol}: {} missing daily candles in the requested range; the daily strategy requires contiguous candles",
            cached.provenance.missing_candles
        ));
    }
    let dataset = load_csv(
        symbol,
        &path,
        Timeframe::D1,
        as_of,
        settle_ms,
        0,
        REGIME_SMA_DAYS,
        cached.candles.len(),
    )
    .map_err(|error| error.to_string())?;
    let info = DatasetInfo::from_provenance(&cached.provenance);
    Ok((dataset, info))
}

fn run_scenario(
    label: &str,
    scenario: &CostScenario,
    candles: &[analyzer::strategy::types::Candle],
    configs: &[(Mode, TsMomentumConfig)],
    all_splits: &[Split],
    boundaries: &[SplitBoundary],
    context: &ReportContext,
) -> Result<ScenarioRuns, String> {
    let mut runs = Vec::new();
    for variant in [
        Variant::Fixed,
        Variant::VolTarget,
        Variant::VolTargetDrawdown,
    ] {
        for (mode, config) in configs {
            for split in all_splits {
                let params = TsBacktestParams {
                    init_fund: context.init_fund,
                    fee_bps: scenario.fee_bps,
                    slip_bps: scenario.slip_bps,
                    borrow_bps_per_day: context.borrow_bps_per_day,
                    max_drawdown: context.max_drawdown,
                    drawdown_resume: context.drawdown_resume,
                    variant,
                    mode: *mode,
                    regime_filter: false,
                };
                let outcome = run_range(candles, config, &params, split.start, split.end)?;
                let metrics = outcome.metrics(context.init_fund);
                let benchmark_outcome = benchmark(candles, context.init_fund, scenario, *split)?;
                let benchmark_metrics = benchmark_outcome.metrics(context.init_fund);
                let boundary = boundaries
                    .iter()
                    .find(|boundary| boundary.name == split.name)
                    .expect("every split has a boundary");
                runs.push(RunReport {
                    scenario: ScenarioInfo {
                        label: label.to_string(),
                        fee_bps: scenario.fee_bps,
                        slip_bps: scenario.slip_bps,
                    },
                    variant,
                    mode: *mode,
                    split: *split,
                    split_start_time: boundary.start_time.clone(),
                    split_end_time: boundary.end_time.clone(),
                    candles: split.len(),
                    init_fund: context.init_fund,
                    borrow_bps_per_day: context.borrow_bps_per_day,
                    max_drawdown: context.max_drawdown,
                    drawdown_resume: context.drawdown_resume,
                    metrics,
                    benchmark: benchmark_metrics,
                    trades: outcome.trades.clone(),
                    trading_costs: outcome.trading_costs,
                    borrow_costs: outcome.borrow_costs,
                    traded_notional: outcome.traded_notional,
                    benchmark_trading_costs: benchmark_outcome.trading_costs,
                    benchmark_traded_notional: benchmark_outcome.traded_notional,
                    points: outcome.points,
                });
            }
        }
    }
    Ok(ScenarioRuns {
        info: ScenarioInfo {
            label: label.to_string(),
            fee_bps: scenario.fee_bps,
            slip_bps: scenario.slip_bps,
        },
        runs,
    })
}

fn print_full_period_summary(scenarios: &[ScenarioRuns]) {
    println!();
    println!(
        "{:<8} {:<48} {:<6} {:>10} {:>10} {:>10} {:>8} {:>6}",
        "costs", "variant", "mode", "net", "max_dd", "sharpe", "trades", "win"
    );
    for scenario in scenarios {
        for run in scenario
            .runs
            .iter()
            .filter(|run| run.split.name == SplitName::Full)
        {
            println!(
                "{:<8} {:<48} {:<6} {:>9.2}% {:>9.2}% {:>10} {:>8} {:>6}",
                scenario.info.label,
                run.variant.as_str(),
                run.mode.as_str(),
                run.metrics.net_cumulative_return.unwrap_or(0.0) * 100.0,
                run.metrics.max_drawdown.unwrap_or(0.0) * 100.0,
                run.metrics
                    .sharpe
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "—".to_string()),
                run.metrics.trades,
                run.metrics
                    .win_rate
                    .map(|value| format!("{:.1}%", value * 100.0))
                    .unwrap_or_else(|| "—".to_string())
            );
        }
    }
}

fn main() -> Result<(), String> {
    load_dotenv();
    share::logger::init_logger();
    let cli = Cli::parse();

    let symbols = parse_list(&cli.symbol);
    if symbols.len() != 1 {
        return Err("--symbol must resolve to exactly one symbol".into());
    }
    let symbol = &symbols[0];
    if !valid_symbol(symbol) {
        return Err(format!(
            "invalid symbol {symbol:?}; expected BASE-QUOTE matching ^[A-Z0-9]+-[A-Z0-9]+$"
        ));
    }
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
    }
    if cli.momentum_lookback == 0 {
        return Err("--momentum-lookback must be at least 1".into());
    }
    if cli.volatility_lookback == 0 {
        return Err("--volatility-lookback must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.max_dd == 0 || cli.max_dd > 100 {
        return Err("--max-dd must be in 1..=100".into());
    }
    if cli.drawdown_resume >= cli.max_dd {
        return Err("--drawdown-resume must be below --max-dd".into());
    }
    for (name, value) in [
        ("--fee-bps", cli.fee_bps),
        ("--slip-bps", cli.slip_bps),
        ("--stress-fee-bps", cli.stress_fee_bps),
        ("--stress-slip-bps", cli.stress_slip_bps),
        ("--borrow-bps-per-day", cli.borrow_bps_per_day),
    ] {
        if value > 1_000 {
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
        .saturating_sub(i64::from(cli.days).saturating_mul(DAY_MS));
    let (dataset, dataset_info) = load_dataset(
        symbol,
        start_ms,
        cli.as_of,
        settle_ms,
        &cli.data_dir,
        &base_url,
        cli.refresh,
    )?;

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
    let init_fund = Decimal::from(cli.init_fund);
    let max_drawdown = Decimal::from(cli.max_dd) / Decimal::from(100);
    let drawdown_resume = Decimal::from(cli.drawdown_resume) / Decimal::from(100);
    let borrow_bps_per_day = Decimal::from(cli.borrow_bps_per_day);

    let flat_cfg = TsMomentumConfig {
        momentum_lookback_days: cli.momentum_lookback,
        volatility_lookback_days: cli.volatility_lookback,
        negative_momentum_mode: NegativeMomentumMode::Flat,
        ..TsMomentumConfig::default()
    };
    let short_cfg = TsMomentumConfig {
        momentum_lookback_days: cli.momentum_lookback,
        volatility_lookback_days: cli.volatility_lookback,
        negative_momentum_mode: NegativeMomentumMode::Short,
        ..TsMomentumConfig::default()
    };
    let configs = [(Mode::Flat, flat_cfg.clone()), (Mode::Short, short_cfg)];

    let split_set = splits(dataset.candles.len());
    let all_splits = [
        Split::full(dataset.candles.len()),
        split_set.train,
        split_set.validation,
        split_set.test,
    ];
    let boundaries: Vec<SplitBoundary> = all_splits
        .iter()
        .map(|split| SplitBoundary::from_split(&dataset.candles, *split))
        .collect();

    let context = ReportContext {
        symbol: symbol.clone(),
        timeframe: Timeframe::D1.as_kucoin().to_string(),
        as_of_ms: cli.as_of,
        generated_at,
        days: cli.days,
        init_fund,
        config: flat_cfg,
        borrow_bps_per_day,
        max_drawdown,
        drawdown_resume,
        dataset: dataset_info,
        splits: boundaries.clone(),
        errors: Vec::new(),
    };

    let mut scenarios = Vec::new();
    for (label, scenario) in [("base", &base), ("stress", &stress)] {
        println!("{label}: running variants across full/train/validation/test splits...");
        scenarios.push(run_scenario(
            label,
            scenario,
            &dataset.candles,
            &configs,
            &all_splits,
            &boundaries,
            &context,
        )?);
    }

    write_reports(&cli.out_dir, &context, &scenarios)?;
    print_full_period_summary(&scenarios);
    let total_runs: usize = scenarios.iter().map(|scenario| scenario.runs.len()).sum();
    println!();
    println!(
        "wrote {total_runs} per-run report/results/equity artifacts and the combined report to {}",
        cli.out_dir.display()
    );
    Ok(())
}
