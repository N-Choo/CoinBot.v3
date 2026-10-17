//! Hand-derived signal-producing fixtures (Task 27). The series are derived from the spec, not
//! tuned to the implementation; see the plan's Step 1 derivation. Gated behind `harness` so the
//! core-only `--no-default-features` test run still compiles.
#![cfg(feature = "harness")]

use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

/// Count `EntrySignal` outputs while replaying a cached OHLCV file under one path policy.
fn count_signals(path: &str, policy: PathPolicy) -> usize {
    let candles = load_csv(path).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
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
    n
}

#[test]
fn signal_fixture_yields_exactly_one_signal() {
    assert_eq!(
        count_signals("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst),
        1,
        "hand-derived fixture must produce exactly one signal"
    );
}

#[test]
fn signal_fixture_counts_match_derivation() {
    let cases = [
        ("tests/fixtures/signal_4h.csv", 1usize),
        ("tests/fixtures/signal_second_touch_4h.csv", 1),
        ("tests/fixtures/signal_mitigated_4h.csv", 0),
        ("tests/fixtures/signal_obstacle_too_close_4h.csv", 0),
    ];
    for (path, expected) in cases {
        assert_eq!(
            count_signals(path, PathPolicy::LowFirst),
            expected,
            "signal count mismatch for {path}"
        );
    }
}
