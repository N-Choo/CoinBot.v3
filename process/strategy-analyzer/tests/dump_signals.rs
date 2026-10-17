//! `dump-signals` CLI tests: the one-JSON-object-per-line signal payload and the in-sample/OOS
//! boundary guard. These spawn the built `backtest` binary and never touch the network.
#![cfg(feature = "harness")]

use rust_decimal::Decimal;
use std::process::{Command, Output};
use std::str::FromStr;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_backtest"))
        .args(args)
        .output()
        .expect("spawn backtest CLI")
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// The happy fixture at 1 dp emits exactly one signal under `low_first`; every emitted level must be
/// the hand-derived value (Task 27 derivation), pinned here independently of the golden hash.
#[test]
fn dump_signals_prints_one_json_line_with_derived_fields() {
    let out = run(&[
        "dump-signals",
        "--data",
        "XRP-USDT=tests/fixtures/signal_4h.csv",
        "--price-scale",
        "XRP-USDT=1",
        "--policy",
        "low_first",
    ]);
    assert!(
        out.status.success(),
        "dump-signals failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one JSON line, got: {stdout}"
    );
    let v: serde_json::Value = serde_json::from_str(lines[0]).expect("line must be a JSON object");

    // Decimals serialize as fixed-scale strings; compare as `Decimal` so scale is not significant.
    let field = |name: &str| -> Decimal {
        let s = v[name]
            .as_str()
            .unwrap_or_else(|| panic!("{name} must be a string: {v}"));
        dec(s)
    };
    assert_eq!(field("trigger_price"), dec("104.5"));
    assert_eq!(field("reference_entry_price"), dec("104.5"));
    assert_eq!(field("stop_loss"), dec("103.9"));
    assert_eq!(field("take_profit"), dec("106.3"));
    assert_eq!(field("obstacle_rr"), dec("7.5"));
    // Stable identity (pinned literals, not derived from the golden hash).
    assert_eq!(v["signal_id"], "XRP-USDT:288000000:273600000:demand:0");
    assert_eq!(v["zone_id"], "XRP-USDT:288000000:273600000:demand");
}

/// `dump-signals` is in-sample only: a dataset with a row at/after the boundary is refused, exactly
/// like `count-signals` (no partial dump, no signal lines).
#[test]
fn dump_signals_refuses_oos_rows() {
    let out = run(&[
        "dump-signals",
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "no signal may be dumped for OOS rows: {stdout}"
    );
}
