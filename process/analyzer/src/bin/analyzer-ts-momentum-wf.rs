use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, load_with_cache};
use analyzer::harness::ts_momentum_backtest::REGIME_SMA_DAYS;
use analyzer::harness::ts_momentum_report::DatasetInfo;
use analyzer::harness::ts_momentum_walkforward::{evaluate, write_reports, WfSettings};
use analyzer::identity;
use clap::Parser;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 86_400_000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-ts-momentum-wf",
    about = "Walk-forward the BTC daily time-series momentum improvement grid"
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
    #[arg(long, default_value = "process/analyzer/experiments/ts-momentum-wf")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/ts-momentum")]
    data_dir: PathBuf,

    /// Refetch instead of reusing a matching cache.
    #[arg(long)]
    refresh: bool,

    /// Initial fund for every window.
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

    /// Minimum OOS trades before a run can succeed.
    #[arg(long, default_value_t = 20)]
    min_oos_trades: usize,

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
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.min_oos_trades == 0 {
        return Err("--min-oos-trades must be at least 1".into());
    }
    for (name, value) in [
        ("--fee-bps", cli.fee_bps),
        ("--slip-bps", cli.slip_bps),
        ("--stress-fee-bps", cli.stress_fee_bps),
        ("--stress-slip-bps", cli.stress_slip_bps),
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

    let settings = WfSettings {
        generated_at,
        as_of_ms: cli.as_of,
        days: cli.days,
        init_fund: Decimal::from(cli.init_fund),
        base: CostScenario {
            label: "base".into(),
            fee_bps: Decimal::from(cli.fee_bps),
            slip_bps: Decimal::from(cli.slip_bps),
        },
        stress: CostScenario {
            label: "stress".into(),
            fee_bps: Decimal::from(cli.stress_fee_bps),
            slip_bps: Decimal::from(cli.stress_slip_bps),
        },
        min_oos_trades: cli.min_oos_trades,
    };

    println!(
        "running {} configs across {} walk-forward windows with {} OOS months each...",
        analyzer::harness::ts_momentum_walkforward::WfConfig::grid().len(),
        analyzer::harness::ts_momentum_walkforward::windows(dataset.candles.len()).len(),
        analyzer::harness::ts_momentum_walkforward::OOS_MONTHS,
    );
    let result = evaluate(&dataset.candles, dataset_info, &settings)?;
    write_reports(&cli.out_dir, &result)?;

    println!();
    println!(
        "verdict: {} ({} / {} windows with a successful top-3 config; {} / {} top-3 runs passed; baseline {} passed {} / {})",
        result.decision.verdict.as_str(),
        result.decision.windows_with_success,
        result.decision.total_windows,
        result.decision.successful_top_runs,
        result.decision.total_top_runs,
        result.baseline.label(),
        result.decision.baseline_successes,
        result.decision.baseline_runs
    );
    println!();
    for window in &result.windows {
        let labels: Vec<String> = window
            .top
            .iter()
            .map(|run| {
                format!(
                    "{} ({})",
                    run.config.label(),
                    if run.successful { "PASS" } else { "FAIL" }
                )
            })
            .collect();
        println!(
            "M1-M{} / OOS M{}-M{}: {}",
            window.split.months,
            window.split.months + 1,
            window.split.months + analyzer::harness::ts_momentum_walkforward::OOS_MONTHS,
            labels.join(", ")
        );
    }
    println!();
    println!("wrote report and results to {}", cli.out_dir.display());
    Ok(())
}
