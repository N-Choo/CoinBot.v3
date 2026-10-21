#![cfg(feature = "harness")]

use analyzer_rs::driver::{CandleDriver, DriverEvent, SuppressReason};
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, Dataset};
use analyzer_rs::harness::replay::{build_replay, ReplayEvent};
use analyzer_rs::identity::{config_version, signal_id};
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::Signal;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn load_synthetic() -> Dataset {
    load_csv(
        "SYNTH-USDT",
        &fixture("synthetic_1day.csv"),
        Timeframe::D1,
        i64::MAX,
        0,
        0,
        0,
        0,
    )
    .unwrap()
}

fn suppress_name(reason: SuppressReason) -> &'static str {
    match reason {
        SuppressReason::Duplicate => "duplicate",
        SuppressReason::BelowWatermark => "below_watermark",
        SuppressReason::TooOld => "too_old",
    }
}

#[derive(Debug, PartialEq, Eq)]
enum EventKind {
    Warmup { seeded_keys: usize },
    Signal { signal_index: usize },
    Suppressed { reason: String },
}

type EventSequence = Vec<(usize, EventKind)>;
type SignalSequence = Vec<(usize, Signal)>;

fn driver_sequence(dataset: &Dataset, cfg: &Config) -> (EventSequence, SignalSequence) {
    let mut driver = CandleDriver::new(&dataset.symbol, cfg.clone());
    let mut events = Vec::new();
    let mut signals = Vec::new();
    for (index, candle) in dataset.candles.iter().enumerate() {
        for event in driver.push_closed(candle.clone()).unwrap() {
            match event {
                DriverEvent::Warmed { seeded_keys } => {
                    events.push((index, EventKind::Warmup { seeded_keys }));
                }
                DriverEvent::Signal { signal, .. } => {
                    let signal_index = signals.len();
                    signals.push((index, signal));
                    events.push((index, EventKind::Signal { signal_index }));
                }
                DriverEvent::Suppressed { reason, .. } => {
                    events.push((
                        index,
                        EventKind::Suppressed {
                            reason: suppress_name(reason).to_string(),
                        },
                    ));
                }
            }
        }
    }
    (events, signals)
}

fn replay_event_parts(event: &ReplayEvent) -> (usize, EventKind) {
    match event {
        ReplayEvent::Warmup {
            candle_index,
            seeded_keys,
        } => (
            *candle_index,
            EventKind::Warmup {
                seeded_keys: *seeded_keys,
            },
        ),
        ReplayEvent::Signal {
            candle_index,
            signal_index,
        } => (
            *candle_index,
            EventKind::Signal {
                signal_index: *signal_index,
            },
        ),
        ReplayEvent::Suppressed {
            candle_index,
            reason,
        } => (
            *candle_index,
            EventKind::Suppressed {
                reason: reason.clone(),
            },
        ),
    }
}

#[test]
fn replay_matches_the_driver_and_uses_live_identity() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let replay = build_replay(&dataset, &cfg, None).unwrap();

    let (expected_events, expected_signals) = driver_sequence(&dataset, &cfg);
    let actual_events: Vec<_> = replay.events.iter().map(replay_event_parts).collect();
    let actual_signals: Vec<_> = replay
        .signals
        .iter()
        .map(|signal| (signal.candle_index, signal.signal.clone()))
        .collect();
    assert!(
        !expected_signals.is_empty(),
        "synthetic fixture must emit signals"
    );
    assert_eq!(actual_events, expected_events);
    assert_eq!(actual_signals, expected_signals);

    let version = config_version(&cfg);
    for signal in &replay.signals {
        assert!(signal.signal_id.starts_with("v1:"));
        assert_eq!(
            signal.signal_id,
            signal_id(&signal.signal, &version),
            "identity must match live"
        );
    }

    assert_eq!(
        replay.events.iter().find_map(|event| match event {
            ReplayEvent::Warmup { candle_index, .. } => Some(*candle_index),
            _ => None,
        }),
        Some(cfg.window - 1),
        "warmup fires when the window fills"
    );

    let allowed_reasons = ["duplicate", "below_watermark", "too_old"];
    for event in &replay.events {
        if let ReplayEvent::Suppressed { reason, .. } = event {
            assert!(
                allowed_reasons.contains(&reason.as_str()),
                "unexpected suppressed reason: {reason}"
            );
        }
    }
}

#[test]
fn age_cap_suppresses_old_divergences() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let baseline = build_replay(&dataset, &cfg, None).unwrap();
    let youngest = baseline
        .signals
        .iter()
        .map(|signal| signal.signal.divergence_age_candles)
        .min()
        .expect("fixture must emit at least one signal");
    assert!(
        youngest > 0,
        "fixture must emit a divergence older than zero candles"
    );

    let replay = build_replay(&dataset, &cfg, Some(youngest - 1)).unwrap();
    assert!(
        replay.events.iter().any(|event| matches!(
            event,
            ReplayEvent::Suppressed { reason, .. } if reason == "too_old"
        )),
        "expected a too_old suppression, got events: {:?}",
        replay.events
    );
}
