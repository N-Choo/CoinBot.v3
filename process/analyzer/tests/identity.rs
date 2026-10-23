use analyzer::identity::{config_version, signal_id};
use analyzer::strategy::config::Config;
use analyzer::strategy::types::{Direction, Signal};
use rust_decimal::Decimal;

fn sample() -> Signal {
    Signal {
        ticker: "BTC-USDT".into(),
        action: Direction::Long,
        confidence: Decimal::ONE,
        entry_price: Decimal::from(100),
        reason: "rsi_long_cross".into(),
        sl_price: None,
        tp_price: None,
        signal_candle_open_time_ms: 1_700_086_400_000,
    }
}

#[test]
fn config_version_changes_with_strategy_fields() {
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
    assert_eq!(base, config_version(&Config::default()));
}

#[test]
fn config_version_normalizes_decimal_scale() {
    use std::str::FromStr;
    let a = Config {
        rsi_long_threshold: Decimal::from(60),
        ..Config::default()
    };
    let b = Config {
        rsi_long_threshold: Decimal::from_str("60.0").unwrap(),
        ..Config::default()
    };
    assert_eq!(config_version(&a), config_version(&b));
}

#[test]
fn signal_id_is_stable_and_action_sensitive() {
    let signal = sample();
    let version = config_version(&Config::default());
    let id = signal_id(&signal, &version);
    assert!(id.starts_with("v2:"));
    assert_eq!(id, signal_id(&signal, &version));

    let mut other = signal.clone();
    other.action = Direction::Short;
    assert_ne!(id, signal_id(&other, &version));
}

#[test]
fn signal_id_changes_with_the_signal_candle() {
    let version = config_version(&Config::default());
    let base = sample();
    let mut moved = base.clone();
    moved.signal_candle_open_time_ms += 1;
    assert_ne!(signal_id(&base, &version), signal_id(&moved, &version));
}
