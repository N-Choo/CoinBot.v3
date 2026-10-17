//! Hand-derived signal-producing fixture (Task 27). The series is derived from the spec, not
//! tuned to the implementation; see the plan's Step 1 derivation. Gated behind `harness` so the
//! core-only `--no-default-features` test run still compiles.
#![cfg(feature = "harness")]

use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

#[test]
fn signal_fixture_yields_exactly_one_signal() {
    let candles = load_csv("tests/fixtures/signal_4h.csv").unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, PathPolicy::LowFirst);
    let mut n = 0usize;
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let (ns, o) = step(&st, Input::PriceTick(t), &cfg).unwrap();
            st = ns;
            n += o.iter().filter(|o| matches!(o, Output::EntrySignal(_))).count();
        }
        let (ns, _) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap();
        st = ns;
    }
    assert_eq!(n, 1, "hand-derived fixture must produce exactly one signal");
}
