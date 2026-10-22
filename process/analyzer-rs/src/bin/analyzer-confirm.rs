use analyzer_rs::config_env::{env_string, load_dotenv};
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{Dataset, EffectiveRange, Provenance, RequestedRange};
use analyzer_rs::harness::sweep::confirmation::{
    decide, evaluate_full_period, pool_parts, Decision,
};
use analyzer_rs::harness::sweep::data::{
    fetch_range, load_with_cache, verify_dataset_quality, CachedCandles,
};
use analyzer_rs::harness::sweep::execution::{ExecModel, ExecPartition};
use analyzer_rs::harness::sweep::freeze::{self, FrozenLock};
use analyzer_rs::identity;
use analyzer_rs::strategy::config::Config;
use clap::Parser;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-confirm",
    about = "Run the frozen fresh-period confirmation for max_age=4 + retest_2c_b2"
)]
struct Cli {
    /// Frozen lock file; relative paths resolve against --repo-root.
    #[arg(long, default_value = "freeze/confirmation-lock.json")]
    lock: PathBuf,

    /// Repository root used to verify the frozen-file hash.
    #[arg(long, default_value = ".")]
    repo_root: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/confirm")]
    data_dir: PathBuf,

    /// Artifact output directory.
    #[arg(long, default_value = "out/confirm")]
    out_dir: PathBuf,

    /// Refetch every symbol instead of reusing the cache.
    #[arg(long)]
    refresh: bool,

    /// KuCoin API base URL; defaults to KUCOIN_BASE_URL, then https://api.kucoin.com.
    #[arg(long)]
    base_url: Option<String>,

    /// Candle settle time in milliseconds.
    #[arg(long, default_value_t = DEFAULT_SETTLE_MS)]
    settle_ms: i64,

    /// Initial account fund per symbol.
    #[arg(long, default_value_t = 1000)]
    init_fund: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ResolvedConfig {
    rsi_period: usize,
    rsi_order: usize,
    max_age: Option<usize>,
    sma_enabled: bool,
    sma_period: usize,
    atr_period: usize,
    atr_sl_multiplier: Decimal,
    atr_tp_multiplier: Decimal,
    price_diff_threshold: Decimal,
    atr_pct_high: Decimal,
    atr_high_adjust: Decimal,
    atr_sl_max_pct: Decimal,
    window: usize,
}

impl ResolvedConfig {
    fn new(config: &Config, max_age: Option<usize>) -> Self {
        Self {
            rsi_period: config.rsi_period,
            rsi_order: config.rsi_order,
            max_age,
            sma_enabled: config.sma_enabled,
            sma_period: config.sma_period,
            atr_period: config.atr_period,
            atr_sl_multiplier: config.atr_sl_multiplier,
            atr_tp_multiplier: config.atr_tp_multiplier,
            price_diff_threshold: config.price_diff_threshold,
            atr_pct_high: config.atr_pct_high,
            atr_high_adjust: config.atr_high_adjust,
            atr_sl_max_pct: config.atr_sl_max_pct,
            window: config.window,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DatasetSummary {
    symbol: String,
    timeframe: String,
    requested_first_ms: i64,
    requested_last_ms: i64,
    first_open_time_ms: i64,
    last_open_time_ms: i64,
    candles: usize,
    requested_candles: usize,
    expected_candles: usize,
    missing_candles: usize,
    duplicates_dropped: usize,
    pages: usize,
    fetched_at_ms: i64,
    settle_ms: i64,
    content_sha256: String,
    used_cache: bool,
}

impl DatasetSummary {
    fn from_cached(cached: &CachedCandles) -> Self {
        let provenance = &cached.provenance;
        Self {
            symbol: provenance.symbol.clone(),
            timeframe: provenance.timeframe.clone(),
            requested_first_ms: provenance.requested_first_ms,
            requested_last_ms: provenance.requested_last_ms,
            first_open_time_ms: provenance.first_open_time_ms,
            last_open_time_ms: provenance.last_open_time_ms,
            candles: provenance.candles,
            requested_candles: provenance.requested_candles,
            expected_candles: provenance.expected_candles,
            missing_candles: provenance.missing_candles,
            duplicates_dropped: provenance.duplicates_dropped,
            pages: provenance.pages,
            fetched_at_ms: provenance.fetched_at_ms,
            settle_ms: provenance.settle_ms,
            content_sha256: provenance.content_sha256.clone(),
            used_cache: cached.used_cache,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConfirmationRow {
    symbol_or_pooled: String,
    signals: usize,
    filled: usize,
    expired: usize,
    skipped_funds: usize,
    invalid_levels: usize,
    capital_fill_rate: Decimal,
    execution_fill_rate: Decimal,
    trades: usize,
    net_expectancy: Decimal,
    net_profit_factor: Option<Decimal>,
    win_rate: Decimal,
    max_drawdown: Decimal,
    max_drawdown_pct: Decimal,
    net_pnl: Decimal,
    expectancy_stderr: Decimal,
}

impl ConfirmationRow {
    fn new(symbol_or_pooled: &str, part: &ExecPartition) -> Self {
        Self {
            symbol_or_pooled: symbol_or_pooled.to_string(),
            signals: part.signals,
            filled: part.filled,
            expired: part.expired,
            skipped_funds: part.skipped_funds,
            invalid_levels: part.invalid_levels,
            capital_fill_rate: part.capital_fill_rate,
            execution_fill_rate: part.execution_fill_rate,
            trades: part.metrics.trades,
            net_expectancy: part.metrics.net_expectancy,
            net_profit_factor: part.metrics.net_profit_factor,
            win_rate: part.metrics.win_rate,
            max_drawdown: part.metrics.max_drawdown,
            max_drawdown_pct: part.metrics.max_drawdown_pct,
            net_pnl: part.metrics.net_pnl,
            expectancy_stderr: part.metrics.expectancy_stderr,
        }
    }
}

#[derive(Debug, Serialize)]
struct ConfirmationResults {
    generated_at: String,
    lock_path: String,
    repo_root: String,
    lock: FrozenLock,
    config: ResolvedConfig,
    model: ExecModel,
    data_dir: String,
    base_url: String,
    settle_ms: i64,
    init_fund: i64,
    window: usize,
    max_age: Option<usize>,
    datasets: BTreeMap<String, DatasetSummary>,
    rows: Vec<ConfirmationRow>,
    decision: Decision,
}

fn resolve_generated_at(value: Option<&str>) -> Result<String, String> {
    match value {
        Some(value) => {
            chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
                format!("invalid --generated-at {value:?}; expected an RFC 3339 timestamp")
            })?;
            Ok(value.to_string())
        }
        None => Ok(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    }
}

fn resolve_base_url(value: Option<String>) -> String {
    value
        .or_else(|| env_string("KUCOIN_BASE_URL"))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

fn absolute_from(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn dataset_from_cache(
    symbol: &str,
    timeframe: Timeframe,
    end_ms: i64,
    cached: &CachedCandles,
) -> Dataset {
    let provenance = &cached.provenance;
    Dataset {
        symbol: symbol.to_string(),
        candles: cached.candles.clone(),
        provenance: Provenance {
            source: "kucoin".into(),
            requested: RequestedRange {
                limit: provenance.requested_candles,
                timeframe: timeframe.as_kucoin().to_string(),
                as_of_ms: end_ms,
            },
            effective_range: EffectiveRange {
                first_open_time_ms: provenance.first_open_time_ms,
                last_open_time_ms: provenance.last_open_time_ms,
                candles: provenance.candles,
            },
            content_sha256: provenance.content_sha256.clone(),
            fetched_at_ms: Some(provenance.fetched_at_ms),
            gaps: provenance.missing_candles,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn load_symbol(
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    end_ms: i64,
    settle_ms: i64,
    window: usize,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
    now_ms: i64,
) -> Result<(Dataset, DatasetSummary), String> {
    let cache_path = data_dir.join(format!("{symbol}_{}.csv", timeframe.as_kucoin()));
    let cached = load_with_cache(
        &cache_path,
        symbol,
        timeframe,
        start_ms,
        end_ms,
        refresh,
        settle_ms,
        window,
        || {
            fetch_range(
                base_url, symbol, timeframe, start_ms, end_ms, settle_ms, now_ms,
            )
        },
    )?;
    verify_dataset_quality(
        &cached.provenance,
        &cached.candles,
        start_ms,
        end_ms,
        timeframe,
    )?;
    let summary = DatasetSummary::from_cached(&cached);
    let dataset = dataset_from_cache(symbol, timeframe, end_ms, &cached);
    Ok((dataset, summary))
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

fn write_csv(path: &Path, rows: &[ConfirmationRow]) -> Result<(), String> {
    ensure_parent(path)?;
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record([
            "symbol_or_pooled",
            "signals",
            "filled",
            "expired",
            "skipped_funds",
            "invalid_levels",
            "capital_fill_rate",
            "execution_fill_rate",
            "trades",
            "net_expectancy",
            "net_profit_factor",
            "win_rate",
            "max_drawdown",
            "max_drawdown_pct",
            "net_pnl",
            "expectancy_stderr",
        ])
        .map_err(|error| error.to_string())?;
    for row in rows {
        writer
            .write_record([
                row.symbol_or_pooled.clone(),
                row.signals.to_string(),
                row.filled.to_string(),
                row.expired.to_string(),
                row.skipped_funds.to_string(),
                row.invalid_levels.to_string(),
                row.capital_fill_rate.to_string(),
                row.execution_fill_rate.to_string(),
                row.trades.to_string(),
                row.net_expectancy.to_string(),
                row.net_profit_factor
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                row.win_rate.to_string(),
                row.max_drawdown.to_string(),
                row.max_drawdown_pct.to_string(),
                row.net_pnl.to_string(),
                row.expectancy_stderr.to_string(),
            ])
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_json(path: &Path, results: &ConfirmationResults) -> Result<(), String> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn display_decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn display_pct(value: Decimal) -> String {
    format!("{}%", (value * Decimal::from(100)).round_dp(2))
}

/// Formats a metric that is already stored as a percentage (for example `win_rate`).
fn display_percent_value(value: Decimal) -> String {
    format!("{}%", value.round_dp(2))
}

fn display_profit_factor(value: Option<Decimal>) -> String {
    value
        .map(|value| value.round_dp(2).to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn write_markdown(path: &Path, results: &ConfirmationResults) -> Result<(), String> {
    ensure_parent(path)?;
    let mut out = String::new();
    writeln!(out, "# Analyzer Fresh Confirmation").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Pre-registered one-shot confirmation. No tuning is permitted after this run."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 1. Freeze").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Generated at | {} |", results.generated_at)
        .expect("writing to String cannot fail");
    writeln!(out, "| Lock path | {} |", results.lock_path).expect("writing to String cannot fail");
    writeln!(out, "| Repository root | {} |", results.repo_root)
        .expect("writing to String cannot fail");
    writeln!(out, "| Frozen commit | {} |", results.lock.git_commit)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Frozen-file SHA-256 (verified) | {} |",
        results.lock.signal_generator_sha256
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Lock SHA-256 (verified) | {} |",
        freeze::EXPECTED_LOCK_SHA256
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Lock created at | {} |", results.lock.created_at)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 2. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Requested range | {} ({}) → {} ({}) |",
        iso(results.lock.dataset.start_ms),
        results.lock.dataset.start_ms,
        iso(results.lock.dataset.as_of_ms),
        results.lock.dataset.as_of_ms
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", results.lock.dataset.timeframe)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbols | {} |",
        results.lock.dataset.symbols.join(", ")
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Data dir | {} |", results.data_dir).expect("writing to String cannot fail");
    writeln!(out, "| Base URL | {} |", results.base_url).expect("writing to String cannot fail");
    writeln!(out, "| Settle (ms) | {} |", results.settle_ms)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Initial fund per symbol | {} (CLI-configurable; the decision is scale-invariant) |",
        results.init_fund
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Warm-up window | {} |", results.window)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | First open | Last open | Candles | Requested | Expected | Missing | Duplicates | Content SHA-256 | Cache |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---|---|---|---|---|---|")
        .expect("writing to String cannot fail");
    for dataset in results.datasets.values() {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            dataset.symbol,
            iso(dataset.first_open_time_ms),
            iso(dataset.last_open_time_ms),
            dataset.candles,
            dataset.requested_candles,
            dataset.expected_candles,
            dataset.missing_candles,
            dataset.duplicates_dropped,
            dataset.content_sha256,
            if dataset.used_cache {
                "reused"
            } else {
                "fetched"
            }
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 3. Frozen config").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    let config = &results.config;
    for (name, value) in [
        ("RSI_PERIOD", config.rsi_period.to_string()),
        ("RSI_DIVERGENCE_ORDER", config.rsi_order.to_string()),
        (
            "ANALYZER_MAX_DIVERGENCE_AGE_CANDLES",
            config
                .max_age
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".into()),
        ),
        ("SMA_ENABLED", config.sma_enabled.to_string()),
        ("SMA_PERIOD", config.sma_period.to_string()),
        ("ATR_PERIOD", config.atr_period.to_string()),
        ("ATR_SL_MULTIPLIER", config.atr_sl_multiplier.to_string()),
        ("ATR_TP_MULTIPLIER", config.atr_tp_multiplier.to_string()),
        (
            "PRICE_DIFF_THRESHOLD",
            config.price_diff_threshold.to_string(),
        ),
        ("ATR_PCT_HIGH", config.atr_pct_high.to_string()),
        ("ATR_HIGH_ADJUST", config.atr_high_adjust.to_string()),
        ("ATR_SL_MAX_PCT", config.atr_sl_max_pct.to_string()),
        ("ANALYZER_WINDOW_CANDLES", config.window.to_string()),
    ] {
        writeln!(out, "| {name} | {value} |").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 4. Execution model").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Label | {} |", results.model.label).expect("writing to String cannot fail");
    writeln!(out, "| Maker | {} bps |", results.model.maker_bps)
        .expect("writing to String cannot fail");
    writeln!(out, "| Taker | {} bps |", results.model.taker_bps)
        .expect("writing to String cannot fail");
    writeln!(out, "| Slippage | {} bps |", results.model.slip_bps)
        .expect("writing to String cannot fail");
    match &results.model.entry {
        analyzer_rs::harness::sweep::execution::EntryMode::Market => {
            writeln!(out, "| Entry | market |").expect("writing to String cannot fail");
        }
        analyzer_rs::harness::sweep::execution::EntryMode::Limit {
            valid_candles,
            buffer_bps,
            offset_bps,
        } => {
            writeln!(out, "| Entry | limit |").expect("writing to String cannot fail");
            writeln!(out, "| Valid candles | {valid_candles} |")
                .expect("writing to String cannot fail");
            writeln!(out, "| Buffer | {buffer_bps} bps |").expect("writing to String cannot fail");
            writeln!(out, "| Offset | {offset_bps} bps |").expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 5. Metrics").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol or pooled | Trades | Net expectancy | SE | PF | Win rate | Max DD | Max DD % | Net PnL | Signals | Filled | Exec fill rate | Capital fill rate | Skipped funds |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    )
    .expect("writing to String cannot fail");
    for row in &results.rows {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.symbol_or_pooled,
            row.trades,
            display_decimal(row.net_expectancy),
            display_decimal(row.expectancy_stderr),
            display_profit_factor(row.net_profit_factor),
            display_percent_value(row.win_rate),
            display_decimal(row.max_drawdown),
            display_percent_value(row.max_drawdown_pct),
            display_decimal(row.net_pnl),
            row.signals,
            row.filled,
            display_pct(row.execution_fill_rate),
            display_pct(row.capital_fill_rate),
            row.skipped_funds
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 6. Decision").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if results.decision.passed {
        writeln!(out, "**confirmation passed**").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "All pre-registered conditions were met. This confirms the result for paper/shadow evaluation, not live deployment."
        )
        .expect("writing to String cannot fail");
    } else {
        writeln!(out, "**confirmation failed**").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(out, "Pre-registered reasons:").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        for reason in &results.decision.reasons {
            writeln!(out, "- {reason}").expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Rule | Threshold | Observed |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|").expect("writing to String cannot fail");
    let pooled = results
        .rows
        .iter()
        .find(|row| row.symbol_or_pooled == "pooled");
    if let Some(pooled) = pooled {
        writeln!(
            out,
            "| Pooled closed trades | ≥ {} | {} |",
            results.lock.decision_rule.min_pooled_trades, pooled.trades
        )
        .expect("writing to String cannot fail");
        writeln!(
            out,
            "| Pooled net expectancy | {} 0 | {} |",
            if results.lock.decision_rule.require_positive_net_expectancy {
                ">"
            } else {
                "any"
            },
            display_decimal(pooled.net_expectancy)
        )
        .expect("writing to String cannot fail");
        writeln!(
            out,
            "| Pooled net profit factor | > {} | {} |",
            results.lock.decision_rule.require_profit_factor_above,
            display_profit_factor(pooled.net_profit_factor)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(
        out,
        "| Positive symbols | ≥ {} | {} of {} |",
        results.lock.decision_rule.min_positive_symbols,
        results.decision.positive_symbols,
        results.decision.symbol_count
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn main() -> Result<(), String> {
    load_dotenv();
    let cli = Cli::parse();

    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    if cli.settle_ms < 0 {
        return Err("--settle-ms must be non-negative".into());
    }
    let generated_at = resolve_generated_at(cli.generated_at.as_deref())?;
    let base_url = resolve_base_url(cli.base_url.clone());
    let lock_path = absolute_from(&cli.repo_root, &cli.lock);

    freeze::verify_lock_digest(&lock_path)?;
    let lock = freeze::load(&lock_path)?;
    freeze::verify(&cli.repo_root, &lock)?;
    freeze::validate_lock(&lock)?;
    let config = lock.config.to_config()?;
    let model = lock.model.to_exec_model()?;
    let timeframe = Timeframe::from_kucoin(&lock.dataset.timeframe).ok_or_else(|| {
        format!(
            "unsupported locked timeframe {:?}; expected 15min, 1hour, 4hour, or 1day",
            lock.dataset.timeframe
        )
    })?;
    let start_ms = lock.dataset.start_ms;
    let end_ms = lock.dataset.as_of_ms;
    let now_ms = identity::now_ms();

    let mut per_symbol: Vec<(String, ExecPartition)> =
        Vec::with_capacity(lock.dataset.symbols.len());
    let mut datasets: BTreeMap<String, DatasetSummary> = BTreeMap::new();
    for symbol in &lock.dataset.symbols {
        let (dataset, summary) = load_symbol(
            symbol,
            timeframe,
            start_ms,
            end_ms,
            cli.settle_ms,
            config.window,
            &cli.data_dir,
            &base_url,
            cli.refresh,
            now_ms,
        )?;
        if dataset.candles.len() < config.window + 2 {
            return Err(format!(
                "{symbol}: only {} candles available; need at least {}",
                dataset.candles.len(),
                config.window + 2
            ));
        }
        let partition = evaluate_full_period(
            &dataset,
            &config,
            lock.config.max_age,
            config.window,
            cli.init_fund,
            &model,
        )
        .map_err(|error| format!("{symbol}: {error}"))?;
        datasets.insert(symbol.clone(), summary);
        per_symbol.push((symbol.clone(), partition));
    }

    let parts: Vec<ExecPartition> = per_symbol
        .iter()
        .map(|(_, partition)| partition.clone())
        .collect();
    let pooled = pool_parts(&parts);
    let decision = decide(&pooled, &parts, &lock.decision_rule);

    let mut rows: Vec<ConfirmationRow> = per_symbol
        .iter()
        .map(|(symbol, partition)| ConfirmationRow::new(symbol, partition))
        .collect();
    rows.push(ConfirmationRow::new("pooled", &pooled));

    let results = ConfirmationResults {
        generated_at,
        lock_path: lock_path.display().to_string(),
        repo_root: cli.repo_root.display().to_string(),
        config: ResolvedConfig::new(&config, lock.config.max_age),
        model,
        data_dir: cli.data_dir.display().to_string(),
        base_url,
        settle_ms: cli.settle_ms,
        init_fund: cli.init_fund,
        window: config.window,
        max_age: lock.config.max_age,
        lock,
        datasets,
        rows,
        decision,
    };

    let csv_path = cli.out_dir.join("confirmation.csv");
    let json_path = cli.out_dir.join("results.json");
    let report_path = cli.out_dir.join("report.md");
    write_csv(&csv_path, &results.rows)?;
    write_json(&json_path, &results)?;
    write_markdown(&report_path, &results)?;

    if results.decision.passed {
        println!("confirmation passed");
    } else {
        println!("confirmation failed");
    }
    for reason in &results.decision.reasons {
        println!("  - {reason}");
    }
    println!("wrote {}", csv_path.display());
    println!("wrote {}", json_path.display());
    println!("wrote {}", report_path.display());

    Ok(())
}
