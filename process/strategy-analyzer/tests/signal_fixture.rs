//! Hand-derived signal-producing fixtures (Task 27). The series are derived from the spec, not
//! tuned to the implementation; see the plan's Step 1 derivation. Gated behind `harness` so the
//! core-only `--no-default-features` test run still compiles.
#![cfg(feature = "harness")]

use rust_decimal::Decimal;
use std::str::FromStr;
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::run::run;
use strategy_analyzer::harness::sim::SimParams;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{
    ConsumeReason, Input, Output, Signal, StrategyState, ZoneEvent, ZoneEventKind, ZoneEventReason,
};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

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
            n += o
                .iter()
                .filter(|o| matches!(o, Output::EntrySignal(_)))
                .count();
        }
        let (ns, _) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap();
        st = ns;
    }
    n
}

/// Replay a fixture and return its first emitted `EntrySignal`.
fn emitted_signal(path: &str, policy: PathPolicy, cfg: &Config) -> Signal {
    let candles = load_csv(path).unwrap();
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    let mut found: Option<Signal> = None;
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let (ns, outs) = step(&st, Input::PriceTick(t), cfg).unwrap();
            st = ns;
            for out in outs {
                if let Output::EntrySignal(sig) = out {
                    found = Some(sig);
                }
            }
        }
        let (ns, _) = step(&st, Input::ClosedCandle(candle), cfg).unwrap();
        st = ns;
    }
    found.expect("fixture must emit a signal")
}

/// Everything a fixture replay tells us about *why* it did (or did not) publish signals:
/// the signal count, the `StateChange` veto tags, the `ZoneEvent`s, and any zone observed
/// consumed by a published signal (`ConsumeReason::SignalPublished`, captured before the
/// next closed candle prunes the terminal zone).
struct FixtureRun {
    signals: usize,
    reasons: Vec<&'static str>,
    zone_events: Vec<ZoneEvent>,
    consumed_signal_zone_ids: Vec<String>,
}

/// Replay a fixture under one path policy, collecting signals, veto reasons, zone events and
/// signal-consumed zone ids. This makes the historically-silent vetoes assertable.
fn replay_fixture(path: &str, policy: PathPolicy) -> FixtureRun {
    let candles = load_csv(path).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    let mut run = FixtureRun {
        signals: 0,
        reasons: Vec::new(),
        zone_events: Vec::new(),
        consumed_signal_zone_ids: Vec::new(),
    };
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let (ns, outs) = step(&st, Input::PriceTick(t), &cfg).unwrap();
            st = ns;
            collect_outs(&mut run, outs);
            collect_consumed_signal_zones(&mut run, &st);
        }
        let (ns, outs) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap();
        st = ns;
        collect_outs(&mut run, outs);
    }
    run
}

fn collect_outs(run: &mut FixtureRun, outs: Vec<Output>) {
    for out in outs {
        match out {
            Output::EntrySignal(_) => run.signals += 1,
            Output::StateChange { reason } => run.reasons.push(reason),
            Output::ZoneEvent(ev) => run.zone_events.push(ev),
        }
    }
}

/// Record zones the signal path consumed (`signal_published`) before pruning drops them.
fn collect_consumed_signal_zones(run: &mut FixtureRun, st: &StrategyState) {
    for z in st
        .zones
        .iter()
        .filter(|z| z.consume_reason == Some(ConsumeReason::SignalPublished))
    {
        if !run.consumed_signal_zone_ids.contains(&z.id) {
            run.consumed_signal_zone_ids.push(z.id.clone());
        }
    }
}

/// Hard-coded field asserts for the happy fixture's emitted signal, taken from the plan's
/// hand-derivation (candle-21 touch at 104.5; stop = zone.low 104.0 − 0.1·ATR19 1.2929 =
/// 103.8707 → 103.9 at 1 dp; risk 0.6; TP = 104.5 + 3·0.6 = 106.3; obstacle = breakout high
/// 109.0 → rr = (109.0 − 104.5)/0.6 = 7.5). These are NOT re-derived from the golden hash.
#[test]
fn happy_fixture_signal_fields_are_hard_coded() {
    let mut cfg = Config::default_for(&["XRP-USDT"]);
    cfg.price_scale.insert("XRP-USDT".to_string(), 1); // 1 dp, per the fixture derivation
    let sig = emitted_signal("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst, &cfg);
    let dec = |s: &str| Decimal::from_str(s).unwrap();
    assert_eq!(sig.trigger_price, dec("104.5"), "trigger");
    assert_eq!(sig.stop_loss, dec("103.9"), "stop");
    assert_eq!(sig.take_profit, dec("106.3"), "take-profit");
    assert_eq!(sig.obstacle_rr, Some(dec("7.5")), "obstacle R:R");
}

#[test]
fn signal_fixture_yields_exactly_one_signal() {
    assert_eq!(
        count_signals("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst),
        1,
        "hand-derived fixture must produce exactly one signal"
    );
}

/// `run` must count `missed_touch` from the **tick** steps (it read candle steps before and was
/// always 0). Re-opening candle 21 inside the zone makes the first observed tick after activation
/// already inside it -> `missed_touch` (§7.2), not a late signal.
#[test]
fn run_counts_missed_touch_from_tick_steps() {
    let mut candles = load_csv("tests/fixtures/signal_4h.csv").unwrap();
    let last = candles.len() - 1;
    candles[last].open = dec("104.5");
    candles[last].low = dec("104.0");
    let cfg = Config::default_for(&["XRP-USDT"]);
    let params = SimParams {
        fee_bps: Decimal::from(5),
        entry_slip_bps: Decimal::from(5),
        stop_slip_bps: Decimal::from(15),
        borrow_bps_per_day: Decimal::from(2),
    };
    let res = run("XRP-USDT", &candles, PathPolicy::LowFirst, &cfg, params).unwrap();
    assert_eq!(
        res.signals, 0,
        "a missed first touch must not publish a signal"
    );
    assert_eq!(res.missed_touch, 1);
    assert_eq!(res.reason_counts.get("missed_touch"), Some(&1));
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

/// The mitigated fixture must *name* its veto instead of returning silently.
#[test]
fn mitigated_fixture_emits_mitigated_reason() {
    let run = replay_fixture(
        "tests/fixtures/signal_mitigated_4h.csv",
        PathPolicy::LowFirst,
    );
    assert_eq!(run.signals, 0, "mitigated zone must publish no signal");
    assert!(
        run.reasons.contains(&"mitigated"),
        "expected a `mitigated` veto reason, got {:?}",
        run.reasons
    );
}

/// The obstacle-too-close fixture vetoes on the first touch and must CONSUME the zone with reason
/// `vetoed_obstacle_too_close` (never leave it `Active`, never emit a `StateChange`): the veto is
/// the zone's one entry attempt, so later/deeper ticks cannot re-trigger it. Exactly one such
/// emission — not two — and still 0 signals.
#[test]
fn obstacle_too_close_fixture_veto_consumes_zone_once() {
    let run = replay_fixture(
        "tests/fixtures/signal_obstacle_too_close_4h.csv",
        PathPolicy::LowFirst,
    );
    assert_eq!(run.signals, 0, "vetoed touch must publish no signal");
    assert!(
        !run.reasons.contains(&"obstacle_too_close"),
        "a first-touch veto must not emit a `StateChange`, got {:?}",
        run.reasons
    );
    let consumed = run
        .zone_events
        .iter()
        .filter(|e| {
            e.kind == ZoneEventKind::Consumed
                && e.reason
                    == Some(ZoneEventReason::Consumed(
                        ConsumeReason::VetoedObstacleTooClose,
                    ))
        })
        .count();
    assert_eq!(
        consumed, 1,
        "the first-touch veto must consume the zone exactly once, got {consumed}"
    );
}

/// The second touch sees a zone already consumed by the first signal: exactly one signal, and the
/// first zone is `Consumed` (either a `Consumed` ZoneEvent or the `signal_published` consume reason).
#[test]
fn second_touch_fixture_consumes_first_zone() {
    let run = replay_fixture(
        "tests/fixtures/signal_second_touch_4h.csv",
        PathPolicy::LowFirst,
    );
    assert_eq!(
        run.signals, 1,
        "second touch must not publish a second signal"
    );
    let consumed = run
        .zone_events
        .iter()
        .any(|e| e.kind == ZoneEventKind::Consumed)
        || !run.consumed_signal_zone_ids.is_empty();
    assert!(
        consumed,
        "the first zone must end `Consumed` (signal_published or a Consumed ZoneEvent)"
    );
}
