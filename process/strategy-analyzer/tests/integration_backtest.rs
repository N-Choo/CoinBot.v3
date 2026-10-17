//! End-to-end harness integration: both path policies replay the signal fixture and must agree
//! on the signal set. Gated behind `harness` so `--no-default-features` still compiles the target.
#![cfg(feature = "harness")]

use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::{Config, TargetMode};
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

#[test]
fn both_policies_run_and_match_signal_counts() {
    let candles = load_csv("tests/fixtures/signal_4h.csv").unwrap();
    let mut counts = Vec::new();
    for policy in [PathPolicy::LowFirst, PathPolicy::HighFirst] {
        let cfg = Config { target_mode: TargetMode::FixedR, ..Config::default_for(&["XRP-USDT"]) };
        let mut st = StrategyState::new("XRP-USDT", 1);
        let mut src = Ohlcv4hSynthesis::new(candles.clone(), policy);
        let mut signals = 0usize;
        while let Some((ticks, candle)) = src.next_candle() {
            for t in ticks { let (ns, _) = step(&st, Input::PriceTick(t), &cfg).unwrap(); st = ns; }
            let (ns, outs) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap(); st = ns;
            signals += outs.iter().filter(|o| matches!(o, Output::EntrySignal(_))).count();
        }
        counts.push(signals);
    }
    assert_eq!(counts[0], counts[1]);
}
