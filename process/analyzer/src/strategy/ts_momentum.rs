use crate::strategy::decimal::{add, div, mul, sub};
use crate::strategy::error::{Error, Result};
use crate::strategy::types::Candle;
use rust_decimal::{Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Stable strategy identifier used in signals and dedup keys.
pub const STRATEGY_ID: &str = "btc-ts-momentum-v1";

/// The strategy consumes daily candles. Missing or irregular spacing is rejected
/// rather than interpolated.
pub const DAILY_INTERVAL_MS: i64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NegativeMomentumMode {
    Flat,
    Short,
}

impl NegativeMomentumMode {
    pub fn as_str(self) -> &'static str {
        match self {
            NegativeMomentumMode::Flat => "Flat",
            NegativeMomentumMode::Short => "Short",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TsDirection {
    Long,
    Short,
    Flat,
}

impl TsDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            TsDirection::Long => "Long",
            TsDirection::Short => "Short",
            TsDirection::Flat => "Flat",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TsMomentumConfig {
    pub momentum_lookback_days: usize,
    pub volatility_lookback_days: usize,
    pub target_annualized_volatility: Decimal,
    pub annualization_days: usize,
    pub max_exposure: Decimal,
    pub min_exposure: Decimal,
    pub negative_momentum_mode: NegativeMomentumMode,
}

impl Default for TsMomentumConfig {
    fn default() -> Self {
        TsMomentumConfig {
            momentum_lookback_days: 30,
            volatility_lookback_days: 30,
            target_annualized_volatility: Decimal::new(20, 2),
            annualization_days: 365,
            max_exposure: Decimal::ONE,
            min_exposure: Decimal::ZERO,
            negative_momentum_mode: NegativeMomentumMode::Flat,
        }
    }
}

impl TsMomentumConfig {
    /// Momentum needs `lookback + 1` closes; volatility needs `lookback + 1`
    /// closes for `lookback` returns. The larger requirement wins.
    pub fn required_candles(&self) -> usize {
        self.momentum_lookback_days
            .max(self.volatility_lookback_days)
            .saturating_add(1)
    }

    /// Stable representation of the parameters that affect a signal. Included
    /// in the dedup key so a parameter change cannot reuse a previous signal id.
    pub fn config_version(&self) -> String {
        format!(
            "mom={};vol={};target={};ann={};min={};max={};neg={}",
            self.momentum_lookback_days,
            self.volatility_lookback_days,
            self.target_annualized_volatility.normalize(),
            self.annualization_days,
            self.min_exposure.normalize(),
            self.max_exposure.normalize(),
            self.negative_momentum_mode.as_str(),
        )
    }

    pub fn validate(&self) -> Result<()> {
        let fail = |message: &str| Err(Error::ConfigInvalid(message.to_string()));
        if self.momentum_lookback_days == 0 {
            return fail("momentum_lookback_days must be >= 1");
        }
        if self.volatility_lookback_days == 0 {
            return fail("volatility_lookback_days must be >= 1");
        }
        if self.target_annualized_volatility <= Decimal::ZERO {
            return fail("target_annualized_volatility must be finite and > 0");
        }
        if self.annualization_days == 0 {
            return fail("annualization_days must be >= 1");
        }
        if self.min_exposure < Decimal::ZERO {
            return fail("min_exposure must be finite and >= 0");
        }
        if self.max_exposure < self.min_exposure {
            return fail("max_exposure must be >= min_exposure");
        }
        if self.max_exposure > Decimal::ONE {
            return fail("max_exposure must be <= 1.0");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TsMomentumSignal {
    pub strategy_id: String,
    pub symbol: String,
    pub signal_candle_open_time_ms: i64,
    pub direction: TsDirection,
    pub momentum: Decimal,
    pub annualized_volatility: Decimal,
    pub target_annualized_volatility: Decimal,
    pub desired_exposure: Decimal,
    pub dedup_key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsSignalResult {
    Signal(TsMomentumSignal),
    NoSignal { reason: String },
}

/// Evaluate the last closed candle in `candles`.
///
/// `candles` must contain only closed daily candles. In-progress candles are
/// excluded by the data/live layer before this call; this core does not attempt
/// to infer completeness from timestamps.
pub fn evaluate(
    candles: &[Candle],
    symbol: &str,
    cfg: &TsMomentumConfig,
) -> Result<TsSignalResult> {
    cfg.validate()?;

    let required = cfg.required_candles();
    if candles.len() < required {
        return Ok(TsSignalResult::NoSignal {
            reason: format!(
                "insufficient_history: need {required} closed candles, got {}",
                candles.len()
            ),
        });
    }

    if let Some(reason) = validate_candles(candles)? {
        return Ok(TsSignalResult::NoSignal { reason });
    }

    let last = &candles[candles.len() - 1];
    let momentum_index = candles.len() - 1 - cfg.momentum_lookback_days;
    let momentum = sub(
        div(last.close, candles[momentum_index].close)?,
        Decimal::ONE,
    )?;

    let volatility_start = candles.len() - 1 - cfg.volatility_lookback_days;
    let Some(annualized_volatility) = annualized_volatility(
        &candles[volatility_start..],
        cfg.volatility_lookback_days,
        cfg.annualization_days,
    )?
    else {
        return Ok(TsSignalResult::NoSignal {
            reason: "volatility_unavailable: zero, negative, or non-finite".to_string(),
        });
    };

    let direction = if momentum > Decimal::ZERO {
        TsDirection::Long
    } else {
        match cfg.negative_momentum_mode {
            NegativeMomentumMode::Flat => TsDirection::Flat,
            NegativeMomentumMode::Short => TsDirection::Short,
        }
    };

    let desired_exposure = if direction == TsDirection::Flat {
        Decimal::ZERO
    } else {
        let raw_exposure = div(cfg.target_annualized_volatility, annualized_volatility)?;
        raw_exposure.max(cfg.min_exposure).min(cfg.max_exposure)
    };

    let reason = match direction {
        TsDirection::Long => "momentum_positive",
        TsDirection::Short => "momentum_non_positive_short",
        TsDirection::Flat => "momentum_non_positive_flat",
    };

    Ok(TsSignalResult::Signal(TsMomentumSignal {
        strategy_id: STRATEGY_ID.to_string(),
        symbol: symbol.to_string(),
        signal_candle_open_time_ms: last.open_time_ms,
        direction,
        momentum,
        annualized_volatility,
        target_annualized_volatility: cfg.target_annualized_volatility,
        desired_exposure,
        dedup_key: dedup_key(symbol, last.open_time_ms, direction, &cfg.config_version()),
        reason: reason.to_string(),
    }))
}

/// Structural validation. Duplicates and out-of-order timestamps are errors;
/// invalid prices and gaps are explicit no-signal outcomes.
fn validate_candles(candles: &[Candle]) -> Result<Option<String>> {
    for candle in candles {
        if candle.close <= Decimal::ZERO {
            return Ok(Some(format!(
                "invalid_close at {}: close must be positive and finite",
                candle.open_time_ms
            )));
        }
    }

    for pair in candles.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if current.open_time_ms == previous.open_time_ms {
            return Err(Error::DuplicateCandle(current.open_time_ms));
        }
        if current.open_time_ms < previous.open_time_ms {
            return Err(Error::OutOfOrderCandle {
                got_ms: current.open_time_ms,
                last_ms: previous.open_time_ms,
            });
        }
        let spacing = current.open_time_ms - previous.open_time_ms;
        if spacing != DAILY_INTERVAL_MS {
            let missing = spacing / DAILY_INTERVAL_MS - 1;
            return Ok(Some(format!(
                "missing_candle_interval at {}: expected {DAILY_INTERVAL_MS} ms after {}, got {spacing} ms ({missing} missing)",
                current.open_time_ms, previous.open_time_ms
            )));
        }
    }

    Ok(None)
}

/// Annualized population volatility of the log returns in `candles`.
///
/// `candles` holds `lookback + 1` closes. `Decimal` logarithms and square roots
/// keep the deterministic core free of floating-point arithmetic.
fn annualized_volatility(
    candles: &[Candle],
    lookback: usize,
    annualization_days: usize,
) -> Result<Option<Decimal>> {
    debug_assert_eq!(candles.len(), lookback + 1);

    let mut log_returns = Vec::with_capacity(lookback);
    for pair in candles.windows(2) {
        let ratio = div(pair[1].close, pair[0].close)?;
        if ratio <= Decimal::ZERO {
            return Ok(None);
        }
        let log_return = ratio.ln();
        log_returns.push(log_return);
    }

    let divisor = Decimal::from(lookback);
    let sum = log_returns
        .iter()
        .try_fold(Decimal::ZERO, |total, value| add(total, *value))?;
    let mean = div(sum, divisor)?;
    let squared_deviations = log_returns.iter().try_fold(Decimal::ZERO, |total, value| {
        let deviation = sub(*value, mean)?;
        add(total, mul(deviation, deviation)?)
    })?;
    let variance = div(squared_deviations, divisor)?;
    if variance <= Decimal::ZERO {
        return Ok(None);
    }

    let Some(daily_volatility) = variance.sqrt() else {
        return Ok(None);
    };
    if daily_volatility <= Decimal::ZERO {
        return Ok(None);
    }
    let Some(annualization_factor) = Decimal::from(annualization_days).sqrt() else {
        return Ok(None);
    };
    if annualization_factor <= Decimal::ZERO {
        return Ok(None);
    }

    let annualized = mul(daily_volatility, annualization_factor)?;
    if annualized <= Decimal::ZERO {
        return Ok(None);
    }
    Ok(Some(annualized))
}

/// Stable, opaque id over every field that can change signal identity.
fn dedup_key(
    symbol: &str,
    signal_candle_open_time_ms: i64,
    direction: TsDirection,
    config_version: &str,
) -> String {
    const SEPARATOR: u8 = 0;
    let mut hasher = Sha256::new();
    hasher.update(b"analyzer-ts-momentum-signal-v1");
    hasher.update([SEPARATOR]);
    hasher.update(STRATEGY_ID.as_bytes());
    hasher.update([SEPARATOR]);
    hasher.update(symbol.as_bytes());
    hasher.update([SEPARATOR]);
    hasher.update(signal_candle_open_time_ms.to_be_bytes());
    hasher.update([SEPARATOR]);
    hasher.update([match direction {
        TsDirection::Long => b'L',
        TsDirection::Short => b'S',
        TsDirection::Flat => b'F',
    }]);
    hasher.update([SEPARATOR]);
    hasher.update(config_version.as_bytes());

    let digest = hasher.finalize();
    let mut key = String::from("tsm-v1:");
    for byte in digest {
        key.push_str(&format!("{byte:02x}"));
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::error::Error;
    use crate::strategy::types::Candle;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    const DAY_MS: i64 = 86_400_000;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    fn assert_close(actual: Decimal, expected: Decimal, tolerance: Decimal) {
        let difference = (actual - expected).abs();
        assert!(
            difference <= tolerance,
            "expected {expected}, got {actual} (difference {difference})"
        );
    }

    fn candle(index: i64, close: Decimal) -> Candle {
        Candle {
            open_time_ms: index * DAY_MS,
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::ONE,
        }
    }

    /// Thirty log returns alternating around `mean_log_return`. The population
    /// standard deviation is exactly `daily_vol`, so the annualized volatility is
    /// `daily_vol * sqrt(365)`.
    fn candles_with_vol(mean_log_return: Decimal, annualized_vol: Decimal) -> Vec<Candle> {
        let daily_vol = annualized_vol / Decimal::from(365).sqrt().expect("365 has a square root");
        let mut price = Decimal::from(100);
        let mut closes = vec![price];
        for index in 0..30 {
            let log_return = if index % 2 == 0 {
                mean_log_return + daily_vol
            } else {
                mean_log_return - daily_vol
            };
            price *= log_return.exp();
            closes.push(price);
        }
        closes
            .into_iter()
            .enumerate()
            .map(|(index, close)| candle(index as i64, close))
            .collect()
    }

    /// Thirty returns alternating strictly above and below zero, with the last
    /// close exactly equal to the first so momentum is exactly zero.
    fn zero_momentum_candles(annualized_vol: Decimal) -> Vec<Candle> {
        let daily_vol = annualized_vol / Decimal::from(365).sqrt().expect("365 has a square root");
        let up = Decimal::from(100) * daily_vol.exp();
        (0..=30)
            .map(|index| {
                let close = if index % 2 == 0 {
                    Decimal::from(100)
                } else {
                    up
                };
                candle(index, close)
            })
            .collect()
    }

    fn expect_signal(result: TsSignalResult) -> TsMomentumSignal {
        match result {
            TsSignalResult::Signal(signal) => signal,
            other => panic!("expected a signal, got {other:?}"),
        }
    }

    fn expect_no_signal(result: TsSignalResult) -> String {
        match result {
            TsSignalResult::NoSignal { reason } => reason,
            TsSignalResult::Signal(signal) => panic!("expected no signal, got {signal:?}"),
        }
    }

    #[test]
    fn defaults_match_the_specification() {
        let cfg = TsMomentumConfig::default();
        assert_eq!(cfg.momentum_lookback_days, 30);
        assert_eq!(cfg.volatility_lookback_days, 30);
        assert_eq!(cfg.target_annualized_volatility, d("0.20"));
        assert_eq!(cfg.annualization_days, 365);
        assert_eq!(cfg.max_exposure, Decimal::ONE);
        assert_eq!(cfg.min_exposure, Decimal::ZERO);
        assert_eq!(cfg.negative_momentum_mode, NegativeMomentumMode::Flat);
        assert_eq!(cfg.required_candles(), 31);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn config_serde_roundtrips_and_uses_named_modes() {
        let cfg = TsMomentumConfig {
            negative_momentum_mode: NegativeMomentumMode::Short,
            ..TsMomentumConfig::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"negative_momentum_mode\":\"Short\""));
        let decoded: TsMomentumConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cfg);
    }

    #[test]
    fn config_validation_rejects_invalid_values() {
        let cases = [
            (
                TsMomentumConfig {
                    momentum_lookback_days: 0,
                    ..TsMomentumConfig::default()
                },
                "momentum_lookback_days",
            ),
            (
                TsMomentumConfig {
                    volatility_lookback_days: 0,
                    ..TsMomentumConfig::default()
                },
                "volatility_lookback_days",
            ),
            (
                TsMomentumConfig {
                    target_annualized_volatility: Decimal::ZERO,
                    ..TsMomentumConfig::default()
                },
                "target_annualized_volatility",
            ),
            (
                TsMomentumConfig {
                    target_annualized_volatility: d("-0.1"),
                    ..TsMomentumConfig::default()
                },
                "target_annualized_volatility",
            ),
            (
                TsMomentumConfig {
                    annualization_days: 0,
                    ..TsMomentumConfig::default()
                },
                "annualization_days",
            ),
            (
                TsMomentumConfig {
                    max_exposure: d("0.4"),
                    min_exposure: d("0.5"),
                    ..TsMomentumConfig::default()
                },
                "max_exposure",
            ),
            (
                TsMomentumConfig {
                    min_exposure: d("-0.1"),
                    ..TsMomentumConfig::default()
                },
                "min_exposure",
            ),
            (
                TsMomentumConfig {
                    max_exposure: d("1.1"),
                    ..TsMomentumConfig::default()
                },
                "max_exposure",
            ),
        ];
        for (cfg, field) in cases {
            assert!(
                matches!(cfg.validate(), Err(Error::ConfigInvalid(message)) if message.contains(field)),
                "expected ConfigInvalid mentioning {field}, got {:?}",
                cfg.validate()
            );
        }
    }

    #[test]
    fn positive_momentum_is_long() {
        let candles = candles_with_vol(d("0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();
        let signal = expect_signal(evaluate(&candles, "BTC-USDT", &cfg).unwrap());

        assert_eq!(signal.strategy_id, STRATEGY_ID);
        assert_eq!(signal.symbol, "BTC-USDT");
        assert_eq!(signal.signal_candle_open_time_ms, 30 * DAY_MS);
        assert_eq!(signal.direction, TsDirection::Long);
        assert!(signal.momentum > Decimal::ZERO);
        assert!(signal.reason.contains("positive"));
        assert_close(signal.annualized_volatility, d("0.40"), d("0.000000001"));
        assert_eq!(
            signal.target_annualized_volatility,
            cfg.target_annualized_volatility
        );
        assert_close(signal.desired_exposure, d("0.50"), d("0.000000001"));
    }

    #[test]
    fn negative_momentum_defaults_to_flat() {
        let candles = candles_with_vol(d("-0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();
        let signal = expect_signal(evaluate(&candles, "BTC-USDT", &cfg).unwrap());

        assert_eq!(signal.direction, TsDirection::Flat);
        assert!(signal.momentum < Decimal::ZERO);
        assert_eq!(signal.desired_exposure, Decimal::ZERO);
        assert!(signal.reason.contains("flat"));
    }

    #[test]
    fn negative_momentum_can_short_when_enabled() {
        let candles = candles_with_vol(d("-0.001"), d("0.4"));
        let cfg = TsMomentumConfig {
            negative_momentum_mode: NegativeMomentumMode::Short,
            ..TsMomentumConfig::default()
        };
        let signal = expect_signal(evaluate(&candles, "BTC-USDT", &cfg).unwrap());

        assert_eq!(signal.direction, TsDirection::Short);
        assert!(signal.momentum < Decimal::ZERO);
        assert_close(signal.desired_exposure, d("0.50"), d("0.000000001"));
        assert!(signal.reason.contains("short"));
    }

    #[test]
    fn zero_momentum_follows_non_positive_mode() {
        let candles = zero_momentum_candles(d("0.4"));
        let flat =
            expect_signal(evaluate(&candles, "BTC-USDT", &TsMomentumConfig::default()).unwrap());
        assert_eq!(flat.momentum, Decimal::ZERO);
        assert_eq!(flat.direction, TsDirection::Flat);
        assert_eq!(flat.desired_exposure, Decimal::ZERO);

        let short_cfg = TsMomentumConfig {
            negative_momentum_mode: NegativeMomentumMode::Short,
            ..TsMomentumConfig::default()
        };
        let short = expect_signal(evaluate(&candles, "BTC-USDT", &short_cfg).unwrap());
        assert_eq!(short.momentum, Decimal::ZERO);
        assert_eq!(short.direction, TsDirection::Short);
        assert_close(short.desired_exposure, d("0.50"), d("0.000000001"));
    }

    #[test]
    fn volatility_40_percent_targets_50_percent_exposure() {
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("0.001"), d("0.4")),
                "BTC-USDT",
                &TsMomentumConfig::default(),
            )
            .unwrap(),
        );
        assert_close(signal.annualized_volatility, d("0.40"), d("0.000000001"));
        assert_close(signal.desired_exposure, d("0.50"), d("0.000001"));
    }

    #[test]
    fn volatility_60_percent_targets_33_33_percent_exposure() {
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("0.001"), d("0.6")),
                "BTC-USDT",
                &TsMomentumConfig::default(),
            )
            .unwrap(),
        );
        assert_close(signal.annualized_volatility, d("0.60"), d("0.000000001"));
        assert_close(signal.desired_exposure, d("0.333333"), d("0.000001"));
    }

    #[test]
    fn volatility_90_percent_targets_22_22_percent_exposure() {
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("0.001"), d("0.9")),
                "BTC-USDT",
                &TsMomentumConfig::default(),
            )
            .unwrap(),
        );
        assert_close(signal.annualized_volatility, d("0.90"), d("0.000000001"));
        assert_close(signal.desired_exposure, d("0.222222"), d("0.000001"));
    }

    #[test]
    fn volatility_120_percent_targets_16_67_percent_exposure() {
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("0.001"), d("1.2")),
                "BTC-USDT",
                &TsMomentumConfig::default(),
            )
            .unwrap(),
        );
        assert_close(signal.annualized_volatility, d("1.20"), d("0.000000001"));
        assert_close(signal.desired_exposure, d("0.166667"), d("0.000001"));
    }

    #[test]
    fn exposure_respects_min_and_max() {
        let capped = TsMomentumConfig {
            max_exposure: d("0.30"),
            ..TsMomentumConfig::default()
        };
        let signal = expect_signal(
            evaluate(&candles_with_vol(d("0.001"), d("0.4")), "BTC-USDT", &capped).unwrap(),
        );
        assert_eq!(signal.desired_exposure, d("0.30"));

        let floored = TsMomentumConfig {
            min_exposure: d("0.25"),
            ..TsMomentumConfig::default()
        };
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("0.001"), d("1.2")),
                "BTC-USDT",
                &floored,
            )
            .unwrap(),
        );
        assert_eq!(signal.desired_exposure, d("0.25"));
    }

    #[test]
    fn flat_direction_has_zero_exposure() {
        let signal = expect_signal(
            evaluate(
                &candles_with_vol(d("-0.001"), d("0.4")),
                "BTC-USDT",
                &TsMomentumConfig::default(),
            )
            .unwrap(),
        );
        assert_eq!(signal.direction, TsDirection::Flat);
        assert_eq!(signal.desired_exposure, Decimal::ZERO);
        assert!(signal.annualized_volatility > Decimal::ZERO);
    }

    #[test]
    fn zero_or_invalid_volatility_yields_no_trade() {
        let flat = (0..=30)
            .map(|index| candle(index, Decimal::from(100)))
            .collect::<Vec<_>>();
        let reason =
            expect_no_signal(evaluate(&flat, "BTC-USDT", &TsMomentumConfig::default()).unwrap());
        assert!(reason.contains("volatility"), "{reason}");

        let mut invalid = candles_with_vol(d("0.001"), d("0.4"));
        invalid[10].close = Decimal::ZERO;
        let reason =
            expect_no_signal(evaluate(&invalid, "BTC-USDT", &TsMomentumConfig::default()).unwrap());
        assert!(reason.contains("invalid_close"), "{reason}");
    }

    #[test]
    fn insufficient_history_is_explicitly_unavailable() {
        let candles = candles_with_vol(d("0.001"), d("0.4"))
            .into_iter()
            .take(30)
            .collect::<Vec<_>>();
        let reason =
            expect_no_signal(evaluate(&candles, "BTC-USDT", &TsMomentumConfig::default()).unwrap());
        assert!(reason.contains("insufficient_history"), "{reason}");
        assert!(reason.contains("31"), "{reason}");
    }

    #[test]
    fn incomplete_trailing_candle_is_ignored() {
        // `evaluate` is the closed-candle core. The feed may hold an in-progress
        // candle, but callers pass the closed prefix; that prefix must produce the
        // same signal as if the in-progress candle never existed.
        let closed = candles_with_vol(d("0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();
        let expected = evaluate(&closed, "BTC-USDT", &cfg).unwrap();

        let mut raw_feed = closed.clone();
        raw_feed.push(candle(31, d("1")));
        let actual = evaluate(&raw_feed[..closed.len()], "BTC-USDT", &cfg).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn missing_intervals_and_duplicates_are_rejected() {
        let candles = candles_with_vol(d("0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();

        let mut duplicate = candles.clone();
        duplicate[5].open_time_ms = duplicate[4].open_time_ms;
        assert_eq!(
            evaluate(&duplicate, "BTC-USDT", &cfg),
            Err(Error::DuplicateCandle(4 * DAY_MS))
        );

        let mut out_of_order = candles.clone();
        out_of_order[5].open_time_ms = 3 * DAY_MS;
        assert_eq!(
            evaluate(&out_of_order, "BTC-USDT", &cfg),
            Err(Error::OutOfOrderCandle {
                got_ms: 3 * DAY_MS,
                last_ms: 4 * DAY_MS,
            })
        );

        let mut missing = candles;
        missing.push(candle(31, d("1")));
        missing.remove(10);
        let reason = expect_no_signal(evaluate(&missing, "BTC-USDT", &cfg).unwrap());
        assert!(reason.contains("missing_candle_interval"), "{reason}");
    }

    #[test]
    fn repeated_evaluation_is_deterministic_with_a_stable_dedup_key() {
        let candles = candles_with_vol(d("0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();
        let first = evaluate(&candles, "BTC-USDT", &cfg).unwrap();
        let second = evaluate(&candles, "BTC-USDT", &cfg).unwrap();
        assert_eq!(first, second);

        let first = expect_signal(first);
        assert!(!first.dedup_key.is_empty());
        assert_eq!(first.dedup_key, expect_signal(second).dedup_key);

        let changed = TsMomentumConfig {
            target_annualized_volatility: d("0.25"),
            ..cfg
        };
        let changed = expect_signal(evaluate(&candles, "BTC-USDT", &changed).unwrap());
        assert_ne!(first.dedup_key, changed.dedup_key);
    }

    #[test]
    fn prefix_evaluations_do_not_depend_on_future_candles() {
        let mut candles = candles_with_vol(d("0.001"), d("0.4"));
        let cfg = TsMomentumConfig::default();
        let at_last_closed = expect_signal(evaluate(&candles, "BTC-USDT", &cfg).unwrap());
        assert_eq!(at_last_closed.signal_candle_open_time_ms, 30 * DAY_MS);

        for index in 31..=40 {
            candles.push(candle(index, d("1")));
        }
        let same_prefix = expect_signal(evaluate(&candles[..31], "BTC-USDT", &cfg).unwrap());
        assert_eq!(same_prefix, at_last_closed);

        let with_future = expect_signal(evaluate(&candles, "BTC-USDT", &cfg).unwrap());
        assert_eq!(with_future.signal_candle_open_time_ms, 40 * DAY_MS);
    }
}
