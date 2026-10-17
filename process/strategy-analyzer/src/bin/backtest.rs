use clap::{Parser, Subcommand};
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
use strategy_analyzer::harness::run::{count_signals, run, RunResult};
use strategy_analyzer::harness::sim::SimParams;
use strategy_analyzer::harness::tick_source::{aggregate_4h, PathPolicy};
use strategy_analyzer::harness::verdict::{
    verdict, Regime, VerdictInputs, VerdictOutcome, BOOTSTRAP_RESAMPLES, BOOTSTRAP_SEED,
};
use strategy_analyzer::strategy::config::{config_version, Config, TargetMode};
use strategy_analyzer::strategy::error::StrategyError;
use strategy_analyzer::strategy::types::Candle;

const KUCOIN_BASE: &str = "https://api.kucoin.com";

type Res<T> = Result<T, Box<dyn std::error::Error>>;

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
            out,
        } => freeze_lock(&data, &price_scale, &policy, &mode, &out),
        Cmd::Backtest {
            data,
            price_scale,
            policy,
            mode,
            lock,
            oos,
            oos_consumed,
        } => backtest(&data, &price_scale, &policy, &mode, lock, oos, oos_consumed),
        Cmd::CountSignals {
            data,
            price_scale,
            policy,
        } => count_signals_cmd(&data, &price_scale, &policy),
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

/// Build the config for `symbols`, taking each price scale from `scales` (no silent default).
fn build_config(symbols: &[String], scales: &BTreeMap<String, u32>) -> Res<Config> {
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
    out: &str,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let symbols: Vec<String> = datasets.iter().map(|(s, _)| s.clone()).collect();
    let cfg = build_config(&symbols, &scales)?;
    let cv = config_version(&cfg.canonical_json());

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

fn count_signals_cmd(data: &[String], price_scale: &[String], policy: &str) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let policies = parse_policies(policy)?;

    println!("# strategy-analyzer count-signals (counts only; no PnL, no trades, no verdict)");
    println!("# oos_boundary_ms={}", lock::OOS_BOUNDARY_MS);

    for (symbol, path) in &datasets {
        let cfg = build_config(std::slice::from_ref(symbol), &scales)?;
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
// backtest
// ---------------------------------------------------------------------------------------------

fn backtest(
    data: &[String],
    price_scale: &[String],
    policy: &str,
    mode: &str,
    lock_path: Option<String>,
    oos: bool,
    oos_consumed: bool,
) -> Res<()> {
    let datasets = parse_data(data)?;
    let scales = parse_scales(price_scale)?;
    let symbols: Vec<String> = datasets.iter().map(|(s, _)| s.clone()).collect();
    let cfg0 = build_config(&symbols, &scales)?;
    let cv = config_version(&cfg0.canonical_json());
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
    println!("# symbols={} resolution=4h-synthesized", symbols.join(","));
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
            // selector. (Cell validation/clamping is deferred to a later batch.)
            let axis = diagnostic_axis(&cfg);
            let eval = |vals: &[Decimal]| -> Decimal {
                let c = apply_diagnostic_axis(&cfg, vals);
                let mut all: Vec<TradeRecord> = Vec::new();
                for l in &loaded {
                    // Diagnostics only: the same validated dataset cannot make the core error here.
                    let lf = run(&l.symbol, &l.candles, PathPolicy::LowFirst, &c, params)
                        .expect("run over the same validated dataset");
                    let hf = run(&l.symbol, &l.candles, PathPolicy::HighFirst, &c, params)
                        .expect("run over the same validated dataset");
                    all.extend(adversarial_composite(&lf.trades, &hf.trades));
                }
                mean_r(&all)
            };
            let rep = perturb::run_diagnostics(&axis, &eval);
            println!("  diagnostics variants_tried={} oat={}/{} ({:.3}) joint={}/{} ({:.3}) elapsed={:?} budget={}s within_budget={} oat_gate_met={} joint_gate_met={}",
                rep.variants_tried(), rep.oat_positive_cells, rep.oat_cells, rep.oat_positive_fraction,
                rep.joint_positive_draws, rep.joint_draws, rep.joint_positive_fraction,
                rep.elapsed, perturb::DIAGNOSTICS_BUDGET_SECS, rep.within_budget(),
                rep.oat_gate_met(), rep.joint_gate_met());
        }
    }
    Ok(())
}

/// The fixed diagnostics parameter axis (config multipliers the ±20% perturbation perturbs).
fn diagnostic_axis(cfg: &Config) -> Vec<Decimal> {
    vec![
        cfg.bos_min_distance_atr,
        cfg.impulse_body_threshold_atr,
        cfg.stop_buffer_atr,
        cfg.min_zone_height_atr,
        cfg.max_zone_height_atr,
        cfg.target_r,
    ]
}

/// Rebuild a config with the diagnostics axis values applied (all other fields held).
fn apply_diagnostic_axis(cfg: &Config, v: &[Decimal]) -> Config {
    Config {
        bos_min_distance_atr: v[0],
        impulse_body_threshold_atr: v[1],
        stop_buffer_atr: v[2],
        min_zone_height_atr: v[3],
        max_zone_height_atr: v[4],
        target_r: v[5],
        ..cfg.clone()
    }
}

// ---------------------------------------------------------------------------------------------
// calibrate (unchanged)
// ---------------------------------------------------------------------------------------------

/// True when the low of `chunk` is visited before its high (intrabar order observed in 1m data).
/// `None` for a degenerate chunk where the low and high share a minute.
fn observed_low_first(chunk: &[Candle]) -> Option<bool> {
    let lo = chunk.iter().enumerate().min_by_key(|(_, c)| c.low)?.0;
    let hi = chunk.iter().enumerate().max_by_key(|(_, c)| c.high)?.0;
    if lo == hi {
        None
    } else {
        Some(lo < hi)
    }
}

fn calibrate(one_minute: &str, fetched_4h: &str) -> Res<()> {
    let minutes = load_csv_with_policy(one_minute, GapPolicy::ForwardFill, 60_000)?;
    let fetched = load_csv(fetched_4h)?;
    let aggregated: Vec<Candle> = minutes.chunks(240).map(aggregate_4h).collect();

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
        let observed: Vec<bool> = minutes.chunks(240).filter_map(observed_low_first).collect();
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
// fetch-data (unchanged except the OOS-window guard)
// ---------------------------------------------------------------------------------------------

fn interval_seconds(interval: &str) -> Res<i64> {
    Ok(match interval {
        "1min" => 60,
        "5min" => 300,
        "15min" => 900,
        "30min" => 1_800,
        "1hour" => 3_600,
        "2hour" => 7_200,
        "4hour" => 14_400,
        "6hour" => 21_600,
        "8hour" => 28_800,
        "12hour" => 43_200,
        "1day" => 86_400,
        "1week" => 604_800,
        other => return Err(format!("unknown interval: {other}").into()),
    })
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
    let mut candles: Vec<[String; 5]> = raw
        .iter()
        .map(|r| {
            let ms = r[0].parse::<i64>().unwrap_or(0) * 1000;
            [
                ms.to_string(),
                r[1].clone(),
                r[3].clone(),
                r[4].clone(),
                r[2].clone(),
            ]
        })
        .collect();
    candles.sort_by_key(|r| r[0].parse::<i64>().unwrap_or(0)); // oldest first
    candles.dedup_by_key(|r| r[0].clone()); // de-duplicate across page boundaries
    let mut wtr = csv::Writer::from_path(out)?;
    wtr.write_record(["open_time_ms", "open", "high", "low", "close"])?;
    for r in &candles {
        wtr.write_record(r)?;
    }
    wtr.flush()?;
    println!("{}", dataset_hash(out)?);
    Ok(())
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
