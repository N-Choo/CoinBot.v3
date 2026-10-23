use analyzer::driver::{CandleDriver, DriverEvent};
use analyzer::strategy::config::Config;
use analyzer::strategy::types::Candle;
use rust_decimal::Decimal;

fn candle(i: usize, close: i64) -> Candle {
    Candle {
        open_time_ms: i as i64 * 86_400_000,
        open: Decimal::from(close),
        high: Decimal::from(close + 1),
        low: Decimal::from(close - 1),
        close: Decimal::from(close),
        volume: Decimal::ONE,
    }
}

#[test]
fn driver_warms_silently_and_emits_a_crossing() {
    let cfg = Config {
        window: 30,
        ..Config::default()
    };
    let mut driver = CandleDriver::new("TEST-USDT", cfg.clone());

    for i in 0..cfg.window {
        let events = driver.push_closed(candle(i, 100)).unwrap();
        if i + 1 == cfg.window {
            assert_eq!(events, vec![DriverEvent::Warmed]);
        } else {
            assert!(events.is_empty());
        }
    }

    let events = driver.push_closed(candle(cfg.window, 101)).unwrap();
    assert!(matches!(events.as_slice(), [DriverEvent::Signal { .. }]));

    let next = driver.push_closed(candle(cfg.window + 1, 102)).unwrap();
    assert!(
        next.is_empty(),
        "no signal while RSI stays above the threshold"
    );
}
