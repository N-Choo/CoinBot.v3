//! The real happy-path fixture signal, filled through the position sim under both intrabar path
//! policies. The entry is the touching tick (104.5); a "take profit" at a negative R must never be
//! produced. Gated behind `harness` so `--no-default-features` still compiles the target.
#![cfg(feature = "harness")]

use rust_decimal::RoundingStrategy;
use rust_decimal::Decimal;
use std::str::FromStr;
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::sim::{simulate, SimOutcome, SimParams};
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, Signal, StrategyState, Tick};

/// Replay the happy fixture, returning (the first emitted signal, the flat tick stream, and the
/// flat index of the signal's touching tick).
fn signal_and_ticks(policy: PathPolicy) -> (Signal, Vec<Tick>, usize) {
    let candles = load_csv("tests/fixtures/signal_4h.csv").unwrap();
    let mut cfg = Config::default_for(&["XRP-USDT"]);
    cfg.price_scale.insert("XRP-USDT".to_string(), 1); // 1 dp, per the fixture derivation
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    let mut flat: Vec<Tick> = Vec::new();
    let mut found: Option<(Signal, usize)> = None;
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let idx = flat.len();
            let (ns, outs) = step(&st, Input::PriceTick(t), &cfg).unwrap();
            st = ns;
            flat.push(t);
            for o in outs {
                if let Output::EntrySignal(s) = o {
                    found = Some((s, idx));
                }
            }
        }
        let (ns, _) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap();
        st = ns;
    }
    let (sig, idx) = found.expect("fixture must emit a signal");
    (sig, flat, idx)
}

/// Pre-registered fill costs with borrow off so the hand-derived R below is exact.
fn params() -> SimParams {
    SimParams {
        fee_bps: Decimal::from(5),
        entry_slip_bps: Decimal::from(5),
        stop_slip_bps: Decimal::from(15),
        borrow_bps_per_day: Decimal::ZERO,
    }
}

/// Hand derivation (plan Task 27 / spec §7.5): trigger 104.5, stop 103.9, TP 106.3, risk 0.6. The
/// touching tick is the 104.5 low in BOTH policies, so entry = 104.5 + 104.5*5bps = 104.55225 and
/// cost = 104.55225*(2*5bps) = 0.10455225.
///
///  * `low_first` path O(107.5) -> L(104.5) -> H(108.0) -> C(105.5): the touch is at L, then H
///    reaches the 106.3 target. R = (106.3 - 104.55225 - 0.10455225)/0.6 = 1.64319775/0.6
///    = 2.7386629166... ≈ 2.74 (> 0).
///  * `high_first` path O(107.5) -> H(108.0) -> L(104.5) -> C(105.5): the touch is at L *after* the
///    high, and the only remaining tick (105.5) clears neither 106.3 nor 103.9, so the position is
///    `Unresolved` at end of data.
///
/// Neither policy may report a `TakeProfit` at a negative R (the old latency-index fill did).
#[test]
fn happy_fixture_signal_fills_sanely_under_both_policies() {
    let dec = |s: &str| Decimal::from_str(s).unwrap();

    let (sig, ticks, idx) = signal_and_ticks(PathPolicy::LowFirst);
    assert_eq!(sig.trigger_price, dec("104.5"));
    match simulate(&sig, &ticks[idx..], params()).outcome {
        SimOutcome::TakeProfit(r) => {
            assert!(r > Decimal::ZERO, "low_first TP R must be positive, got {r}");
            assert_eq!(
                r.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
                dec("2.74"),
                "hand-derived low_first TP R"
            );
        }
        other => panic!("low_first: expected a positive TP, got {other:?}"),
    }

    let (sig, ticks, idx) = signal_and_ticks(PathPolicy::HighFirst);
    assert_eq!(sig.trigger_price, dec("104.5"));
    match simulate(&sig, &ticks[idx..], params()).outcome {
        SimOutcome::Unresolved => {}
        SimOutcome::TakeProfit(r) => panic!("high_first: bogus take-profit R={r}"),
        other => panic!("high_first: expected Unresolved, got {other:?}"),
    }
}
