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

/// The fixture's event stream: four ticks then the `ClosedCandle`, per candle (spec §4 ordering).
fn fixture_events(path: &str, policy: PathPolicy) -> Vec<Input> {
    let candles = load_csv(path).unwrap();
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    let mut ev = Vec::new();
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            ev.push(Input::PriceTick(t));
        }
        ev.push(Input::ClosedCandle(candle));
    }
    ev
}

/// Drive a state through `events`, returning the final state and the outputs *per event*.
fn run_events(
    state: StrategyState,
    events: &[Input],
    cfg: &Config,
) -> (StrategyState, Vec<Vec<Output>>) {
    let mut st = state;
    let mut per = Vec::with_capacity(events.len());
    for ev in events {
        let (ns, outs) = step(&st, ev.clone(), cfg).unwrap();
        st = ns;
        per.push(outs);
    }
    (st, per)
}

#[test]
fn snapshot_resume_matches_uninterrupted_on_signal_fixture() {
    let cfg = Config::default_for(&["XRP-USDT"]);
    let all = fixture_events("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst);
    let (full_state, full_per) = run_events(StrategyState::new("XRP-USDT", 1), &all, &cfg);

    // 5 events per candle: snapshot after candle 20 (zone active) and mid-candle-21 (after its
    // first tick), i.e. splits 105 and 106.
    for split in [21 * 5, 21 * 5 + 1] {
        let (mid_state, _) = run_events(StrategyState::new("XRP-USDT", 1), &all[..split], &cfg);
        // Compare serialized JSON strings, not `==`: Decimal equality ignores scale, so `==` would
        // pass even if the snapshot round-trip changed a scale (which would break byte-equality).
        let snapshot = serde_json::to_string(&mid_state).unwrap();
        let resumed: StrategyState = serde_json::from_str(&snapshot).unwrap();
        let (tail_state, tail_per) = run_events(resumed, &all[split..], &cfg);
        assert_eq!(
            serde_json::to_string(&tail_state).unwrap(),
            serde_json::to_string(&full_state).unwrap(),
            "state after resume must match the uninterrupted run (split {split})"
        );
        assert_eq!(
            serde_json::to_string(&tail_per).unwrap(),
            serde_json::to_string(&full_per[split..]).unwrap(),
            "outputs after resume must match the uninterrupted run (split {split})"
        );
    }
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
    // No write-if-empty fallback: the golden hash is committed and must be compared on every CI
    // matrix leg (x86_64 and arm64). A missing golden file is a hard failure, never a silent pass.
    let golden = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("required golden hash {path} is missing ({e}); it is never written by the test")
    });
    assert_eq!(golden.trim(), digest, "cross-platform golden hash mismatch");
}
