use clap::{Parser, Subcommand};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use strategy_analyzer::harness::dataset::{
    dataset_hash, load_csv, load_csv_with_policy, GapPolicy,
};
use strategy_analyzer::harness::fetch;
use strategy_analyzer::harness::lock::{
    self, BacktestParams, LockContents, LockedConstants, OosLogRecord,
};
use strategy_analyzer::harness::perturb;
use strategy_analyzer::harness::report::{
    adversarial_composite, calibration_wrong_fraction, cushion_warning, summarize,
    time_blocked_bootstrap, validate_aggregates, TradeRecord, AGG_TOLERANCE, BOOTSTRAP_BLOCK_MS,
};
use strategy_analyzer::harness::run::{count_signals, emitted_signals, run, RunResult};
use strategy_analyzer::harness::sim::SimParams;
use strategy_analyzer::harness::tick_source::{aggregate_4h_bucket, group_4h_buckets, PathPolicy};
use strategy_analyzer::harness::verdict::{
    verdict, Regime, VerdictInputs, VerdictOutcome, BOOTSTRAP_RESAMPLES, BOOTSTRAP_SEED,
};
use strategy_analyzer::strategy::config::{config_version, interval_to_ms, Config, TargetMode};
use strategy_analyzer::strategy::decimal::round_to;
use strategy_analyzer::strategy::error::StrategyError;
use strategy_analyzer::strategy::types::Candle;

const KUCOIN_BASE: &str = "https://api.kucoin.com";

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// One cached OHLCV row: `(open_time_ms, open, high, low, close)`.
type CacheRow = (i64, String, String, String, String);

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fetch OHLCV and cache to a file. Never used by `backtest`. Refuses OOS-window ranges until
    /// the freeze lock exists.
    FetchData {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "4hour")]
        interval: String,
        #[arg(long)]
        start_s: i64,
        #[arg(long)]
        end_s: i64,
        #[arg(long)]
        out: String,
        /// Freeze-lock path; its existence (not contents) lifts the OOS-window fetch guard.
        #[arg(long)]
        lock: Option<String>,
    },
    /// Seal the pre-registered freeze lock from the current datasets and constants.
    FreezeLock {
        /// `SYMBOL=PATH`, repeatable.
        #[arg(long = "data", value_name = "SYMBOL=PATH", required = true)]
        data: Vec<String>,
        /// `SYMBOL=DP`, repeatable; required for every symbol.
        #[arg(long = "price-scale", value_name = "SYMBOL=DP")]
        price_scale: Vec<String>,
        #[arg(long, default_value = "both")]
        policy: String,
        #[arg(long, default_value = "both")]
        mode: String,
        /// Candle interval the lock seals (KuCoin string, e.g. `4hour`, `5min`). The frozen
        /// `config_version` depends on it, so the lock only matches a `backtest` run on the same
        /// interval.
        #[arg(long, default_value = "4hour")]
        candle_interval: String,
        #[arg(long)]
        out: String,
    },
    /// Run the backtest offline against cached datasets. `--data SYMBOL=PATH` is repeatable; every
    /// symbol needs its own `--price-scale SYMBOL=DP`.
    Backtest {
        #[arg(long = "data", value_name = "SYMBOL=PATH", required = true)]
        data: Vec<String>,
        #[arg(long = "price-scale", value_name = "SYMBOL=DP")]
        price_scale: Vec<String>,
        #[arg(long, default_value = "both")]
        policy: String,
        #[arg(long, default_value = "both")]
        mode: String,
        #[arg(long)]
        lock: Option<String>,
        #[arg(long, default_value_t = false)]
        oos: bool,
        #[arg(long, default_value_t = false)]
        oos_consumed: bool,
        /// Candle interval of the `--data` CSVs (KuCoin string, e.g. `4hour`, `5min`). The data must
        /// actually be on this interval: the core rejects candles not spaced by it as `GappedCandle`.
        #[arg(long, default_value = "4hour")]
        candle_interval: String,
    },
    /// Count-only in-sample run (§10.4 Step 0): per-symbol signal counts and reason counts, no PnL,
    /// no trades, no verdict. Refuses any row at/after the in-sample/OOS boundary.
    CountSignals {
        #[arg(long = "data", value_name = "SYMBOL=PATH", required = true)]
        data: Vec<String>,
        #[arg(long = "price-scale", value_name = "SYMBOL=DP")]
        price_scale: Vec<String>,
        #[arg(long, default_value = "both")]
        policy: String,
        /// Candle interval of the `--data` CSVs (KuCoin string, e.g. `4hour`, `5min`). The data must
        /// actually be on this interval: the core rejects candles not spaced by it as `GappedCandle`.
        #[arg(long, default_value = "4hour")]
        candle_interval: String,
    },
    /// Dump every emitted `Signal` as one JSON object per line (in-sample only; no PnL, no trades,
    /// no verdict). Refuses any row at/after the in-sample/OOS boundary, like `count-signals`.
    DumpSignals {
        #[arg(long = "data", value_name = "SYMBOL=PATH", required = true)]
        data: Vec<String>,
        #[arg(long = "price-scale", value_name = "SYMBOL=DP")]
        price_scale: Vec<String>,
        #[arg(long, default_value = "both")]
        policy: String,
        /// Target mode (`fixed_r` or `structural`); defaults to `fixed_r`. `structural` changes the
        /// emitted take-profit level, so the mode is an explicit choice.
        #[arg(long, default_value = "fixed_r")]
        mode: String,
        /// Candle interval of the `--data` CSVs (KuCoin string, e.g. `4hour`, `5min`). The data must
        /// actually be on this interval: the core rejects candles not spaced by it as `GappedCandle`.
        #[arg(long, default_value = "4hour")]
        candle_interval: String,
    },
    /// Fast candle-interval sweep. Per interval: the signal count and the net-of-cost R
    /// (fees/slippage/borrow), with **no** perturbation/verdict diagnostics. In-sample/exploratory
    /// only, so a long sweep cannot time out on the diagnostics the `backtest` command runs.
    Benchmark {
        #[arg(long)]
        symbol: String,
        #[arg(long = "price-scale", value_name = "SYMBOL=DP")]
        price_scale: Vec<String>,
        /// Comma-separated KuCoin intervals (unknown entries rejected), e.g. `5min,15min,1hour,4hour`.
        #[arg(long, default_value = "5min,15min,1hour,4hour")]
        intervals: String,
        /// `INTERVAL=PATH`, repeatable: supply a cached CSV per interval (skips fetching it).
        #[arg(long = "data", value_name = "INTERVAL=PATH")]
        data: Vec<String>,
        /// In-sample fetch range (seconds); used only when an interval actually needs fetching.
        #[arg(long)]
        start_s: Option<i64>,
        /// In-sample fetch range (seconds); used only when an interval actually needs fetching.
        #[arg(long)]
        end_s: Option<i64>,
        /// Cache directory for `{cache_dir}/{symbol}_{interval}.csv`.
        #[arg(long, default_value = "data")]
        cache_dir: String,
        /// Use `--data`/cache only; never fetch. An interval with no cached file becomes a `no data`
        /// row rather than aborting the run.
        #[arg(long, default_value_t = false)]
        no_fetch: bool,
        #[arg(long, default_value = "5")]
        fee_bps: String,
        #[arg(long, default_value = "5")]
        entry_slip_bps: String,
        #[arg(long, default_value = "15")]
        stop_slip_bps: String,
        #[arg(long, default_value = "2")]
        borrow_bps_per_day: String,
        #[arg(long, default_value = "both")]
        policy: String,
    },
    /// Validate a 1m-derived 4H aggregate against an independently fetched 4H series (offline).
    /// Refuses the run above `AGG_TOLERANCE`; never silently patches. Network stays in `fetch-data`.
    Calibrate {
        #[arg(long)]
        one_minute: String,
        #[arg(long)]
        fetched_4h: String,
    },
}

fn main() -> Res<()> {
    match Cli::parse().cmd {
        Cmd::FetchData {
            symbol,
            interval,
            start_s,
            end_s,
            out,
            lock,
        } => fetch_data(&symbol, &interval, start_s, end_s, &out, lock.as_deref()),
        Cmd::FreezeLock {
            data,
            price_scale,
            policy,
            mode,
            candle_interval,
            out,
        } => freeze_lock(&data, &price_scale, &policy, &mode, &candle_interval, &out),
        Cmd::Backtest {
            data,
            price_scale,
            policy,
            mode,
            lock,
            oos,
            oos_consumed,
            candle_interval,
        } => backtest(
            &data,
            &price_scale,
            &policy,
            &mode,
            lock,
            oos,
            oos_consumed,
            &candle_interval,
        ),
        Cmd::CountSignals {
            data,
            price_scale,
            policy,
            candle_interval,
        } => count_signals_cmd(&data, &price_scale, &policy, &candle_interval),
        Cmd::DumpSignals {
            data,
            price_scale,
            policy,
            mode,
            candle_interval,
        } => dump_signals_cmd(&data, &price_scale, &policy, &mode, &candle_interval),
        Cmd::Benchmark {
            symbol,
            price_scale,
            intervals,
            data,
            start_s,
            end_s,
            cache_dir,
            no_fetch,
            fee_bps,
            entry_slip_bps,
            stop_slip_bps,
            borrow_bps_per_day,
            policy,
        } => benchmark(
            &symbol,
            &price_scale,
            &intervals,
            &data,
            start_s,
            end_s,
            &cache_dir,
            no_fetch,
            &fee_bps,
            &entry_slip_bps,
            &stop_slip_bps,
            &borrow_bps_per_day,
            &policy,
        ),
        Cmd::Calibrate {
            one_minute,
            fetched_4h,
        } => calibrate(&one_minute, &fetched_4h),
    }
}

// ---------------------------------------------------------------------------------------------
// CLI argument parsing helpers
// ---------------------------------------------------------------------------------------------

/// Parse repeatable `SYMBOL=PATH` data arguments. Symbol/price scale are never hard-coded.
fn parse_data(items: &[String]) -> Res<Vec<(String, String)>> {
    let mut out = Vec::new();
    for item in items {
        let (symbol, path) = item
            .split_once('=')
            .ok_or_else(|| format!("--data must be SYMBOL=PATH, got {item:?}"))?;
        if symbol.is_empty() || path.is_empty() {
            return Err(format!("--data must be SYMBOL=PATH, got {item:?}").into());
        }
        out.push((symbol.to_string(), path.to_string()));
    }
    if out.is_empty() {
        return Err("--data requires at least one SYMBOL=PATH".into());
    }
    Ok(out)
}

/// Parse repeatable `SYMBOL=DP` price-scale arguments.
fn parse_scales(items: &[String]) -> Res<BTreeMap<String, u32>> {
    let mut out = BTreeMap::new();
    for item in items {
        let (symbol, dp) = item
            .split_once('=')
            .ok_or_else(|| format!("--price-scale must be SYMBOL=DP, got {item:?}"))?;
        let dp: u32 = dp
            .parse()
            .map_err(|_| format!("--price-scale DP must be an integer, got {item:?}"))?;
        out.insert(symbol.to_string(), dp);
    }
    Ok(out)
}

/// Build the config for `symbols`, taking each price scale from `scales` (no silent default) and the
/// candle interval from `candle_interval` (a KuCoin interval string; no silent default there either).
fn build_config(
    symbols: &[String],
    scales: &BTreeMap<String, u32>,
    candle_interval: &str,
) -> Res<Config> {
    for s in symbols {
        if !scales.contains_key(s) {
            return Err(format!(
                "no --price-scale for {s}; every symbol needs an explicit price scale (no silent default)"
            )
            .into());
        }
    }
    let refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    let mut cfg = Config::default_for(&refs);
    cfg.price_scale = scales.clone();
    cfg.candle_interval_ms = interval_to_ms(candle_interval)
        .map_err(|e| format!("--candle-interval {candle_interval:?}: {e}"))?;
    cfg.validate()?;
    Ok(cfg)
}

fn parse_policies(policy: &str) -> Res<Vec<PathPolicy>> {
    Ok(match policy {
        "low_first" => vec![PathPolicy::LowFirst],
        "high_first" => vec![PathPolicy::HighFirst],
        "both" => vec![PathPolicy::LowFirst, PathPolicy::HighFirst],
        other => return Err(format!("unknown policy: {other}").into()),
    })
}

fn parse_modes(mode: &str) -> Res<Vec<TargetMode>> {
    Ok(match mode {
        "fixed_r" => vec![TargetMode::FixedR],
        "structural" => vec![TargetMode::Structural],
        "both" => vec![TargetMode::FixedR, TargetMode::Structural],
        other => return Err(format!("unknown mode: {other}").into()),
    })
}

/// The pre-registered backtest parameters recorded in the lock (constants read from their single
/// sources of truth).
fn backtest_params(policy: &str, mode: &str, p: SimParams) -> BacktestParams {
    BacktestParams {
        policy: policy.to_string(),
        mode: mode.to_string(),
        fee_bps: p.fee_bps.to_string(),
        entry_slip_bps: p.entry_slip_bps.to_string(),
        stop_slip_bps: p.stop_slip_bps.to_string(),
        borrow_bps_per_day: p.borrow_bps_per_day.to_string(),
        // The entry fill rule: the touching tick fills (latency is a fill-model assumption).
        entry_fill: "touching_tick".to_string(),
        constants: LockedConstants::from_constants(),
    }
}

/// Assemble the freeze-lock document for a run over `dataset_hashes` with the pre-registered
/// boundary and git commit.
fn build_lock(
    policy: &str,
    mode: &str,
    params: SimParams,
    config_version: String,
    dataset_hashes: BTreeMap<String, String>,
) -> Res<LockContents> {
    Ok(LockContents {
        git_commit: lock::git_commit_hash()?,
        oos_boundary_ms: lock::OOS_BOUNDARY_MS,
        config_version,
        dataset_hashes,
        params: backtest_params(policy, mode, params),
    })
}

// ---------------------------------------------------------------------------------------------
// Shared run helpers
// ---------------------------------------------------------------------------------------------

/// One loaded dataset: its symbol, source path, candles, and stable hash.
struct Loaded {
    symbol: String,
    path: String,
    candles: Vec<Candle>,
    hash: String,
}

fn load_datasets(data: &[String]) -> Res<Vec<Loaded>> {
    let mut out = Vec::new();
    for (symbol, path) in parse_data(data)? {
        let candles = load_csv(&path)?;
        let hash = dataset_hash(&path)?;
        out.push(Loaded {
            symbol,
            path,
            candles,
            hash,
        });
    }
    Ok(out)
}

/// Run every policy for one symbol.
fn run_policies(
    symbol: &str,
    candles: &[Candle],
    policies: &[PathPolicy],
    cfg: &Config,
    params: SimParams,
) -> Result<Vec<(PathPolicy, RunResult)>, StrategyError> {
    policies
        .iter()
        .map(|p| Ok((*p, run(symbol, candles, *p, cfg, params)?)))
        .collect()
}

/// The full per-policy trade set across every symbol (used for the composite and the
/// "positive under both path policies" gate).
fn policy_trades(runs: &[(String, PathPolicy, RunResult)], policy: PathPolicy) -> Vec<TradeRecord> {
    runs.iter()
        .filter(|(_, p, _)| *p == policy)
        .flat_map(|(_, _, r)| r.trades.clone())
        .collect()
}

/// Pre-registered sim defaults (freeze-lock contents): 5 bps fee/side, 5 bps entry slip,
/// 15 bps (worse) stop slip, 2 bps/day borrow.
fn default_sim_params() -> SimParams {
    SimParams {
        fee_bps: Decimal::from(5),
        entry_slip_bps: Decimal::from(5),
        stop_slip_bps: Decimal::from(15),
        borrow_bps_per_day: Decimal::from(2),
    }
}

fn zero_cost_params(_p: SimParams) -> SimParams {
    SimParams {
        fee_bps: Decimal::ZERO,
        entry_slip_bps: Decimal::ZERO,
        stop_slip_bps: Decimal::ZERO,
        borrow_bps_per_day: Decimal::ZERO,
    }
}

/// Scale every cost rate by `num/den` (used for the pre-registered 1.5× cost stress).
fn scaled_cost_params(p: SimParams, num: Decimal, den: Decimal) -> SimParams {
    SimParams {
        fee_bps: p.fee_bps * num / den,
        entry_slip_bps: p.entry_slip_bps * num / den,
        stop_slip_bps: p.stop_slip_bps * num / den,
        borrow_bps_per_day: p.borrow_bps_per_day * num / den,
    }
}

/// Decimal portfolio mean R (0 when there are no trades).
fn mean_r(trades: &[TradeRecord]) -> Decimal {
    if trades.is_empty() {
        return Decimal::ZERO;
    }
    trades.iter().map(|t| t.r).sum::<Decimal>() / Decimal::from(trades.len() as u64)
}

fn policy_name(p: PathPolicy) -> &'static str {
    match p {
        PathPolicy::LowFirst => "low_first",
        PathPolicy::HighFirst => "high_first",
    }
}

fn mode_name(m: TargetMode) -> &'static str {
    match m {
        TargetMode::FixedR => "fixed_r",
        TargetMode::Structural => "structural",
    }
}

fn outcome_name(o: VerdictOutcome) -> &'static str {
    match o {
        VerdictOutcome::Insufficient => "insufficient",
        VerdictOutcome::Inconclusive => "inconclusive",
        VerdictOutcome::Fail => "fail",
        VerdictOutcome::Pass => "pass",
    }
}

// ---------------------------------------------------------------------------------------------
// freeze-lock
// ---------------------------------------------------------------------------------------------

fn freeze_lock(
    data: &[String],
    price_scale: &[String],
    policy: &str,
    mode: &str,
    candle_interval: &str,
    out: &str,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let symbols: Vec<String> = datasets.iter().map(|(s, _)| s.clone()).collect();
    // The freeze lock seals the strategy on `candle_interval` (default 4h); `backtest` accepts the
    // same `--candle-interval`, so a sealed lock only matches a run on the same interval.
    let cfg = build_config(&symbols, &scales, candle_interval)?;
    let cv = config_version(&cfg.canonical_params_json());

    let mut dataset_hashes: BTreeMap<String, String> = BTreeMap::new();
    for (symbol, path) in &datasets {
        dataset_hashes.insert(symbol.clone(), dataset_hash(path)?);
    }
    // Validate the policy/mode strings even at freeze time (a typo must not seal a bad bar).
    parse_policies(policy)?;
    parse_modes(mode)?;

    let lock = build_lock(policy, mode, default_sim_params(), cv, dataset_hashes)?;
    lock::write_lock(out, &lock.canonical_json())?;
    println!("# freeze-lock written to {out}");
    println!("# oos_boundary_ms={}", lock.oos_boundary_ms);
    println!("{}", lock.hash());
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// count-signals (ITEM 6)
// ---------------------------------------------------------------------------------------------

fn count_signals_cmd(
    data: &[String],
    price_scale: &[String],
    policy: &str,
    candle_interval: &str,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let policies = parse_policies(policy)?;

    println!("# strategy-analyzer count-signals (counts only; no PnL, no trades, no verdict)");
    println!("# oos_boundary_ms={}", lock::OOS_BOUNDARY_MS);

    for (symbol, path) in &datasets {
        let cfg = build_config(std::slice::from_ref(symbol), &scales, candle_interval)?;
        let candles = load_csv(path)?;
        // Refuse any row at/after the in-sample/OOS boundary: counting is in-sample only.
        if let Some(ms) = lock::first_oos_candle(&candles, lock::OOS_BOUNDARY_MS) {
            return Err(format!(
                "{symbol}: row at/after the in-sample/OOS boundary (open_time_ms={ms} >= {}); count-signals is in-sample only",
                lock::OOS_BOUNDARY_MS
            )
            .into());
        }
        for p in &policies {
            let c = count_signals(symbol, &candles, *p, &cfg)?;
            let reasons: Vec<String> = c
                .reason_counts
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            println!(
                "symbol={symbol} policy={} signals={} reason_counts [{}]",
                policy_name(*p),
                c.signals,
                reasons.join(" ")
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// dump-signals
// ---------------------------------------------------------------------------------------------

/// Dump every emitted `Signal` as one JSON object per line to stdout (in-sample only; no PnL, no
/// trades, no verdict). Replays each symbol × policy through the pure core exactly like
/// `count-signals` and refuses any row at/after the in-sample/OOS boundary.
fn dump_signals_cmd(
    data: &[String],
    price_scale: &[String],
    policy: &str,
    mode: &str,
    candle_interval: &str,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let policies = parse_policies(policy)?;
    let mode = match mode {
        "fixed_r" => TargetMode::FixedR,
        "structural" => TargetMode::Structural,
        other => return Err(format!("unknown mode: {other}").into()),
    };

    for (symbol, path) in &datasets {
        let cfg = Config {
            target_mode: mode,
            ..build_config(std::slice::from_ref(symbol), &scales, candle_interval)?
        };
        let candles = load_csv(path)?;
        // Refuse any row at/after the in-sample/OOS boundary: dumping is in-sample only.
        if let Some(ms) = lock::first_oos_candle(&candles, lock::OOS_BOUNDARY_MS) {
            return Err(format!(
                "{symbol}: row at/after the in-sample/OOS boundary (open_time_ms={ms} >= {}); dump-signals is in-sample only",
                lock::OOS_BOUNDARY_MS
            )
            .into());
        }
        for p in &policies {
            for sig in emitted_signals(symbol, &candles, *p, &cfg)? {
                println!("{}", serde_json::to_string(&sig)?);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// benchmark (fast interval sweep)
// ---------------------------------------------------------------------------------------------

/// Per-interval benchmark result: the signal count and the net-of-cost trade statistics.
///
/// `signals` is summed across the selected path policies; every R statistic pools the resolved trades
/// of all selected policies (a single "aggregate over policies" view, rather than one line per
/// policy). All values are net of fees, entry/stop slippage and borrow.
struct IntervalStats {
    signals: usize,
    signals_per_day: f64,
    trades: usize,
    win_rate: f64,
    mean_net_r: f64,
    median_net_r: f64,
    total_net_r: Decimal,
    /// Realized fee/slippage/borrow drag per trade in R: mean gross R (a zero-cost replay on the same
    /// signals) minus mean net R.
    mean_cost_r: f64,
}

/// Median net R across `trades` (the mean of the two middle values for an even count).
fn median_r(trades: &[TradeRecord]) -> Decimal {
    if trades.is_empty() {
        return Decimal::ZERO;
    }
    let mut rs: Vec<Decimal> = trades.iter().map(|t| t.r).collect();
    rs.sort();
    let n = rs.len();
    if n % 2 == 1 {
        rs[n / 2]
    } else {
        (rs[n / 2 - 1] + rs[n / 2]) / Decimal::from(2u64)
    }
}

/// Parse the comma-separated `--intervals` list, rejecting any interval the shared `interval_to_ms`
/// does not recognise (never a silent skip) and any empty entry.
fn parse_intervals(spec: &str) -> Res<Vec<String>> {
    let mut out = Vec::new();
    for raw in spec.split(',') {
        let iv = raw.trim();
        if iv.is_empty() {
            return Err(format!("--intervals has an empty entry: {spec:?}").into());
        }
        interval_to_ms(iv).map_err(|e| format!("--intervals {iv:?}: {e}"))?;
        out.push(iv.to_string());
    }
    if out.is_empty() {
        return Err("--intervals requires at least one interval".into());
    }
    Ok(out)
}

/// Parse repeatable `INTERVAL=PATH` `--data` mappings for `benchmark`; the interval key is validated
/// with the shared parser.
fn parse_interval_data(items: &[String]) -> Res<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for item in items {
        let (interval, path) = item
            .split_once('=')
            .ok_or_else(|| format!("--data must be INTERVAL=PATH, got {item:?}"))?;
        if interval.is_empty() || path.is_empty() {
            return Err(format!("--data must be INTERVAL=PATH, got {item:?}").into());
        }
        interval_to_ms(interval).map_err(|e| format!("--data {interval:?}: {e}"))?;
        out.insert(interval.to_string(), path.to_string());
    }
    Ok(out)
}

/// Parse one sim-rate CLI argument (a decimal) with a named error.
fn parse_decimal_arg(name: &str, value: &str) -> Res<Decimal> {
    value
        .parse::<Decimal>()
        .map_err(|e| format!("--{name} must be a decimal, got {value:?}: {e}").into())
}

/// Resolve one interval's candles, applying the OOS-boundary guard on every load. `Ok(None)` means
/// the interval has no data — no cached file with `--no-fetch`, or an empty file — so the caller can
/// report a `no data` row instead of aborting the sweep. An explicit `--data INTERVAL=PATH` naming a
/// missing file is an error (a misconfiguration, not a `no data` interval).
#[allow(clippy::too_many_arguments)]
fn resolve_candles(
    symbol: &str,
    interval: &str,
    interval_ms: i64,
    data_map: &BTreeMap<String, String>,
    cache_dir: &str,
    no_fetch: bool,
    start_s: Option<i64>,
    end_s: Option<i64>,
) -> Res<Option<Vec<Candle>>> {
    let path = match data_map.get(interval) {
        Some(p) => {
            if !Path::new(p).exists() {
                return Err(format!("--data {interval}={p}: file not found").into());
            }
            p.clone()
        }
        None => {
            let cached = format!("{cache_dir}/{symbol}_{interval}.csv");
            if Path::new(&cached).exists() {
                cached
            } else if no_fetch {
                return Ok(None);
            } else {
                let (s, e) = match (start_s, end_s) {
                    (Some(s), Some(e)) => (s, e),
                    _ => {
                        return Err(format!(
                            "interval {interval}: no cached file at {cached} and --start-s/--end-s are required to fetch"
                        )
                        .into())
                    }
                };
                fetch_data(symbol, interval, s, e, &cached, None)?;
                cached
            }
        }
    };

    // Forward-fill on the interval's own spacing (KuCoin omits empty intervals; the core rejects
    // gaps), so a 5min file is filled at 5min, not the `load_csv` 4h default.
    let candles = load_csv_with_policy(&path, GapPolicy::ForwardFill, interval_ms)?;
    // Refuse any row at/after the in-sample/OOS boundary, exactly like the other in-sample commands.
    if let Some(ms) = lock::first_oos_candle(&candles, lock::OOS_BOUNDARY_MS) {
        return Err(format!(
            "{symbol} {interval}: row at/after the in-sample/OOS boundary (open_time_ms={ms} >= {}); benchmark is in-sample only",
            lock::OOS_BOUNDARY_MS
        )
        .into());
    }
    if candles.is_empty() {
        return Ok(None);
    }
    Ok(Some(candles))
}

/// Replay + simulate one interval (net and zero-cost, for every selected policy) and collect the
/// counts and net R statistics. Only two passes per policy run here — no perturbation or verdict
/// diagnostics — which is what keeps a long sweep fast.
fn interval_stats(
    symbol: &str,
    candles: &[Candle],
    cfg: &Config,
    policies: &[PathPolicy],
    params: SimParams,
) -> Result<IntervalStats, StrategyError> {
    let mut signals = 0usize;
    let mut net_trades: Vec<TradeRecord> = Vec::new();
    let mut gross_trades: Vec<TradeRecord> = Vec::new();
    for p in policies {
        let net = run(symbol, candles, *p, cfg, params)?;
        signals += net.signals;
        net_trades.extend(net.trades);
        // A zero-cost replay on the same signals yields the gross R, so the fee/slippage/borrow drag
        // per trade is `mean(gross) - mean(net)` (mirrors `backtest`'s cushion).
        let zero = run(symbol, candles, *p, cfg, zero_cost_params(params))?;
        gross_trades.extend(zero.trades);
    }
    let net = summarize(&net_trades);
    let gross = summarize(&gross_trades);
    let total_net_r: Decimal = net_trades.iter().map(|t| t.r).sum();
    // Wall-clock span of the interval's data, for a per-day signal rate.
    let span_days = match (candles.first(), candles.last()) {
        (Some(first), Some(last)) => {
            ((last.open_time_ms - first.open_time_ms) as f64 + cfg.candle_interval_ms as f64)
                / 86_400_000.0
        }
        _ => 0.0,
    };
    Ok(IntervalStats {
        signals,
        signals_per_day: if span_days > 0.0 {
            signals as f64 / span_days
        } else {
            0.0
        },
        trades: net.n,
        win_rate: net.win_rate,
        mean_net_r: net.mean_r,
        median_net_r: median_r(&net_trades).to_string().parse::<f64>().unwrap(),
        total_net_r,
        mean_cost_r: gross.mean_r - net.mean_r,
    })
}

// The CLI's run knobs are passed individually for readability at the `main` match arm; the count is
// well past the clippy threshold for this benchmark sweep.
#[allow(clippy::too_many_arguments)]
fn benchmark(
    symbol: &str,
    price_scale: &[String],
    intervals: &str,
    data: &[String],
    start_s: Option<i64>,
    end_s: Option<i64>,
    cache_dir: &str,
    no_fetch: bool,
    fee_bps: &str,
    entry_slip_bps: &str,
    stop_slip_bps: &str,
    borrow_bps_per_day: &str,
    policy: &str,
) -> Res<()> {
    let scales = parse_scales(price_scale)?;
    let policies = parse_policies(policy)?;
    let interval_list = parse_intervals(intervals)?;
    let data_map = parse_interval_data(data)?;
    let params = SimParams {
        fee_bps: parse_decimal_arg("fee-bps", fee_bps)?,
        entry_slip_bps: parse_decimal_arg("entry-slip-bps", entry_slip_bps)?,
        stop_slip_bps: parse_decimal_arg("stop-slip-bps", stop_slip_bps)?,
        borrow_bps_per_day: parse_decimal_arg("borrow-bps-per-day", borrow_bps_per_day)?,
    };
    // The symbol must name an explicit price scale, exactly like `backtest` (no silent default).
    let dp = scales.get(symbol).copied().ok_or_else(|| {
        format!(
            "no --price-scale for {symbol}; the symbol needs an explicit price scale (no silent default)"
        )
    })?;

    println!("# strategy-analyzer benchmark (in-sample / exploratory)");
    println!(
        "# symbol={symbol} price_scale={dp} policy={policy} intervals={}",
        interval_list.join(",")
    );
    println!(
        "# sim fee_bps={} entry_slip_bps={} stop_slip_bps={} borrow_bps_per_day={}",
        params.fee_bps, params.entry_slip_bps, params.stop_slip_bps, params.borrow_bps_per_day
    );
    println!("# oos_boundary_ms={}", lock::OOS_BOUNDARY_MS);
    println!("# note: in-sample and exploratory only; this is not a confirmatory verdict");

    let mut rows: Vec<(String, Option<IntervalStats>)> = Vec::new();
    for interval in &interval_list {
        let ms = interval_to_ms(interval).map_err(|e| format!("--intervals {interval:?}: {e}"))?;
        let candles = match resolve_candles(
            symbol, interval, ms, &data_map, cache_dir, no_fetch, start_s, end_s,
        )? {
            Some(c) => c,
            None => {
                rows.push((interval.clone(), None));
                continue;
            }
        };
        let symbols = vec![symbol.to_string()];
        let cfg = build_config(&symbols, &scales, interval)?;
        let stats = interval_stats(symbol, &candles, &cfg, &policies, params)?;
        rows.push((interval.clone(), Some(stats)));
    }

    // Highest total net R first; an interval with no data sorts to the bottom.
    rows.sort_by(|a, b| match (&a.1, &b.1) {
        (Some(x), Some(y)) => y.total_net_r.cmp(&x.total_net_r),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    println!();
    println!("interval\tsignals\tsignals_per_day\ttrades\twin_rate\tmean_net_r\tmedian_net_r\ttotal_net_r\tmean_cost_r\tcovers_fees");
    for (interval, stats) in &rows {
        match stats {
            None => println!("{interval}\tno data"),
            Some(s) => {
                // "Survives fees?": the pooled net R is positive both in total and on average.
                let covers = s.total_net_r > Decimal::ZERO && s.mean_net_r > 0.0;
                println!(
                    "{interval}\t{}\t{:.2}\t{}\t{:.4}\t{:.4}\t{:.4}\t{}\t{:.4}\t{}",
                    s.signals,
                    s.signals_per_day,
                    s.trades,
                    s.win_rate,
                    s.mean_net_r,
                    s.median_net_r,
                    s.total_net_r,
                    s.mean_cost_r,
                    if covers { "yes" } else { "no" }
                );
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// backtest
// ---------------------------------------------------------------------------------------------

// The CLI's run knobs are passed individually for readability at the `main` match arm; the count is
// at the clippy threshold (8/7) now that `--candle-interval` was added.
#[allow(clippy::too_many_arguments)]
fn backtest(
    data: &[String],
    price_scale: &[String],
    policy: &str,
    mode: &str,
    lock_path: Option<String>,
    oos: bool,
    oos_consumed: bool,
    candle_interval: &str,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let symbols: Vec<String> = datasets.iter().map(|(s, _)| s.clone()).collect();
    let cfg0 = build_config(&symbols, &scales, candle_interval)?;
    let cv = config_version(&cfg0.canonical_params_json());
    let policies = parse_policies(policy)?;
    let modes = parse_modes(mode)?;

    let loaded = load_datasets(data)?;
    let dataset_hashes: BTreeMap<String, String> = loaded
        .iter()
        .map(|l| (l.symbol.clone(), l.hash.clone()))
        .collect();

    // A run "touches the OOS window" when any loaded row is at/after the pre-registered boundary.
    let touches_oos = loaded.iter().any(|l| {
        l.candles
            .iter()
            .any(|c| c.open_time_ms >= lock::OOS_BOUNDARY_MS)
    });

    let params = default_sim_params();
    let expected_lock = build_lock(policy, mode, params, cv.clone(), dataset_hashes.clone())?;

    // Guarded path: any OOS-touching run requires --oos AND a matching lock. Otherwise refuse and
    // never print a verdict.
    let mut non_confirmatory = false;
    let oos_log: Option<String> = if oos {
        let path = lock_path.as_deref().ok_or("--oos requires --lock <path>")?;
        // ITEM 5: OOS must name exactly one mode that matches the frozen lock. `both` would let the
        // target mode be chosen on OOS (a hidden degree of freedom); any other mismatch is drift.
        if mode == "both" {
            return Err(
                "--oos requires exactly one explicit --mode (fixed_r or structural); --mode both would select the mode on OOS".into(),
            );
        }
        let locked = lock::read_lock(path)?;
        if locked.params.mode != mode {
            return Err(format!(
                "--oos mode {mode:?} does not match the frozen lock mode {:?}",
                locked.params.mode
            )
            .into());
        }
        lock::check_lock(path, &expected_lock.hash())?;
        let log = format!("{path}.oos.log");
        match lock::oos_gate(&log, oos_consumed)? {
            lock::OosDecision::Allow => {}
            lock::OosDecision::Refuse => {
                return Err(format!(
                    "OOS already consumed in {log}; pass --oos-consumed to override (stamps non-confirmatory)"
                )
                .into());
            }
            lock::OosDecision::RerunNonConfirmatory => {
                non_confirmatory = true;
                if let Some(prior) = lock::last_oos_record(&log) {
                    println!("# OOS override: prior run verdict = {}", prior.verdict);
                }
            }
        }
        Some(log)
    } else {
        if touches_oos {
            return Err(format!(
                "dataset touches the OOS window (a row is at/after {} ms); a guarded run requires --oos with a matching --lock",
                lock::OOS_BOUNDARY_MS
            )
            .into());
        }
        None
    };

    // §8.5 header: symbol(s), resolution, dataset hashes, config_version, path policy, target mode.
    println!("# strategy-analyzer backtest");
    println!(
        "# symbols={} resolution=4h-synthesized candle_interval={candle_interval}",
        symbols.join(",")
    );
    for l in &loaded {
        println!(
            "# dataset symbol={} path={} hash={}",
            l.symbol, l.path, l.hash
        );
    }
    println!("# config_version={cv}");
    println!("# path_policy={policy} target_mode={mode}");
    println!("# oos_boundary_ms={}", lock::OOS_BOUNDARY_MS);
    println!("# caveat: 4H synthesis (4 ticks/candle) => latency-insensitive; sim approximates engine net-R:R");
    if non_confirmatory {
        println!("# NON-CONFIRMATORY (one-shot OOS overridden)");
    } else if oos {
        println!("# CONFIRMATORY OOS run");
    } else {
        println!("# EXPLORATORY (in-sample; not a confirmatory result)");
    }

    for m in &modes {
        let cfg = Config {
            target_mode: *m,
            ..cfg0.clone()
        };
        let mut runs: Vec<(String, PathPolicy, RunResult)> = Vec::new();
        for l in &loaded {
            for (p, r) in run_policies(&l.symbol, &l.candles, &policies, &cfg, params)? {
                runs.push((l.symbol.clone(), p, r));
            }
        }

        println!("\n## mode={}", mode_name(*m));
        for l in &loaded {
            for (_, p, r) in runs.iter().filter(|(s, _, _)| s == &l.symbol) {
                let s = summarize(&r.trades);
                println!("  symbol={} path={:<10} signals={} trades={} skipped_open_position={} unresolved={} unfillable={} missed_touch={} mean_r={:.4} win_rate={:.4} long_mean_r={:.4} short_mean_r={:.4}",
                    l.symbol, policy_name(*p), r.signals, s.n, r.skipped_open_position, r.unresolved, r.unfillable, r.missed_touch,
                    s.mean_r, s.win_rate, s.by_direction_long, s.by_direction_short);
                let reasons: Vec<String> = r
                    .reason_counts
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                println!("    reason_counts [{}]", reasons.join(" "));
            }
        }

        // Adversarial composite across all symbols: longs from low_first, shorts from high_first.
        let low = policy_trades(&runs, PathPolicy::LowFirst);
        let high = policy_trades(&runs, PathPolicy::HighFirst);
        let composite = adversarial_composite(&low, &high);

        if !composite.is_empty() || !runs.is_empty() {
            let cs = summarize(&composite);
            let (mean, lower) = time_blocked_bootstrap(
                &composite,
                BOOTSTRAP_BLOCK_MS,
                BOOTSTRAP_RESAMPLES,
                BOOTSTRAP_SEED,
            );
            println!("  adversarial_composite n={} mean_r={:.4} bootstrap_mean={:.4} bootstrap_5pct_lower={:.4}",
                cs.n, cs.mean_r, mean, lower);

            // Cushion calibration: realized cost per trade in R (zero-cost vs with-cost composite).
            let zero = zero_cost_params(params);
            let mut zero_runs: Vec<(String, PathPolicy, RunResult)> = Vec::new();
            for l in &loaded {
                for (p, r) in run_policies(&l.symbol, &l.candles, &policies, &cfg, zero)? {
                    zero_runs.push((l.symbol.clone(), p, r));
                }
            }
            let comp0 = summarize(&adversarial_composite(
                &policy_trades(&zero_runs, PathPolicy::LowFirst),
                &policy_trades(&zero_runs, PathPolicy::HighFirst),
            ));
            let mean_cost_r = comp0.mean_r - cs.mean_r;
            match cushion_warning(mean_cost_r) {
                Some(w) => println!("  cushion mean_cost_r={mean_cost_r:.4} WARNING: {w}"),
                None => println!("  cushion mean_cost_r={mean_cost_r:.4} ok (<= 0.5R)"),
            }

            // Verdict (pre-registered §10.4 bar) on the adversarial composite, with stats derived
            // from the trades. The 1.5× cost stress reruns every symbol/policy at scaled costs.
            let stress = scaled_cost_params(params, Decimal::from(3), Decimal::from(2));
            let mut stress_runs: Vec<(String, PathPolicy, RunResult)> = Vec::new();
            for l in &loaded {
                for (p, r) in run_policies(&l.symbol, &l.candles, &policies, &cfg, stress)? {
                    stress_runs.push((l.symbol.clone(), p, r));
                }
            }
            let stress_composite = adversarial_composite(
                &policy_trades(&stress_runs, PathPolicy::LowFirst),
                &policy_trades(&stress_runs, PathPolicy::HighFirst),
            );
            // Regime segmentation needs per-symbol 200-day closes over the OOS window, which a
            // single cached 4H file cannot supply here; the regime therefore fails CLOSED.
            let v = verdict(&VerdictInputs {
                composite: &composite,
                low_first: &low,
                high_first: &high,
                composite_at_1_5x: &stress_composite,
                regime: Regime::Unknown,
            });
            println!("  verdict n={} outcome={} mean_r={:.4} lower_95={:.4} total_r={} max_pair_r={} regime={:?} concentrated={} cost_stress_ok={} both_policies_positive={}",
                v.n, outcome_name(v.outcome), v.mean_r, v.lower_95, v.total_r, v.max_pair_r, v.regime,
                v.concentrated, v.cost_stress_ok, v.both_policies_positive);
            if let Some(log) = &oos_log {
                lock::append_oos_log(
                    log,
                    &OosLogRecord {
                        dataset_hashes: dataset_hashes.clone(),
                        verdict: format!("{}={}", mode_name(*m), outcome_name(v.outcome)),
                    },
                )?;
            }

            // Perturbation diagnostics: 4H synthesis only, seeded and reproducible, never a
            // selector. Every perturbed cell is validated; an invalid one is skipped and counted,
            // never panicked on (the pass stops at its pre-registered wall-clock budget).
            let axis = diagnostic_axis(&cfg, *m);
            let eval = |vals: &[Decimal]| -> Option<Decimal> {
                // `apply_diagnostic_axis` validates the rebuilt config; a cell that violates a
                // config rule (e.g. a −20% target_r below the engine floor) returns `None`.
                let c = apply_diagnostic_axis(&cfg, *m, vals).ok()?;
                let mut all: Vec<TradeRecord> = Vec::new();
                for l in &loaded {
                    // Diagnostics only, 4H synthesis: a validated config over the same validated
                    // dataset must not error. If it ever did, skip the cell rather than panic.
                    let lf = run(&l.symbol, &l.candles, PathPolicy::LowFirst, &c, params).ok()?;
                    let hf = run(&l.symbol, &l.candles, PathPolicy::HighFirst, &c, params).ok()?;
                    all.extend(adversarial_composite(&lf.trades, &hf.trades));
                }
                Some(mean_r(&all))
            };
            let rep = perturb::run_diagnostics(&axis, &eval);
            println!("  diagnostics variants_tried={} oat={}/{} invalid={} ({:.3}) joint={}/{} invalid={} ({:.3}) elapsed={:?} budget={}s within_budget={} oat_gate_met={} joint_gate_met={}",
                rep.variants_tried(), rep.oat_positive_cells, rep.oat_cells, rep.oat_invalid_cells, rep.oat_positive_fraction,
                rep.joint_positive_draws, rep.joint_draws, rep.joint_invalid_draws, rep.joint_positive_fraction,
                rep.elapsed, perturb::DIAGNOSTICS_BUDGET_SECS, rep.within_budget,
                rep.oat_gate_met(), rep.joint_gate_met());
        }
    }
    Ok(())
}

/// Integer-parameter rounding rule for the diagnostics axis (documented, pre-registered): a ±20%
/// perturbation of an integer parameter (ATR period, zone-search lookback) is rounded **half-up**
/// (`MidpointAwayFromZero`) to the nearest whole number, then floored at 1 so a perturbed period
/// can never be zero. E.g. `atr_period` 14 → 11.2 → 11 and → 16.8 → 17.
fn round_axis_integer(v: Decimal) -> u32 {
    round_to(v, 0).max(Decimal::ONE).to_u32().unwrap_or(1)
}

/// The diagnostics parameter axis, in a fixed documented order:
/// `0` `bos_min_distance_atr`, `1` `impulse_body_threshold_atr`, `2` `stop_buffer_atr`,
/// `3` `min_zone_height_atr`, `4` `max_zone_height_atr`, `5` `target_r`,
/// `6` `atr_period` (integer), `7` `zone_search_lookback` (integer),
/// `8` `min_gross_rr` (**`structural` mode only**).
///
/// `min_gross_rr` gates `structural` only, so it is added to the axis only there; perturbing an
/// unused parameter in `fixed_r` would just dilute the diagnostic.
fn diagnostic_axis(cfg: &Config, mode: TargetMode) -> Vec<Decimal> {
    let mut axis = vec![
        cfg.bos_min_distance_atr,
        cfg.impulse_body_threshold_atr,
        cfg.stop_buffer_atr,
        cfg.min_zone_height_atr,
        cfg.max_zone_height_atr,
        cfg.target_r,
        Decimal::from(cfg.atr_period),
        Decimal::from(cfg.zone_search_lookback),
    ];
    if mode == TargetMode::Structural {
        axis.push(cfg.min_gross_rr);
    }
    axis
}

/// Rebuild a config from a diagnostics-axis vector (see `diagnostic_axis` for the slot order) and
/// **validate** it. Integer slots are rounded by `round_axis_integer`. An invalid cell is returned
/// as `Err` so the caller can skip and count it instead of panicking.
fn apply_diagnostic_axis(
    cfg: &Config,
    mode: TargetMode,
    v: &[Decimal],
) -> Result<Config, StrategyError> {
    let mut c = Config {
        bos_min_distance_atr: v[0],
        impulse_body_threshold_atr: v[1],
        stop_buffer_atr: v[2],
        min_zone_height_atr: v[3],
        max_zone_height_atr: v[4],
        target_r: v[5],
        atr_period: round_axis_integer(v[6]),
        zone_search_lookback: round_axis_integer(v[7]),
        ..cfg.clone()
    };
    if mode == TargetMode::Structural {
        if let Some(m) = v.get(8) {
            c.min_gross_rr = *m;
        }
    }
    c.validate()?;
    Ok(c)
}

// ---------------------------------------------------------------------------------------------
// calibrate (unchanged)
// ---------------------------------------------------------------------------------------------

/// True when the low of `chunk` is visited before its high (intrabar order observed in 1m data).
/// `None` for a degenerate chunk where the low and high share a minute.
///
/// Ties use the **first occurrence of both extremes** (`min_by_key` already picks the first minimum,
/// but `max_by_key` picks the *last* maximum, which would make the tie rule asymmetric).
fn observed_low_first(chunk: &[Candle]) -> Option<bool> {
    let mut lo: Option<usize> = None;
    let mut hi: Option<usize> = None;
    for (i, c) in chunk.iter().enumerate() {
        if lo.is_none_or(|j| c.low < chunk[j].low) {
            lo = Some(i);
        }
        if hi.is_none_or(|j| c.high > chunk[j].high) {
            hi = Some(i);
        }
    }
    match (lo, hi) {
        (Some(l), Some(h)) if l != h => Some(l < h),
        _ => None,
    }
}

fn calibrate(one_minute: &str, fetched_4h: &str) -> Res<()> {
    let minutes = load_csv_with_policy(one_minute, GapPolicy::ForwardFill, 60_000)?;
    let fetched = load_csv(fetched_4h)?;
    // Bucket the 1m candles by their 4H grid cell (open_time_ms / 4h), not by a 240-count from the
    // start, so a series that begins mid-bucket still aggregates each grid cell correctly.
    let buckets = group_4h_buckets(&minutes);
    let aggregated: Vec<Candle> = buckets.iter().map(|b| aggregate_4h_bucket(b)).collect();

    let mismatches = validate_aggregates(&aggregated, &fetched)?;
    println!("# 1m calibration");
    println!("#   aggregated_candles={} fetched_candles={} within_tolerance_mismatches={} tolerance={AGG_TOLERANCE:e}",
        aggregated.len(), fetched.len(), mismatches.len());
    for m in &mismatches {
        println!(
            "#   candle={} field={} aggregated={} fetched={} rel_diff={:.3e}",
            m.open_time_ms, m.field, m.aggregated, m.fetched, m.rel_diff
        );
    }

    if aggregated.len() == fetched.len() {
        let observed: Vec<bool> = buckets
            .iter()
            .filter_map(|b| observed_low_first(b))
            .collect();
        for (name, assumed) in [("low_first", true), ("high_first", false)] {
            let assumed_flags = vec![assumed; observed.len()];
            println!(
                "#   intrabar_{name}_wrong_fraction={:.4}",
                calibration_wrong_fraction(&assumed_flags, &observed)
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// fetch-data (interval parsing shared with the other subcommands; OOS-window guard)
// ---------------------------------------------------------------------------------------------

/// Seconds per candle for `fetch-data --interval`, derived from the shared `interval_to_ms` mapping
/// (the CLI's single source of truth). An unknown interval is rejected with the same error the other
/// subcommands use (the shared parser's `unknown candle interval`), never silently defaulted, so
/// every KuCoin interval — including `3min`, missing from the previous bespoke table — works.
fn interval_seconds(interval: &str) -> Res<i64> {
    Ok(interval_to_ms(interval).map_err(|e| format!("--interval {interval:?}: {e}"))? / 1000)
}

fn fetch_data(
    symbol: &str,
    interval: &str,
    start_s: i64,
    end_s: i64,
    out: &str,
    lock_path: Option<&str>,
) -> Res<()> {
    // Until the freeze lock exists, refuse any range overlapping the OOS window (§10.4).
    let lock_exists = lock_path.is_some_and(|p| Path::new(p).exists());
    fetch::guard_oos_range(start_s, end_s, lock::OOS_BOUNDARY_MS, lock_exists)?;

    let interval_s = interval_seconds(interval)?;
    let mut raw: Vec<[String; 7]> = Vec::new();
    for (s, e) in fetch::page_windows(start_s, end_s, interval_s) {
        raw.extend(fetch_with_retry(symbol, interval, s, e)?);
        std::thread::sleep(Duration::from_millis(250)); // throttle: respect rate limits
    }
    // KuCoin columns: time,open,close,high,low,volume,turnover -> ms + OHLC cache order.
    let mut rows = rows_to_candles(&raw)?;
    rows.sort_by_key(|r| r.0); // oldest first
    rows.dedup_by_key(|r| r.0); // de-duplicate across page boundaries

    let mut wtr = csv::Writer::from_path(out)?;
    wtr.write_record(["open_time_ms", "open", "high", "low", "close"])?;
    for (ms, open, high, low, close) in &rows {
        wtr.write_record([
            ms.to_string(),
            open.clone(),
            high.clone(),
            low.clone(),
            close.clone(),
        ])?;
    }
    wtr.flush()?;

    let hash = dataset_hash(out)?;
    let provenance_path =
        write_provenance(out, symbol, interval, start_s, end_s, rows.len(), &hash)?;
    println!("# provenance written to {provenance_path}");
    println!("{hash}");
    Ok(())
}

/// Convert raw KuCoin rows (`time,open,close,high,low,volume,turnover`; time in seconds) to
/// `(open_time_ms, open, high, low, close)`. A malformed timestamp is an **error** — never a silent
/// `0`, which would inject a bogus 1970 candle and corrupt the dataset hash.
fn rows_to_candles(raw: &[[String; 7]]) -> Res<Vec<CacheRow>> {
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        let sec: i64 = r[0]
            .parse()
            .map_err(|e| format!("fetch-data: bad timestamp {:?}: {e}", r[0]))?;
        let ms = sec
            .checked_mul(1000)
            .ok_or_else(|| format!("fetch-data: timestamp out of range {:?}", r[0]))?;
        out.push((ms, r[1].clone(), r[3].clone(), r[4].clone(), r[2].clone()));
    }
    Ok(out)
}

/// Provenance sidecar written next to the cached CSV (symbol, interval, requested range, row count,
/// dataset hash) so a cache file's origin can be audited without re-fetching.
#[derive(serde::Serialize)]
struct Provenance<'a> {
    symbol: &'a str,
    interval: &'a str,
    start_s: i64,
    end_s: i64,
    row_count: usize,
    dataset_hash: &'a str,
}

fn write_provenance(
    out: &str,
    symbol: &str,
    interval: &str,
    start_s: i64,
    end_s: i64,
    row_count: usize,
    dataset_hash: &str,
) -> Res<String> {
    let path = format!("{out}.provenance.json");
    let p = Provenance {
        symbol,
        interval,
        start_s,
        end_s,
        row_count,
        dataset_hash,
    };
    std::fs::write(&path, serde_json::to_string_pretty(&p)?)?;
    Ok(path)
}

fn fetch_with_retry(symbol: &str, interval: &str, s: i64, e: i64) -> Res<Vec<[String; 7]>> {
    let mut delay_ms: u64 = 500;
    for attempt in 0..6u32 {
        match fetch::fetch_page(KUCOIN_BASE, symbol, interval, s, e) {
            Ok(rows) => return Ok(rows),
            Err(err) if fetch::is_rate_limited(&err) && attempt < 5 => {
                std::thread::sleep(Duration::from_millis(delay_ms)); // exponential backoff on 429
                delay_ms = delay_ms.saturating_mul(2);
            }
            Err(err) => return Err(err.into()),
        }
    }
    Err("fetch-data: exhausted retries on rate limit".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn integer_axis_rounds_half_up_with_a_floor_of_one() {
        assert_eq!(round_axis_integer(dec("11.2")), 11); // 14 × 0.8
        assert_eq!(round_axis_integer(dec("16.8")), 17); // 14 × 1.2
        assert_eq!(round_axis_integer(dec("2.5")), 3, "half-up, not half-even");
        assert_eq!(round_axis_integer(dec("0.4")), 1, "floored at 1");
        assert_eq!(round_axis_integer(dec("0")), 1);
    }

    #[test]
    fn diagnostic_axis_adds_min_gross_rr_only_in_structural() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(diagnostic_axis(&cfg, TargetMode::FixedR).len(), 8);
        let structural = diagnostic_axis(&cfg, TargetMode::Structural);
        assert_eq!(structural.len(), 9);
        assert_eq!(structural[8], cfg.min_gross_rr);
    }

    #[test]
    fn apply_diagnostic_axis_validates_and_reports_invalid_cells() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // The base axis is valid in both modes.
        for mode in [TargetMode::FixedR, TargetMode::Structural] {
            let base = diagnostic_axis(&cfg, mode);
            assert!(apply_diagnostic_axis(&cfg, mode, &base).is_ok());
        }
        // A −20% target_r (2.4) falls below the engine net floor (2.5): invalid, not a panic.
        let mut v = diagnostic_axis(&cfg, TargetMode::FixedR);
        v[5] = dec("2.4");
        assert!(apply_diagnostic_axis(&cfg, TargetMode::FixedR, &v).is_err());
        // A −20% min_gross_rr (2.4) is invalid in structural mode.
        let mut s = diagnostic_axis(&cfg, TargetMode::Structural);
        s[8] = dec("2.4");
        assert!(apply_diagnostic_axis(&cfg, TargetMode::Structural, &s).is_err());
    }

    #[test]
    fn apply_diagnostic_axis_applies_the_integer_rounding_rule() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut v = diagnostic_axis(&cfg, TargetMode::FixedR);
        v[6] = dec("11.2");
        v[7] = dec("12.0");
        let applied = apply_diagnostic_axis(&cfg, TargetMode::FixedR, &v).unwrap();
        assert_eq!(applied.atr_period, 11);
        assert_eq!(applied.zone_search_lookback, 12);
    }

    #[test]
    fn observed_low_first_uses_the_first_occurrence_of_each_extreme() {
        let mk = |h: &str, l: &str| Candle {
            open_time_ms: 0,
            open: dec("5"),
            high: dec(h),
            low: dec(l),
            close: dec("5"),
        };
        // First low at index 1, first high at index 2 (both duplicated later).
        let low_first = vec![mk("6", "1"), mk("10", "2"), mk("6", "1"), mk("10", "2")];
        assert_eq!(observed_low_first(&low_first), Some(true));
        // Swap so the first high precedes the first low.
        let high_first = vec![mk("10", "2"), mk("6", "1"), mk("10", "2"), mk("6", "1")];
        assert_eq!(observed_low_first(&high_first), Some(false));
        // Degenerate: a single minute holds both extremes (nothing observed).
        assert_eq!(observed_low_first(&[mk("10", "2")]), None);
        assert_eq!(observed_low_first(&[]), None);
    }

    /// `fetch-data --interval` must use the shared parser, not a separate interval table that can
    /// drift (the old bespoke map was missing `3min`). Every registered KuCoin interval must parse
    /// and agree with `interval_to_ms`; an unknown interval is rejected, never silently defaulted.
    #[test]
    fn fetch_data_interval_parser_accepts_every_kucoin_interval_and_agrees_with_the_shared_parser()
    {
        for interval in [
            "1min", "3min", "5min", "15min", "30min", "1hour", "2hour", "4hour", "6hour", "8hour",
            "12hour", "1day", "1week",
        ] {
            assert_eq!(
                interval_seconds(interval).unwrap(),
                interval_to_ms(interval).unwrap() / 1000,
                "interval {interval} must agree with the shared parser"
            );
        }
        assert!(interval_seconds("3min").is_ok());
        assert!(interval_seconds("7min").is_err());
    }

    #[test]
    fn rows_to_candles_rejects_a_bad_timestamp() {
        let mut good = std::array::from_fn(|_| String::new());
        good[0] = "1700000000".to_string(); // seconds
        good[1] = "10.0".to_string(); // open
        good[2] = "12.0".to_string(); // close
        good[3] = "13.0".to_string(); // high
        good[4] = "9.0".to_string(); // low
        let rows = rows_to_candles(&[good]).unwrap();
        assert_eq!(rows[0].0, 1_700_000_000_000);
        // KuCoin column order is time,open,close,high,low -> cached open,high,low,close.
        assert_eq!(
            (
                rows[0].1.as_str(),
                rows[0].2.as_str(),
                rows[0].3.as_str(),
                rows[0].4.as_str()
            ),
            ("10.0", "13.0", "9.0", "12.0")
        );

        let mut bad = std::array::from_fn(|_| String::new());
        bad[0] = "not-a-number".to_string();
        assert!(rows_to_candles(&[bad]).is_err());
    }
}
