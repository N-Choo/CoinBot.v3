#![cfg(feature = "harness")]

use analyzer::driver::{CandleDriver, DriverEvent};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::replay::{build_replay, Replay, ReplayEvent};
use analyzer::harness::sim::{simulate, SimSignal};
use analyzer::identity::{config_version, signal_id};
use analyzer::strategy::config::Config;
use analyzer::strategy::indicators::rsi::rsi;
use analyzer::strategy::types::Signal;
use rust_decimal::Decimal;

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

#[derive(Debug, PartialEq, Eq)]
enum EventKind {
    Warmup,
    Signal { signal_index: usize },
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
                DriverEvent::Warmed => events.push((index, EventKind::Warmup)),
                DriverEvent::Signal { signal } => {
                    let signal_index = signals.len();
                    signals.push((index, signal));
                    events.push((index, EventKind::Signal { signal_index }));
                }
            }
        }
    }

    (events, signals)
}

fn replay_event_sequence(replay: &Replay) -> EventSequence {
    replay
        .events
        .iter()
        .map(|event| match event {
            ReplayEvent::Warmup { candle_index } => (*candle_index, EventKind::Warmup),
            ReplayEvent::Signal {
                candle_index,
                signal_index,
            } => (
                *candle_index,
                EventKind::Signal {
                    signal_index: *signal_index,
                },
            ),
        })
        .collect()
}

#[test]
fn replay_matches_the_driver_and_uses_live_identity() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let replay = build_replay(&dataset, &cfg).unwrap();

    let (expected_events, expected_signals) = driver_sequence(&dataset, &cfg);
    let actual_events = replay_event_sequence(&replay);
    let actual_signals: Vec<_> = replay
        .signals
        .iter()
        .map(|s| (s.candle_index, s.signal.clone()))
        .collect();

    assert!(
        !expected_signals.is_empty(),
        "synthetic fixture must emit signals"
    );
    assert_eq!(actual_events, expected_events);
    assert_eq!(actual_signals, expected_signals);

    for event in &replay.events {
        if let ReplayEvent::Signal {
            candle_index,
            signal_index,
        } = event
        {
            let signal = replay
                .signals
                .get(*signal_index)
                .unwrap_or_else(|| panic!("signal index {signal_index} is out of range"));
            assert_eq!(
                signal.candle_index, *candle_index,
                "signal index {signal_index} must point at candle {candle_index}"
            );
        }
    }

    let version = config_version(&cfg);
    for signal in &replay.signals {
        assert!(signal.signal_id.starts_with("v2:"));
        assert_eq!(signal.signal_id, signal_id(&signal.signal, &version));
    }

    assert_eq!(
        replay.events.iter().find_map(|event| match event {
            ReplayEvent::Warmup { candle_index } => Some(*candle_index),
            _ => None,
        }),
        Some(cfg.window - 1),
        "warmup fires when the window fills"
    );
}

#[test]
fn replay_rsi_matches_the_rolling_window() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let replay = build_replay(&dataset, &cfg).unwrap();
    let closes: Vec<Option<rust_decimal::Decimal>> =
        dataset.candles.iter().map(|c| Some(c.close)).collect();
    assert!(replay.rsi[..cfg.window - 1].iter().all(Option::is_none));
    for i in (cfg.window - 1)..closes.len() {
        let expected = rsi(&closes[i + 1 - cfg.window..=i], cfg.rsi_period)
            .unwrap()
            .last()
            .copied()
            .flatten();
        assert_eq!(replay.rsi[i], expected, "rsi at {i}");
    }
}

#[test]
fn signals_fill_on_the_next_candle_and_trades_carry_pnl() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let replay = build_replay(&dataset, &cfg).unwrap();
    let signals: Vec<SimSignal> = replay
        .signals
        .iter()
        .map(|signal| SimSignal {
            candle_index: signal.candle_index,
            signal_id: signal.signal_id.clone(),
            signal: signal.signal.clone(),
        })
        .collect();
    let sim = simulate(&dataset.candles, &signals, 1000, cfg.window);

    assert!(
        !replay.signals.is_empty(),
        "synthetic fixture must emit signals"
    );
    assert!(!sim.trades.is_empty(), "signals must produce closed trades");

    for trade in &sim.trades {
        let signal = replay
            .signals
            .iter()
            .find(|signal| signal.signal_id == trade.signal_id)
            .unwrap_or_else(|| {
                panic!(
                    "trade signal {} has no matching replay signal",
                    trade.signal_id
                )
            });

        // A signal is emitted while processing the rolling candle, then filled
        // at the next candle's open.
        assert_eq!(
            trade.entry_index,
            signal.candle_index + 1,
            "signal on candle {} must fill on the next candle",
            signal.candle_index
        );
        assert_eq!(trade.fill, dataset.candles[trade.entry_index].open);

        assert_eq!(trade.pnl_pct, trade.pnl / trade.e * Decimal::from(100));
    }
}

#[test]
fn build_replay_rejects_a_short_dataset() {
    let cfg = Config::default();
    let dataset = load_synthetic();
    let short = Dataset {
        symbol: dataset.symbol.clone(),
        candles: dataset.candles[..cfg.window + 1].to_vec(),
        provenance: dataset.provenance.clone(),
    };
    assert!(build_replay(&short, &cfg).is_err());
}

#[test]
fn build_replay_rejects_an_invalid_config() {
    let dataset = load_synthetic();
    let cfg = Config {
        window: 7,
        ..Config::default()
    };
    assert!(build_replay(&dataset, &cfg).is_err());
}
