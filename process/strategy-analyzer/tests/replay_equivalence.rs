//! Replay-equivalence integration tests. The golden hash covers the concatenated outputs of the
//! four hand-derived Task 27 fixtures (happy path + second touch + two rejection variants), so it
//! exercises real signal and rejection paths. Gated behind `harness` so `--no-default-features`
//! still compiles the test target (empty).
#![cfg(feature = "harness")]

use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

/// The four hand-derived Task 27 fixtures, in a fixed order. Hashed together so the golden also
/// covers the rejection paths (`mitigated`, `obstacle_too_close`), not just the happy path.
const FIXTURES: [&str; 4] = [
    "tests/fixtures/signal_4h.csv",
    "tests/fixtures/signal_second_touch_4h.csv",
    "tests/fixtures/signal_mitigated_4h.csv",
    "tests/fixtures/signal_obstacle_too_close_4h.csv",
];

fn replay_all_outputs(path: &str, policy: PathPolicy) -> Vec<Output> {
    let candles = load_csv(path).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut s = StrategyState::new("XRP-USDT", 1);
    let mut outs: Vec<Output> = Vec::new();
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let (ns, mut o) = step(&s, Input::PriceTick(t), &cfg).unwrap();
            s = ns;
            outs.append(&mut o);
        }
        let (ns, mut o) = step(&s, Input::ClosedCandle(candle), &cfg).unwrap();
        s = ns;
        outs.append(&mut o);
    }
    outs
}

#[test]
fn golden_hash_is_stable() {
    let mut outs: Vec<Output> = Vec::new();
    for path in FIXTURES {
        outs.extend(replay_all_outputs(path, PathPolicy::LowFirst));
    }
    let digest = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(serde_json::to_string(&outs).unwrap().as_bytes());
        format!("{:x}", h.finalize())
    };
    let path = "tests/golden/outs.hash";
    let golden = std::fs::read_to_string(path).unwrap_or_default();
    if golden.trim().is_empty() {
        std::fs::create_dir_all("tests/golden").unwrap();
        std::fs::write(path, &digest).unwrap();
    } else {
        assert_eq!(golden.trim(), digest, "cross-platform golden hash mismatch");
    }
}
