// Fixture fields are staged for the Task 13 signal parity test.
#![allow(dead_code)]

use analyzer_rs::strategy::config::{decimal_from_str, Config};
use analyzer_rs::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub symbol: String,
    pub window: usize,
    pub config: ConfigFixture,
    pub candles: Vec<CandleFixture>,
    pub indicators: Indicators,
    pub divergences: Divergences,
    pub signals: Vec<ExpectedSignal>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFixture {
    pub rsi_period: usize,
    pub rsi_order: usize,
    pub price_diff_threshold: String,
    pub sma_enabled: bool,
    pub sma_period: usize,
    pub atr_period: usize,
    pub atr_sl_multiplier: String,
    pub atr_tp_multiplier: String,
    pub atr_pct_high: String,
    pub atr_high_adjust: String,
    pub atr_sl_max_pct: String,
    pub window: usize,
}

#[derive(Debug, Deserialize)]
pub struct CandleFixture {
    pub open_time_ms: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Debug, Deserialize)]
pub struct Indicators {
    pub rsi: Vec<Option<String>>,
    pub sma: Vec<Option<String>>,
    pub atr: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Divergences {
    pub regular_bullish: Vec<i64>,
    pub regular_bearish: Vec<i64>,
    pub hidden_bullish: Vec<i64>,
    pub hidden_bearish: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedSignal {
    pub window_end: usize,
    pub signal: Option<SignalFixture>,
}

#[derive(Debug, Deserialize)]
pub struct SignalFixture {
    pub action: String,
    pub reason: String,
    pub kind: String,
    pub confidence: String,
    pub entry: String,
    pub sl: String,
    pub tp: String,
    pub divergence_index: usize,
    pub divergence_open_time_ms: i64,
    pub divergence_age_candles: usize,
    pub signal_candle_open_time_ms: i64,
}

impl Fixture {
    pub fn candles(&self) -> Vec<Candle> {
        self.candles
            .iter()
            .map(|c| Candle {
                open_time_ms: c.open_time_ms,
                open: decimal_from_str(&c.open).unwrap(),
                high: decimal_from_str(&c.high).unwrap(),
                low: decimal_from_str(&c.low).unwrap(),
                close: decimal_from_str(&c.close).unwrap(),
                volume: decimal_from_str(&c.volume).unwrap(),
            })
            .collect()
    }

    pub fn config(&self) -> Config {
        Config {
            rsi_period: self.config.rsi_period,
            rsi_order: self.config.rsi_order,
            price_diff_threshold: decimal_from_str(&self.config.price_diff_threshold).unwrap(),
            sma_enabled: self.config.sma_enabled,
            sma_period: self.config.sma_period,
            atr_period: self.config.atr_period,
            atr_sl_multiplier: decimal_from_str(&self.config.atr_sl_multiplier).unwrap(),
            atr_tp_multiplier: decimal_from_str(&self.config.atr_tp_multiplier).unwrap(),
            atr_pct_high: decimal_from_str(&self.config.atr_pct_high).unwrap(),
            atr_high_adjust: decimal_from_str(&self.config.atr_high_adjust).unwrap(),
            atr_sl_max_pct: decimal_from_str(&self.config.atr_sl_max_pct).unwrap(),
            window: self.config.window,
        }
    }
}

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/parity")
        .join(format!("{name}.json"))
}

pub fn load_fixture(name: &str) -> Fixture {
    let path = fixture_path(name);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Compare decimals with an exact-match fast path, then a relative tolerance:
/// `|a - b| <= max(|a|, |b|, 1) * tolerance`.
pub fn close_enough(actual: Decimal, expected: Decimal, tolerance: f64) -> bool {
    if actual == expected {
        return true;
    }
    let diff = (actual - expected).abs();
    let scale = actual.abs().max(expected.abs()).max(Decimal::ONE);
    let tolerance = decimal_from_str(&format!("{tolerance:e}")).unwrap();
    diff <= scale * tolerance
}

pub fn assert_close(actual: Decimal, expected: &str, tolerance: f64) {
    let expected = decimal_from_str(expected).unwrap();
    assert!(
        close_enough(actual, expected, tolerance),
        "got {actual}, want {expected}"
    );
}

pub fn assert_opt_close_at(
    actual: Option<Decimal>,
    expected: Option<&str>,
    tolerance: f64,
    label: &str,
) {
    match (actual, expected) {
        (None, None) => {}
        (Some(a), Some(e)) => {
            let expected = decimal_from_str(e).unwrap();
            assert!(
                close_enough(a, expected, tolerance),
                "{label}: got {a}, want {expected}"
            );
        }
        (a, e) => panic!("{label}: got {a:?}, want {e:?}"),
    }
}

pub fn assert_opt_close(actual: Option<Decimal>, expected: Option<&str>, tolerance: f64) {
    assert_opt_close_at(actual, expected, tolerance, "");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_enough_uses_exact_then_relative_tolerance() {
        assert!(close_enough(Decimal::ZERO, Decimal::ZERO, 1e-9));
        assert!(close_enough(Decimal::new(1, 9), Decimal::ZERO, 1e-9));
        assert!(close_enough(
            Decimal::ONE,
            decimal_from_str("1.000000001").unwrap(),
            1e-9
        ));
        assert_opt_close_at(Some(Decimal::ONE), Some("1"), 1e-9, "x");
        assert!(close_enough(
            decimal_from_str("1.0000000001").unwrap(),
            Decimal::ONE,
            1e-10
        ));
    }
}
