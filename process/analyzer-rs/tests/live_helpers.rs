#![cfg(feature = "live")]

use analyzer_rs::driver::CandleDriver;
use analyzer_rs::live::config::LiveConfig;
use analyzer_rs::live::sink::Sink;
use analyzer_rs::live::symbol_task::process_batch;
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::Signal;

mod common;

fn collected(sink: &Sink) -> Vec<Signal> {
    match sink {
        Sink::Collect(buffer) => buffer.lock().unwrap().clone(),
        _ => panic!("expected a collect sink"),
    }
}

#[test]
fn live_config_reads_symbols_and_defaults() {
    let cfg = LiveConfig::new(
        vec!["BTC-USDT".into(), "ETH-USDT".into()],
        "15min",
        100,
        5_000,
        1_000,
        30_000,
        "http://127.0.0.1:50051".into(),
        "https://api.kucoin.com".into(),
    )
    .unwrap();
    assert_eq!(cfg.symbols.len(), 2);
    assert_eq!(cfg.timeframe.as_kucoin(), "15min");
    assert_eq!(cfg.window, 100);
    assert_eq!(cfg.settle_ms, 5_000);
    assert_eq!(cfg.poll_ms, 1_000);
    assert_eq!(cfg.activation_poll_ms, 30_000);
    assert_eq!(cfg.endpoint, "http://127.0.0.1:50051");
    assert_eq!(cfg.base_url, "https://api.kucoin.com");
}

#[test]
fn process_batch_emits_signals_through_the_sink() {
    let f = common::load_fixture("synthetic_1day");
    let candles = f.candles();
    let strategy = Config::default();
    let mut driver = CandleDriver::new("SYNTH-USDT", strategy.clone());
    let sink = Sink::Collect(Default::default());

    let emitted = process_batch(&mut driver, &candles, &sink).unwrap();
    assert_eq!(emitted, collected(&sink).len());
    assert!(emitted > 0);
}
