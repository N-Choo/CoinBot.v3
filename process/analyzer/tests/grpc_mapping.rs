#![cfg(feature = "grpc")]

use analyzer::exchange::timeframes::Timeframe;
use analyzer::live::grpc::{config_version, to_proto_signal};
use analyzer::strategy::config::Config;
use analyzer::strategy::types::{Direction, Signal};
use rust_decimal::Decimal;
use std::str::FromStr;

fn sample() -> Signal {
    Signal {
        ticker: "BTC-USDT".into(),
        action: Direction::Long,
        confidence: Decimal::from_str("0.8").unwrap(),
        entry_price: Decimal::from_str("100").unwrap(),
        reason: "rsi_momentum".into(),
        sl_price: Some(Decimal::from_str("95").unwrap()),
        tp_price: Some(Decimal::from_str("110").unwrap()),
        signal_candle_open_time_ms: 1_700_001_800_000,
    }
}

#[test]
fn maps_signal_to_proto() {
    let signal = sample();
    let proto = to_proto_signal(
        &signal,
        7,
        1_000_000,
        60_000,
        &config_version(&Config::default()),
        Timeframe::M15,
    )
    .unwrap();

    assert_eq!(proto.ticker, "BTC/USDT");
    assert_eq!(proto.action, "BUY");
    assert_eq!(proto.entry_price, 100.0);
    assert_eq!(proto.sl_price, 95.0);
    assert_eq!(proto.tp_price, 110.0);
    assert_eq!(
        proto.trigger_time_ms,
        signal.signal_candle_open_time_ms + 900_000
    );
    assert_eq!(proto.reason, "rsi_momentum");
    assert_eq!(proto.seq, 7);
    assert_eq!(proto.expires_at_ms, 1_060_000);
    assert_eq!(proto.schema_version, 1);
    assert!(proto.signal_id.starts_with("v2:"));
}

#[test]
fn short_maps_to_sell() {
    let mut signal = sample();
    signal.action = Direction::Short;

    let proto = to_proto_signal(
        &signal,
        1,
        0,
        0,
        &config_version(&Config::default()),
        Timeframe::M15,
    )
    .unwrap();

    assert_eq!(proto.action, "SELL");
}

#[test]
fn absent_levels_map_to_zero() {
    let mut signal = sample();
    signal.sl_price = None;
    signal.tp_price = None;
    let proto = to_proto_signal(
        &signal,
        1,
        0,
        0,
        &config_version(&Config::default()),
        Timeframe::M15,
    )
    .unwrap();
    assert_eq!(proto.sl_price, 0.0);
    assert_eq!(proto.tp_price, 0.0);
}

#[test]
fn ttl_and_trigger_use_the_dequeue_time_and_timeframe() {
    let signal = sample();
    let proto = to_proto_signal(
        &signal,
        1,
        5_000,
        60_000,
        &config_version(&Config::default()),
        Timeframe::H4,
    )
    .unwrap();

    assert_eq!(proto.expires_at_ms, 65_000);
    assert_eq!(
        proto.trigger_time_ms,
        signal.signal_candle_open_time_ms + 14_400_000
    );
}

#[test]
fn signal_id_is_stable_and_depends_on_the_candle() {
    let cv = config_version(&Config::default());
    let first = to_proto_signal(&sample(), 1, 0, 0, &cv, Timeframe::M15).unwrap();
    let second = to_proto_signal(&sample(), 2, 999, 123, &cv, Timeframe::M15).unwrap();
    assert_eq!(first.signal_id, second.signal_id);

    let mut moved = sample();
    moved.signal_candle_open_time_ms += 1;
    let third = to_proto_signal(&moved, 1, 0, 0, &cv, Timeframe::M15).unwrap();
    assert_ne!(first.signal_id, third.signal_id);
}

#[test]
fn signal_id_depends_on_config() {
    let cv = config_version(&Config::default());
    let base = to_proto_signal(&sample(), 1, 0, 0, &cv, Timeframe::M15).unwrap();

    let changed_config = Config {
        rsi_period: 15,
        ..Config::default()
    };
    let by_config = to_proto_signal(
        &sample(),
        1,
        0,
        0,
        &config_version(&changed_config),
        Timeframe::M15,
    )
    .unwrap();
    assert_ne!(base.signal_id, by_config.signal_id);
}

#[test]
fn config_version_covers_every_strategy_field() {
    let base = config_version(&Config::default());
    let variants = [
        Config {
            window: 101,
            ..Config::default()
        },
        Config {
            rsi_period: 15,
            ..Config::default()
        },
        Config {
            rsi_long_threshold: Decimal::from(61),
            ..Config::default()
        },
        Config {
            rsi_short_threshold: Decimal::from(49),
            ..Config::default()
        },
        Config {
            atr_enabled: true,
            ..Config::default()
        },
        Config {
            atr_period: 15,
            ..Config::default()
        },
        Config {
            atr_sl_multiplier: Decimal::new(21, 1),
            ..Config::default()
        },
        Config {
            atr_tp_multiplier: Decimal::new(31, 1),
            ..Config::default()
        },
        Config {
            atr_pct_high: Decimal::new(51, 1),
            ..Config::default()
        },
        Config {
            atr_high_adjust: Decimal::new(16, 1),
            ..Config::default()
        },
        Config {
            atr_sl_max_pct: Decimal::new(1, 1),
            ..Config::default()
        },
    ];

    for variant in variants {
        assert_ne!(base, config_version(&variant));
    }
}
