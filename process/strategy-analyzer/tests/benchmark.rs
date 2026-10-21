//! `benchmark` CLI tests: the interval sweep table, the `no data` row for an interval with no cached
//! file, and the unknown-interval rejection. These spawn the built `backtest` binary and are fully
//! offline (cached fixtures via `--data`, `--no-fetch`).
#![cfg(feature = "harness")]

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_backtest"))
        .args(args)
        .output()
        .expect("spawn backtest CLI")
}

/// Build a tiny cached candle CSV on `interval_ms` from the committed happy-path fixture, whose OHLC
/// is interval-agnostic (the core is candle-count, not wall-clock, driven), by rewriting each row's
/// `open_time_ms` to `i * interval_ms`. Returns the temp file path.
fn fixture_at(interval_ms: i64, tag: &str) -> String {
    let src = std::fs::read_to_string("tests/fixtures/signal_4h.csv").unwrap();
    let mut out = String::from("open_time_ms,open,high,low,close\n");
    for (i, line) in src.lines().skip(1).enumerate() {
        let rest = line
            .split_once(',')
            .map(|(_, r)| r)
            .expect("fixture row has OHLC");
        out.push_str(&format!("{},{rest}\n", i as i64 * interval_ms));
    }
    let path = std::env::temp_dir().join(format!("sa_bench_{}_{}.csv", std::process::id(), tag));
    std::fs::write(&path, out).unwrap();
    path.to_str().unwrap().to_string()
}

fn row<'a>(stdout: &'a str, interval: &str) -> &'a str {
    stdout
        .lines()
        .find(|l| l.starts_with(&format!("{interval}\t")))
        .unwrap_or_else(|| panic!("no table row for {interval} in:\n{stdout}"))
}

/// A table row per requested interval: `5min`/`15min` from `--data`, each with the happy fixture's
/// one `low_first` signal and a finite mean net R; the uncached `1hour` is reported `no data` (with
/// `--no-fetch`) rather than aborting the run.
#[test]
fn benchmark_reports_a_row_per_interval_and_marks_missing_as_no_data() {
    let five = fixture_at(300_000, "5min");
    let fifteen = fixture_at(900_000, "15min");
    let out = run(&[
        "benchmark",
        "--symbol",
        "XRP-USDT",
        "--price-scale",
        "XRP-USDT=4",
        "--intervals",
        "5min,15min,1hour",
        "--data",
        &format!("5min={five}"),
        "--data",
        &format!("15min={fifteen}"),
        "--no-fetch",
        "--policy",
        "low_first",
    ]);
    assert!(
        out.status.success(),
        "benchmark failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Columns: interval, signals, signals_per_day, trades, win_rate, mean_net_r, median_net_r,
    // total_net_r, mean_cost_r, covers_fees.
    let five_cols: Vec<&str> = row(&stdout, "5min").split('\t').collect();
    assert_eq!(
        five_cols[1],
        "1",
        "5min signal count: {}",
        row(&stdout, "5min")
    );
    let mean: f64 = five_cols[5].parse().unwrap();
    assert!(
        mean.is_finite(),
        "5min mean net R must be finite: {five_cols:?}"
    );

    let fifteen_cols: Vec<&str> = row(&stdout, "15min").split('\t').collect();
    assert_eq!(
        fifteen_cols[1],
        "1",
        "15min signal count: {}",
        row(&stdout, "15min")
    );

    let missing = row(&stdout, "1hour");
    assert!(
        missing.contains("no data"),
        "an uncached interval must be a `no data` row: {missing}"
    );
}

/// An unknown interval string in `--intervals` is rejected (via the shared `interval_to_ms`), never
/// silently skipped.
#[test]
fn benchmark_rejects_unknown_intervals() {
    let five = fixture_at(300_000, "unknown");
    let out = run(&[
        "benchmark",
        "--symbol",
        "XRP-USDT",
        "--price-scale",
        "XRP-USDT=4",
        "--intervals",
        "5min,7min",
        "--data",
        &format!("5min={five}"),
        "--no-fetch",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown candle interval"),
        "unknown interval must be rejected: {stderr}"
    );
}
