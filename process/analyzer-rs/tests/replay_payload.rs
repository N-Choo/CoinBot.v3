#![cfg(feature = "harness")]

use analyzer_rs::driver::CandleDriver;
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::replay::{build_replay, ReplayEvent};
use analyzer_rs::identity::{config_version, signal_id};
use analyzer_rs::strategy::config::Config;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn replay_matches_the_driver_and_uses_live_identity() {
    let cfg = Config::default();
    let dataset = load_csv(
        "SYNTH-USDT",
        &fixture("synthetic_1day.csv"),
        Timeframe::D1,
        i64::MAX,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    let replay = build_replay(&dataset, &cfg).unwrap();

    let mut driver = CandleDriver::new("SYNTH-USDT", cfg.clone());
    let mut expected_signals = 0usize;
    for candle in &dataset.candles {
        for event in driver.push_closed(candle.clone()).unwrap() {
            if matches!(event, analyzer_rs::driver::DriverEvent::Signal { .. }) {
                expected_signals += 1;
            }
        }
    }
    let actual_signals = replay
        .signals
        .iter()
        .filter(|signal| signal.candle_index >= cfg.window)
        .count();
    assert_eq!(actual_signals, expected_signals);
    assert!(actual_signals > 0);

    let version = config_version(&cfg);
    for signal in &replay.signals {
        assert!(signal.signal_id.starts_with("v1:"));
        assert_eq!(
            signal.signal_id,
            signal_id(&signal.signal, &version),
            "identity must match live"
        );
    }

    assert!(replay
        .events
        .iter()
        .any(|event| matches!(event, ReplayEvent::Warmup { .. })));

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
