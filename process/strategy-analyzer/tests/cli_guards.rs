//! CLI guard tests (Batch B): the multi-symbol surface, the in-sample/OOS boundary refusals, and
//! the count-only subcommand. These spawn the built `backtest` binary and check the exit status and
//! stream output; they never touch the network.
#![cfg(feature = "harness")]

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_backtest"))
        .args(args)
        .output()
        .expect("spawn backtest CLI")
}

#[test]
fn count_signals_prints_per_symbol_counts_and_reasons() {
    let out = run(&[
        "count-signals",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--data",
        "MITIGATED=tests/fixtures/signal_mitigated_4h.csv",
        "--data",
        "OBSTACLE=tests/fixtures/signal_obstacle_too_close_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
        "--price-scale",
        "MITIGATED=4",
        "--price-scale",
        "OBSTACLE=4",
    ]);
    assert!(
        out.status.success(),
        "count-signals failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("symbol=XRP-USDT policy=low_first signals=1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("symbol=MITIGATED")
            && stdout.contains("signals=0 reason_counts [mitigated=1]"),
        "{stdout}"
    );
    assert!(stdout.contains("vetoed_obstacle_too_close=1"), "{stdout}");
    // Counts only: no verdict, no PnL, no trades.
    assert!(
        !stdout.contains("verdict n="),
        "counts-only output: {stdout}"
    );
    assert!(!stdout.contains("trades="), "counts-only output: {stdout}");
}

#[test]
fn backtest_refuses_oos_without_the_flag() {
    let out = run(&[
        "backtest",
        "--data",
        "XRP-USDT=tests/fixtures/oos_signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("touches the OOS window"), "{stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("verdict n="),
        "no verdict may be printed for OOS without --oos: {stdout}"
    );
}

#[test]
fn backtest_refuses_a_symbol_without_a_price_scale() {
    let out = run(&[
        "backtest",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no --price-scale"), "{stderr}");
}

#[test]
fn count_signals_refuses_oos_rows() {
    let out = run(&[
        "count-signals",
        "--data",
        "XRP-USDT=tests/fixtures/oos_signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("at/after the in-sample/OOS boundary"),
        "{stderr}"
    );
}

#[test]
fn backtest_labels_in_sample_output_exploratory() {
    let out = run(&[
        "backtest",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
        "--mode",
        "fixed_r",
    ]);
    // The run may exit non-zero because the *diagnostics* pass panics on a perturbed config
    // (deferred to a later batch); what matters here is that the header labels it exploratory.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("EXPLORATORY (in-sample; not a confirmatory result)"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("CONFIRMATORY OOS run"),
        "in-sample must not be labelled confirmatory: {stdout}"
    );
}
