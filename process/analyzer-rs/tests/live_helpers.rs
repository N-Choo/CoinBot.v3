#![cfg(feature = "live")]

use analyzer_rs::driver::CandleDriver;
use analyzer_rs::live::config::{LiveConfig, LiveConfigError};
use analyzer_rs::live::sink::Sink;
use analyzer_rs::live::symbol_task::process_batch;
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::Signal;

mod common;

fn new_config(
    symbols: Vec<&str>,
    window: usize,
    settle_ms: i64,
    poll_ms: u64,
    activation_poll_ms: u64,
) -> Result<LiveConfig, LiveConfigError> {
    LiveConfig::new(
        symbols.into_iter().map(str::to_string).collect(),
        "15min",
        window,
        settle_ms,
        poll_ms,
        activation_poll_ms,
        "http://127.0.0.1:50051".into(),
        "https://api.kucoin.com".into(),
    )
}

fn collected(sink: &Sink) -> Vec<Signal> {
    match sink {
        Sink::Collect(buffer) => buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
        _ => panic!("expected a collect sink"),
    }
}

#[test]
fn live_config_reads_symbols_and_defaults() {
    let cfg = new_config(vec!["BTC-USDT", "ETH-USDT"], 100, 5_000, 1_000, 30_000).unwrap();
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
fn live_config_normalizes_and_dedupes_symbols() {
    let cfg = new_config(
        vec![" BTC/USDT ", "BTC-USDT", "ETH/USDT", "BTC/USDT", ""],
        100,
        5_000,
        1_000,
        30_000,
    )
    .unwrap();
    assert_eq!(
        cfg.symbols,
        vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()]
    );
}

#[test]
fn live_config_rejects_zero_poll_interval() {
    let error = new_config(vec!["BTC/USDT"], 100, 5_000, 0, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::PollIntervalZero));
}

#[test]
fn live_config_rejects_zero_activation_poll_interval() {
    let error = new_config(vec!["BTC/USDT"], 100, 5_000, 1_000, 0).unwrap_err();
    assert!(matches!(error, LiveConfigError::ActivationPollIntervalZero));
}

#[test]
fn live_config_rejects_negative_settle() {
    let error = new_config(vec!["BTC/USDT"], 100, -1, 1_000, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::NegativeSettle));
}

#[test]
fn live_config_rejects_a_window_below_eight() {
    let error = new_config(vec!["BTC/USDT"], 7, 5_000, 1_000, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::WindowTooSmall));
}

#[test]
fn live_config_rejects_a_window_below_the_strategy_minimum() {
    let strategy = Config::default();
    let window = strategy.min_window() - 1;
    let cfg = new_config(vec!["BTC/USDT"], window, 5_000, 1_000, 30_000).unwrap();

    assert!(matches!(
        cfg.validate_strategy(&strategy),
        Err(LiveConfigError::WindowBelowStrategy { window: got, need })
            if got == window && need == strategy.min_window()
    ));
}

#[test]
fn live_config_accepts_a_window_at_the_strategy_minimum() {
    let strategy = Config::default();
    let cfg = new_config(
        vec!["BTC/USDT"],
        strategy.min_window(),
        5_000,
        1_000,
        30_000,
    )
    .unwrap();
    assert!(cfg.validate_strategy(&strategy).is_ok());
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
