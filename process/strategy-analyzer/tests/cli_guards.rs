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
    assert!(
        out.status.success(),
        "backtest must now exit 0 (the perturbation panic is fixed): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("EXPLORATORY (in-sample; not a confirmatory result)"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("CONFIRMATORY OOS run"),
        "in-sample must not be labelled confirmatory: {stdout}"
    );
    // The diagnostics section is present and reports its invalid-cell counts and budget.
    assert!(stdout.contains("diagnostics variants_tried="), "{stdout}");
    assert!(stdout.contains("within_budget="), "{stdout}");
}

/// ITEM 5: `--oos` must name exactly one explicit mode; `both` would select the mode on OOS.
#[test]
fn backtest_oos_refuses_mode_both() {
    let out = run(&[
        "backtest",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
        "--oos",
        "--lock",
        "/tmp/strategy-analyzer-nonexistent.lock",
        "--mode",
        "both",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("exactly one explicit --mode"), "{stderr}");
}

/// ITEM 5: `--oos` must match the mode frozen in the lock; a different mode is refused with the
/// locked mode named (not just a bare hash mismatch).
#[test]
fn backtest_oos_refuses_a_mode_that_differs_from_the_lock() {
    let lock = std::env::temp_dir().join(format!(
        "strategy_analyzer_oos_mode_{}.lock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&lock);
    let lock_path = lock.to_str().unwrap();
    let freeze = run(&[
        "freeze-lock",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
        "--mode",
        "fixed_r",
        "--policy",
        "both",
        "--out",
        lock_path,
    ]);
    assert!(
        freeze.status.success(),
        "freeze-lock failed: {}",
        String::from_utf8_lossy(&freeze.stderr)
    );

    let out = run(&[
        "backtest",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--price-scale",
        "XRP-USDT=4",
        "--oos",
        "--lock",
        lock_path,
        "--mode",
        "structural",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not match the frozen lock mode"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&lock);
    let _ = std::fs::remove_file(format!("{lock_path}.oos.log"));
}
