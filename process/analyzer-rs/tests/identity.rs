use analyzer_rs::identity::{config_version, signal_id};
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::{Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;

fn sample() -> Signal {
    Signal {
        ticker: "BTC-USDT".into(),
        action: Direction::Long,
        confidence: Decimal::new(8, 1),
        entry_price: Decimal::from(100),
        reason: "regular_bullish_divergence".into(),
        sl_price: Decimal::from(98),
        tp_price: Decimal::from(104),
        divergence_index: 12,
        divergence_kind: DivergenceKind::RegularBullish,
        divergence_open_time_ms: 1_700_000_000_000,
        divergence_age_candles: 5,
        signal_candle_open_time_ms: 1_700_086_400_000,
    }
}

#[test]
fn config_version_changes_with_strategy_fields() {
    let base = config_version(&Config::default());
    let changed = Config {
        atr_sl_multiplier: Decimal::new(25, 1),
        ..Config::default()
    };
    assert_ne!(base, config_version(&changed));
    assert_eq!(base, config_version(&Config::default()));
}

#[test]
fn signal_id_is_stable_and_kind_sensitive() {
    let signal = sample();
    let version = config_version(&Config::default());
    let id = signal_id(&signal, &version);
    assert!(id.starts_with("v1:"));
    assert_eq!(id, signal_id(&signal, &version));

    let mut other = signal.clone();
    other.divergence_kind = DivergenceKind::HiddenBullish;
    assert_ne!(id, signal_id(&other, &version));
}
