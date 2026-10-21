mod common;

use analyzer_rs::driver::{CandleDriver, DriverEvent};
use analyzer_rs::strategy::config::Config;

#[test]
fn driver_warms_up_and_emits_only_new_divergences() {
    let f = common::load_fixture("synthetic_1day");
    let candles = f.candles();
    let cfg = Config::default();
    let mut driver = CandleDriver::new("SYNTH-USDT", cfg.clone());
    let mut warmed = false;
    let mut signals = 0usize;

    for (i, candle) in candles.iter().enumerate() {
        let events = driver.push_closed(candle.clone()).unwrap();
        for event in events {
            match event {
                DriverEvent::Warmed { .. } => {
                    assert_eq!(i + 1, cfg.window, "warms exactly when the window fills");
                    warmed = true;
                }
                DriverEvent::Signal { signal, .. } => {
                    assert!(warmed);
                    assert_eq!(signal.ticker, "SYNTH-USDT");
                    signals += 1;
                }
                DriverEvent::Suppressed { .. } => {}
            }
        }
        assert_eq!(driver.window().len(), (i + 1).min(cfg.window));
    }

    assert!(warmed, "driver must warm");
    assert!(
        signals > 0,
        "synthetic fixture must produce at least one new signal"
    );
    assert!(driver.push_closed(candles.last().unwrap().clone()).is_err());
}
