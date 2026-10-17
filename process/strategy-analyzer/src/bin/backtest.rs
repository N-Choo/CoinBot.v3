use clap::{Parser, Subcommand};
use std::time::Duration;
use strategy_analyzer::harness::dataset::{dataset_hash, load_csv};
use strategy_analyzer::harness::fetch;
use strategy_analyzer::harness::lock;
use strategy_analyzer::harness::perturb;
use strategy_analyzer::harness::report::{adversarial_composite, cushion_warning, summarize, time_blocked_bootstrap, TradeRecord, BOOTSTRAP_BLOCK_MS};
use strategy_analyzer::harness::run::{run, RunResult};
use strategy_analyzer::harness::sim::SimParams;
use strategy_analyzer::harness::tick_source::PathPolicy;
use strategy_analyzer::strategy::config::{config_version, Config, TargetMode};
use rust_decimal::Decimal;

const KUCOIN_BASE: &str = "https://api.kucoin.com";
const SYMBOL: &str = "XRP-USDT";

#[derive(Parser)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    /// Fetch OHLCV and cache to a file. Never used by `backtest`.
    FetchData {
        #[arg(long)] symbol: String,
        #[arg(long, default_value = "4hour")] interval: String,
        #[arg(long)] start_s: i64,
        #[arg(long)] end_s: i64,
        #[arg(long)] out: String,
    },
    /// Run the backtest offline against a cached dataset.
    Backtest {
        #[arg(long)] data: String,
        #[arg(long, default_value = "both")] policy: String,
        #[arg(long, default_value = "both")] mode: String,
        #[arg(long)] lock: Option<String>,
        #[arg(long, default_value_t = false)] oos: bool,
        #[arg(long, default_value_t = false)] oos_consumed: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().cmd {
        Cmd::FetchData { symbol, interval, start_s, end_s, out } =>
            fetch_data(&symbol, &interval, start_s, end_s, &out),
        Cmd::Backtest { data, policy, mode, lock, oos, oos_consumed } =>
            backtest(&data, &policy, &mode, lock, oos, oos_consumed),
    }
}

/// Pre-registered sim defaults (freeze-lock contents): 5 bps fee/side, 5 bps entry slip,
/// 15 bps (worse) stop slip, 2 bps/day borrow, 1-tick latency.
fn default_sim_params() -> SimParams {
    SimParams {
        fee_bps: Decimal::from(5),
        entry_slip_bps: Decimal::from(5),
        stop_slip_bps: Decimal::from(15),
        borrow_bps_per_day: Decimal::from(2),
        latency_ticks: 1,
    }
}

fn zero_cost_params(p: SimParams) -> SimParams {
    SimParams { fee_bps: Decimal::ZERO, entry_slip_bps: Decimal::ZERO, stop_slip_bps: Decimal::ZERO, borrow_bps_per_day: Decimal::ZERO, latency_ticks: p.latency_ticks }
}

/// Decimal portfolio mean R (0 when there are no trades).
fn mean_r(trades: &[TradeRecord]) -> Decimal {
    if trades.is_empty() { return Decimal::ZERO; }
    trades.iter().map(|t| t.r).sum::<Decimal>() / Decimal::from(trades.len() as u64)
}

/// The fixed diagnostics parameter axis (config multipliers the ±20% perturbation perturbs).
/// Order is documented here; diagnostics only — never written back to the frozen config.
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

fn policy_name(p: PathPolicy) -> &'static str {
    match p { PathPolicy::LowFirst => "low_first", PathPolicy::HighFirst => "high_first" }
}

fn mode_name(m: TargetMode) -> &'static str {
    match m { TargetMode::FixedR => "fixed_r", TargetMode::Structural => "structural" }
}

fn interval_seconds(interval: &str) -> Result<i64, Box<dyn std::error::Error>> {
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

fn fetch_data(symbol: &str, interval: &str, start_s: i64, end_s: i64, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let interval_s = interval_seconds(interval)?;
    let mut raw: Vec<[String; 7]> = Vec::new();
    for (s, e) in fetch::page_windows(start_s, end_s, interval_s) {
        raw.extend(fetch_with_retry(symbol, interval, s, e)?);
        std::thread::sleep(Duration::from_millis(250)); // throttle: respect rate limits
    }
    // KuCoin columns: time,open,close,high,low,volume,turnover -> ms + OHLC cache order.
    let mut candles: Vec<[String; 5]> = raw.iter().map(|r| {
        let ms = r[0].parse::<i64>().unwrap_or(0) * 1000;
        [ms.to_string(), r[1].clone(), r[3].clone(), r[4].clone(), r[2].clone()]
    }).collect();
    candles.sort_by_key(|r| r[0].parse::<i64>().unwrap_or(0)); // oldest first
    candles.dedup_by_key(|r| r[0].clone());                    // de-duplicate across page boundaries
    let mut wtr = csv::Writer::from_path(out)?;
    wtr.write_record(["open_time_ms", "open", "high", "low", "close"])?;
    for r in &candles { wtr.write_record(r)?; }
    wtr.flush()?;
    println!("{}", dataset_hash(out)?);
    Ok(())
}

fn fetch_with_retry(symbol: &str, interval: &str, s: i64, e: i64) -> Result<Vec<[String; 7]>, Box<dyn std::error::Error>> {
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

fn backtest(data: &str, policy: &str, mode: &str, lock_path: Option<String>, oos: bool, oos_consumed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let candles = load_csv(data)?;
    let hash = dataset_hash(data)?;
    let params = default_sim_params();
    let backtest_params_json = serde_json::json!({
        "policy": policy,
        "mode": mode,
        "fee_bps": params.fee_bps.to_string(),
        "entry_slip_bps": params.entry_slip_bps.to_string(),
        "stop_slip_bps": params.stop_slip_bps.to_string(),
        "borrow_bps_per_day": params.borrow_bps_per_day.to_string(),
        "latency_ticks": params.latency_ticks,
        "bootstrap_block_ms": BOOTSTRAP_BLOCK_MS,
        "bootstrap_resamples": 10_000,
        "bootstrap_seed": 42,
    }).to_string();

    let cfg0 = Config::default_for(&[SYMBOL]);
    let cv = config_version(&cfg0.canonical_json());
    let expected_lock = lock::lock_hash(&cv, &backtest_params_json);

    let mut non_confirmatory = false;
    if oos {
        let path = lock_path.as_deref().ok_or("--oos requires --lock <path>")?;
        lock::check_lock(path, &expected_lock)?;
        let consumed = format!("{path}.consumed");
        match lock::oos_gate(&consumed, oos_consumed)? {
            lock::OosDecision::Allow => {}
            lock::OosDecision::Refuse => {
                return Err(format!("OOS already consumed at {consumed}; pass --oos-consumed to override (stamps non-confirmatory)").into());
            }
            lock::OosDecision::RerunNonConfirmatory => {
                // Contract of this outcome: keep/overwrite the marker and stamp non-confirmatory.
                non_confirmatory = true;
                if let Ok(prior) = std::fs::read_to_string(&consumed) {
                    println!("# OOS override: prior run hash = {}", prior.trim());
                }
            }
        }
        lock::write_lock(&consumed, &hash)?; // log this invocation (one-shot guard)
    }

    let policies: Vec<PathPolicy> = match policy {
        "low_first" => vec![PathPolicy::LowFirst],
        "high_first" => vec![PathPolicy::HighFirst],
        "both" => vec![PathPolicy::LowFirst, PathPolicy::HighFirst],
        other => return Err(format!("unknown policy: {other}").into()),
    };
    let modes: Vec<TargetMode> = match mode {
        "fixed_r" => vec![TargetMode::FixedR],
        "structural" => vec![TargetMode::Structural],
        "both" => vec![TargetMode::FixedR, TargetMode::Structural],
        other => return Err(format!("unknown mode: {other}").into()),
    };

    // §8.5 header: symbol(s), resolution, dataset hash, config_version, path policy, target mode,
    // and the resolution caveat.
    println!("# strategy-analyzer backtest");
    println!("# symbol={SYMBOL} resolution=4h-synthesized dataset={data}");
    println!("# dataset_hash={hash}");
    println!("# config_version={cv}");
    println!("# path_policy={policy} target_mode={mode}");
    println!("# caveat: 4H synthesis (4 ticks/candle) => latency-insensitive; sim approximates engine net-R:R");
    if non_confirmatory { println!("# NON-CONFIRMATORY (one-shot OOS overridden)"); }

    for m in &modes {
        let cfg = Config { target_mode: *m, ..Config::default_for(&[SYMBOL]) };
        let mut runs: Vec<(PathPolicy, RunResult)> = Vec::new();
        for p in &policies {
            runs.push((*p, run(SYMBOL, &candles, *p, &cfg, params)));
        }
        println!("\n## mode={}", mode_name(*m));
        for (p, r) in &runs {
            let s = summarize(&r.trades);
            println!("  path={:<10} signals={} trades={} skipped_open_position={} missed_touch={} mean_r={:.4} win_rate={:.4} long_mean_r={:.4} short_mean_r={:.4}",
                policy_name(*p), r.signals, s.n, r.skipped_open_position, r.missed_touch,
                s.mean_r, s.win_rate, s.by_direction_long, s.by_direction_short);
            for (pair, mean) in &s.by_pair {
                println!("    pair={pair} mean_r={mean:.4}");
            }
        }
        // Adversarial composite: longs from low_first, shorts from high_first.
        let low = runs.iter().find(|(p, _)| *p == PathPolicy::LowFirst).map(|(_, r)| r);
        let high = runs.iter().find(|(p, _)| *p == PathPolicy::HighFirst).map(|(_, r)| r);
        if let (Some(l), Some(h)) = (low, high) {
            let composite = adversarial_composite(&l.trades, &h.trades);
            let cs = summarize(&composite);
            let (mean, lower) = time_blocked_bootstrap(&composite, BOOTSTRAP_BLOCK_MS, 10_000, 42);
            println!("  adversarial_composite n={} mean_r={:.4} bootstrap_mean={:.4} bootstrap_5pct_lower={:.4}",
                cs.n, cs.mean_r, mean, lower);
            // Cushion calibration: realized cost per trade in R, measured as the gap between a
            // zero-cost and the with-cost composite (both use the same pre-registered fill model).
            let zero = zero_cost_params(params);
            let l0 = run(SYMBOL, &candles, PathPolicy::LowFirst, &cfg, zero);
            let h0 = run(SYMBOL, &candles, PathPolicy::HighFirst, &cfg, zero);
            let comp0 = summarize(&adversarial_composite(&l0.trades, &h0.trades));
            let mean_cost_r = comp0.mean_r - cs.mean_r;
            match cushion_warning(mean_cost_r) {
                Some(w) => println!("  cushion mean_cost_r={mean_cost_r:.4} WARNING: {w}"),
                None => println!("  cushion mean_cost_r={mean_cost_r:.4} ok (<= 0.5R)"),
            }

            // Perturbation diagnostics: 4H synthesis only (`run` always synthesizes 4H candles),
            // seeded and reproducible, and never used to select parameters.
            let axis = diagnostic_axis(&cfg);
            let eval = |vals: &[Decimal]| -> Decimal {
                let c = apply_diagnostic_axis(&cfg, vals);
                let l = run(SYMBOL, &candles, PathPolicy::LowFirst, &c, params);
                let h = run(SYMBOL, &candles, PathPolicy::HighFirst, &c, params);
                mean_r(&adversarial_composite(&l.trades, &h.trades))
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
