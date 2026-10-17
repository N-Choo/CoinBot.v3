//! `run` pass 2 opens at most one position at a time: a signal whose entry falls while an
//! earlier position is still open is counted in `skipped_open_position` and is not traded.
//! This drives a hand-built 4H series that emits two long signals with the second signal's
//! entry before the first position resolves. Gated behind `harness` so `--no-default-features`
//! still compiles the target.
#![cfg(feature = "harness")]

use rust_decimal::Decimal;
use std::str::FromStr;
use strategy_analyzer::harness::run::run;
use strategy_analyzer::harness::sim::SimParams;
use strategy_analyzer::harness::tick_source::PathPolicy;
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::types::Candle;

fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

fn candle(open_time_ms: i64, open: &str, high: &str, low: &str, close: &str) -> Candle {
    Candle { open_time_ms, open: dec(open), high: dec(high), low: dec(low), close: dec(close) }
}

/// Same pre-registered sim defaults as the `backtest` CLI (5 bps fee/side, 5 bps entry slip,
/// 15 bps stop slip, 2 bps/day borrow, 1-tick latency). The skip is a time/ordering property
/// of pass 2 and does not depend on the cost model.
fn params() -> SimParams {
    SimParams {
        fee_bps: Decimal::from(5), entry_slip_bps: Decimal::from(5), stop_slip_bps: Decimal::from(15),
        borrow_bps_per_day: Decimal::from(2), latency_ticks: 1,
    }
}

/// Build a 4H series (candle index `i` opens at `i * 14_400_000` ms) that produces TWO demand
/// (long) signals, the second while the first position is still open:
///
///  * Candles 0..20 are flat (high-low 1.0) so Wilder ATR settles at ~= 1.0.
///  * Block 1: a swing low at candle 20 (99.0) and a swing high at candle 23 (101.0). Candle 26
///    is the bearish zone candle [100.0, 103.0] (height 3.0 = max, so risk is large and TP1 is
///    far); candle 27 closes at 113.0, a bullish BOS above 101.0 that creates demand zone 1
///    [100.0, 103.0]. Candle 28 dips and first-touches zone 1's proximal edge at 103.0 -> signal 1
///    (stop ~99.89 below at 0.1*ATR from the zone low; TP1 ~112.33, resisting slippage by design).
///  * Block 2: a NEW swing high forms at candle 31 (105.2, confirmed at candle 33). Candle 35 is
///    the bearish zone candle [101.0, 103.0]; candle 36 closes at 110.0, a bullish BOS above
///    105.2 that creates demand zone 2 [101.0, 103.0]. Its breakout high 110.5 stays below TP1
///    (~112.33), so the open position 1 is not resolved or stopped (99.89 < 110.5 < 112.33).
///  * Candle 38 dips and first-touches zone 2's edge at 103.0 -> signal 2. Pass 2 order puts
///    signal 1 first; its position is still open (it never hits stop/TP), so signal 2's entry is
///    skipped rather than traded.
fn two_signal_series() -> Vec<Candle> {
    const STEP: i64 = 14_400_000;
    let c = |i: i64, o: &str, h: &str, l: &str, cl: &str| candle(i * STEP, o, h, l, cl);
    let mut out = Vec::new();
    for i in 0..20 { out.push(c(i, "100.0", "100.5", "99.5", "100.0")); } // 0..20: ATR warmup
    out.push(c(20, "100.0", "100.2", "99.0", "99.5"));   // swing low 99.0
    out.push(c(21, "99.5", "100.0", "99.2", "99.8"));
    out.push(c(22, "99.8", "100.5", "99.6", "100.2"));
    out.push(c(23, "100.2", "101.0", "100.0", "100.6")); // swing high 101.0
    out.push(c(24, "100.6", "100.8", "100.2", "100.4"));
    out.push(c(25, "100.4", "100.6", "100.0", "100.2"));
    out.push(c(26, "103.0", "103.0", "100.0", "100.2")); // zone-1 candle [100.0, 103.0]
    out.push(c(27, "100.2", "113.5", "100.1", "113.0")); // BOS1 -> demand zone 1, high 113.5
    out.push(c(28, "105.0", "105.5", "103.0", "104.0")); // touch1 at 103.0 -> signal 1
    out.push(c(29, "104.0", "104.5", "103.0", "104.0"));
    out.push(c(30, "104.0", "105.0", "103.8", "104.6"));
    out.push(c(31, "104.6", "105.2", "104.2", "104.8")); // new swing high 105.2
    out.push(c(32, "104.8", "104.9", "104.0", "104.3"));
    out.push(c(33, "104.3", "104.5", "103.9", "104.1")); // confirms the swing high at 31
    out.push(c(34, "104.1", "104.4", "103.8", "104.0"));
    out.push(c(35, "103.0", "103.0", "101.0", "101.2")); // zone-2 candle [101.0, 103.0]
    out.push(c(36, "101.2", "110.5", "101.1", "110.0")); // BOS2 -> demand zone 2, high 110.5
    out.push(c(37, "107.0", "107.5", "103.5", "104.5"));
    out.push(c(38, "104.5", "105.0", "103.0", "103.5")); // touch2 at 103.0 -> signal 2 (skipped)
    out
}

#[test]
fn second_signal_while_position_open_is_skipped() {
    let cfg = Config::default_for(&["XRP-USDT"]);
    let res = run("XRP-USDT", &two_signal_series(), PathPolicy::LowFirst, &cfg, params());
    assert_eq!(res.signals, 2, "the series must emit two signals");
    assert_eq!(res.trades.len(), 1, "only the first signal may be traded");
    assert_eq!(res.skipped_open_position, 1, "the second signal arrives while position 1 is open");
}
