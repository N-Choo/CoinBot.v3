//! Replay-equivalence integration tests. The golden hash covers the concatenated outputs of the
//! four hand-derived Task 27 fixtures (happy path + second touch + two rejection variants), so it
//! exercises real signal and rejection paths. Gated behind `harness` so `--no-default-features`
//! still compiles the test target (empty).
#![cfg(feature = "harness")]

use rust_decimal::Decimal;
use std::str::FromStr;
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{
    ConsumeReason, Input, Output, StrategyState, Tick, ZoneEventReason,
};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

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

    // The restart is injected as the core `Resume` input (spec §4) after deserialization, so the
    // snapshot-resume property covers the real reconnect path, not just a struct round-trip. Splits
    // are taken at CANDLE boundaries (5 events/candle): split 100 is before candle 20's ticks (no
    // zone yet) and split 105 is after candle 20's close (the zone is Active). A mid-candle resume
    // legitimately diverges — `Resume` forgets the last price, so an unsampled move through the zone
    // becomes a `missed_touch` (that path is covered by
    // `resume_after_downtime_price_through_zone_is_missed`), not equivalence.
    for split in [20 * 5, 21 * 5] {
        let (mid_state, _) = run_events(StrategyState::new("XRP-USDT", 1), &all[..split], &cfg);
        // Compare serialized JSON strings, not `==`: Decimal equality ignores scale, so `==` would
        // pass even if the snapshot round-trip changed a scale (which would break byte-equality).
        let snapshot = serde_json::to_string(&mid_state).unwrap();
        let resumed: StrategyState = serde_json::from_str(&snapshot).unwrap();
        let (resumed, resume_outs) = step(&resumed, Input::Resume, &cfg).unwrap();
        assert!(resume_outs.is_empty(), "a resume emits no output");
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

/// Restart with a genuine gap: the price moves through the zone while the core is down, so the first
/// tick back is already inside it. `Resume` forgets the previous price, making that first
/// observation a `missed_touch` (§7.2) — it must never be published as a stale entry.
#[test]
fn resume_after_downtime_price_through_zone_is_missed() {
    let cfg = Config::default_for(&["XRP-USDT"]);
    let all = fixture_events("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst);
    // Split 105 = just after candle 20 (the breakout) closes: the demand zone [104.0, 105.0] is
    // Active and awaiting its first tick.
    let split = 21 * 5;
    let (mid_state, _) = run_events(StrategyState::new("XRP-USDT", 1), &all[..split], &cfg);
    let snapshot = serde_json::to_string(&mid_state).unwrap();
    let resumed: StrategyState = serde_json::from_str(&snapshot).unwrap();
    let (resumed, _) = step(&resumed, Input::Resume, &cfg).unwrap();

    // The first tick back is already inside the zone (104.5 <= zone_high 105.0).
    let (after, outs) = step(
        &resumed,
        Input::PriceTick(Tick {
            time_ms: 302_400_001,
            price: dec("104.5"),
            is_bar_open: false,
        }),
        &cfg,
    )
    .unwrap();

    assert!(
        !outs.iter().any(|o| matches!(o, Output::EntrySignal(_))),
        "a restart at an unobserved touch must not signal"
    );
    assert!(
        outs.iter().any(|o| matches!(
            o,
            Output::ZoneEvent(ev)
                if ev.reason
                    == Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch))
        )),
        "the first tick inside the zone after a downtime must be a missed_touch"
    );
    assert_eq!(after.missed_touch_count, 1);
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

    // `UPDATE_GOLDEN=1` regenerates the committed golden without editing this assertion (report the
    // before/after digest in the commit when the core output legitimately changes).
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::write(path, format!("{digest}\n")).expect("write golden hash");
        eprintln!("UPDATE_GOLDEN=1: wrote {path} = {digest}");
        return;
    }

    // Default behavior: the golden hash is committed and must be compared on every CI matrix leg
    // (x86_64 and arm64). A missing file is a hard failure, never a silent pass.
    let golden = match std::fs::read_to_string(path) {
        Ok(g) => g,
        Err(e) => panic!(
            "required golden hash {path} is missing ({e}); it is never written by the test \
             (run with UPDATE_GOLDEN=1 to write it). computed digest: {digest}"
        ),
    };
    assert_eq!(
        golden.trim(),
        digest,
        "cross-platform golden hash mismatch: computed {digest}, golden {}. \
         Re-run with UPDATE_GOLDEN=1 to regenerate.",
        golden.trim()
    );
}
