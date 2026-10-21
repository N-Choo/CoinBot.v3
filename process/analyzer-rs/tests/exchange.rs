#![cfg(feature = "exchange")]

use analyzer_rs::exchange::kucoin::parse_klines;
use analyzer_rs::exchange::timeframes::Timeframe;

#[test]
fn parses_and_sorts_ku_coin_rows() {
    let raw = r#"{"code":"200000","data":[
        ["1700003600","101","103","104","100","12","1200"],
        ["1700000000","100","102","103","99","10","1000"]
    ]}"#;
    let candles = parse_klines(raw).unwrap();
    assert_eq!(candles.len(), 2);
    assert_eq!(candles[0].open_time_ms, 1_700_000_000_000);
    assert_eq!(candles[1].open_time_ms, 1_700_003_600_000);
    assert_eq!(candles[0].close.to_string(), "102");
}

#[test]
fn closed_candle_rule_excludes_the_forming_candle() {
    let tf = Timeframe::H4;
    let open = 1_700_000_000_000;
    assert!(!tf.is_closed(open, open + tf.millis(), 0));
    assert!(tf.is_closed(open, open + tf.millis() + 1, 0));
    assert!(!tf.is_closed(open, open + tf.millis() + 2_000, 5_000));
}

#[test]
fn timeframes_parse_ku_coin_names() {
    assert_eq!(Timeframe::from_kucoin("15min"), Some(Timeframe::M15));
    assert_eq!(Timeframe::from_kucoin("4hour"), Some(Timeframe::H4));
    assert_eq!(Timeframe::from_kucoin("1day"), Some(Timeframe::D1));
    assert_eq!(Timeframe::from_kucoin("nope"), None);
}
