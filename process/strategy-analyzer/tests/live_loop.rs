//! Loop-level tests for the Phase 1 shadow adapter: a fake tick/candle stream drives a real
//! [`SymbolLoop`] with no network. Covers signal logging + recorded stream + persisted state, restart
//! resumption without duplicate signals, and the gap/staleness -> `Resume` -> `MissedTouch` path.
//!
//! Gated on the `live` feature only (no `harness`), so `--features live` exercises it directly.
#![cfg(feature = "live")]

use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use strategy_analyzer::live::{LiveEvent, LiveInput, StateStore, SymbolLoop};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::types::{Candle, ConsumeReason, ZoneEventReason};
use strategy_analyzer::strategy::STATE_SCHEMA_VERSION;

const FOUR_H_MS: i64 = 14_400_000;
const SETTLE_MS: i64 = 5_000;
const STALENESS_MS: i64 = 30_000;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Parse the `open_time_ms,open,high,low,close` fixture without the harness `csv` dependency.
fn load_fixture(path: &str) -> Vec<Candle> {
    let text = std::fs::read_to_string(path).expect("read fixture");
    text.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let c: Vec<&str> = line.split(',').collect();
            Candle {
                open_time_ms: c[0].parse().unwrap(),
                open: dec(c[1]),
                high: dec(c[2]),
                low: dec(c[3]),
                close: dec(c[4]),
            }
        })
        .collect()
}

/// The harness's low-first intrabar path: open, low, high, close (spec §4 ordering).
fn synth_ticks(c: &Candle) -> Vec<(Decimal, i64)> {
    vec![
        (c.open, c.open_time_ms),
        (c.low, c.open_time_ms + 1),
        (c.high, c.open_time_ms + 2),
        (c.close, c.open_time_ms + 3),
    ]
}

/// A deterministic `(input, now_ms)` stream: four ticks then the closed candle, per candle.
fn event_stream(candles: &[Candle]) -> Vec<(LiveInput, i64)> {
    let mut out = Vec::new();
    for c in candles {
        for (price, at_ms) in synth_ticks(c) {
            out.push((LiveInput::Price { price, at_ms }, at_ms + 1));
        }
        let closed_now = c.open_time_ms + FOUR_H_MS + SETTLE_MS + 1;
        out.push((LiveInput::Candle(*c), closed_now));
    }
    out
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("sa_live_loop_{tag}_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn count_signals(events: &[LiveEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, LiveEvent::Signal(_)))
        .count()
}

#[test]
fn loop_logs_signal_appends_stream_and_persists_state() {
    let state_dir = temp_dir("state");
    let stream_dir = temp_dir("stream");
    let candles = load_fixture("tests/fixtures/signal_4h.csv");
    let events = event_stream(&candles);
    let store = Arc::new(StateStore::open(&state_dir, STATE_SCHEMA_VERSION).unwrap());

    let mut lp = SymbolLoop::new(
        "XRP-USDT",
        Config::default_for(&["XRP-USDT"]),
        store.clone(),
        &stream_dir,
        STALENESS_MS,
        0,
    )
    .unwrap();
    assert!(lp.is_cold(), "first run is a cold start");

    // Drive until the first signal, then stop (the zone is consumed and the state persisted).
    let mut signals = 0;
    let mut consumed = 0usize;
    for (input, now) in &events {
        let out = lp.process(input.clone(), *now).unwrap();
        signals += count_signals(&out);
        consumed += 1;
        if signals == 1 {
            break;
        }
    }
    assert_eq!(signals, 1, "the happy fixture emits exactly one signal");

    // The recorded input stream holds the consumed `(PriceTick) ` inputs, one JSON line each.
    let stream_lines = std::fs::read_to_string(lp.stream_path()).unwrap();
    let recorded = stream_lines.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(
        recorded, consumed,
        "every fed input is appended to the stream"
    );
    assert!(
        stream_lines.contains("PriceTick"),
        "the stream records the fed inputs: {stream_lines}"
    );

    // The state file exists (persisted after the zone consume).
    let state_file = state_dir.join("XRP-USDT.json");
    assert!(state_file.exists(), "state file written after the consume");
    drop(lp);

    // Restart from the persisted state: warm start, and the remaining inputs add no new signal.
    let mut lp2 = SymbolLoop::new(
        "XRP-USDT",
        Config::default_for(&["XRP-USDT"]),
        store.clone(),
        &stream_dir,
        STALENESS_MS,
        0,
    )
    .unwrap();
    assert!(!lp2.is_cold(), "the state file resumes warm");

    let mut restarted_signals = 0;
    let mut restarted_resume = false;
    for (input, now) in &events[consumed..] {
        let out = lp2.process(input.clone(), *now).unwrap();
        restarted_signals += count_signals(&out);
        restarted_resume |= out.iter().any(|e| matches!(e, LiveEvent::Resume));
    }
    assert_eq!(
        restarted_signals, 0,
        "a consumed zone never re-fires after restart"
    );
    assert!(
        restarted_resume,
        "a warm restart injects Resume before the first fed input"
    );

    let _ = std::fs::remove_dir_all(&state_dir);
    let _ = std::fs::remove_dir_all(&stream_dir);
}

#[test]
fn gap_staleness_resume_makes_a_later_in_zone_tick_a_missed_touch() {
    let state_dir = temp_dir("gap_state");
    let stream_dir = temp_dir("gap_stream");
    let candles = load_fixture("tests/fixtures/signal_4h.csv");
    let events = event_stream(&candles);
    let store = Arc::new(StateStore::open(&state_dir, STATE_SCHEMA_VERSION).unwrap());

    let mut lp = SymbolLoop::new(
        "XRP-USDT",
        Config::default_for(&["XRP-USDT"]),
        store,
        &stream_dir,
        STALENESS_MS,
        0,
    )
    .unwrap();

    // Candle 20 closes at open_time 288_000_000 with the demand zone [104.0, 105.0] Active and
    // awaiting its first tick (see `replay_equivalence`). That is 21 candles * 5 events.
    let up_to_breakout = 21 * 5;
    for (input, now) in &events[..up_to_breakout] {
        let out = lp.process(input.clone(), *now).unwrap();
        assert_eq!(count_signals(&out), 0, "no signal before the touch");
    }

    // A price older than the staleness limit is not fed, and flags a gap.
    let stale = lp
        .process(
            LiveInput::Price {
                price: dec("104.5"),
                at_ms: 288_000_050,
            },
            288_000_050 + STALENESS_MS + 1,
        )
        .unwrap();
    assert!(matches!(stale[0], LiveEvent::StalePrice { .. }));

    // The next observed tick is already inside the zone: the injected Resume makes it a missed
    // touch, never a first-touch signal.
    let after = lp
        .process(
            LiveInput::Price {
                price: dec("104.5"),
                at_ms: 288_000_100,
            },
            288_000_101,
        )
        .unwrap();
    assert!(
        after.iter().any(|e| matches!(e, LiveEvent::Resume)),
        "the gap injects Resume"
    );
    assert_eq!(count_signals(&after), 0, "a downtime cross must not signal");
    assert!(
        after.iter().any(|e| matches!(
            e,
            LiveEvent::ZoneEvent(ev)
                if ev.reason
                    == Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch))
        )),
        "the first tick inside the zone after a gap must be a missed_touch"
    );

    let _ = std::fs::remove_dir_all(&state_dir);
    let _ = std::fs::remove_dir_all(&stream_dir);
}
