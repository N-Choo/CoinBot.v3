#![cfg(feature = "grpc")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::live::grpc::{config_version, to_proto_signal};
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::{Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;
use std::str::FromStr;

fn sample() -> Signal {
    Signal {
        ticker: "BTC-USDT".into(),
        action: Direction::Long,
        confidence: Decimal::from_str("0.8").unwrap(),
        entry_price: Decimal::from_str("100").unwrap(),
        reason: "regular_bullish_divergence".into(),
        sl_price: Decimal::from_str("95").unwrap(),
        tp_price: Decimal::from_str("110").unwrap(),
        divergence_index: 10,
        divergence_kind: DivergenceKind::RegularBullish,
        divergence_open_time_ms: 1_700_000_000_000,
        divergence_age_candles: 3,
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
    assert_eq!(proto.reason, "regular_bullish_divergence");
    assert_eq!(proto.seq, 7);
    assert_eq!(proto.expires_at_ms, 1_060_000);
    assert_eq!(proto.schema_version, 1);
    assert!(proto.signal_id.starts_with("v1:"));
}

#[test]
fn signal_id_is_stable_and_depends_on_the_divergence() {
    let config_version = config_version(&Config::default());
    let first = to_proto_signal(&sample(), 1, 0, 0, &config_version, Timeframe::M15).unwrap();
    let second = to_proto_signal(&sample(), 2, 999, 123, &config_version, Timeframe::M15).unwrap();
    assert_eq!(first.signal_id, second.signal_id);

    let mut moved = sample();
    moved.divergence_open_time_ms += 1;
    let third = to_proto_signal(&moved, 1, 0, 0, &config_version, Timeframe::M15).unwrap();
    assert_ne!(first.signal_id, third.signal_id);
}
