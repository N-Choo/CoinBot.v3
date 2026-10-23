use crate::strategy::config::Config;
use crate::strategy::decimal::{add, div, mul, sub};
use crate::strategy::error::Result;
use crate::strategy::indicators::atr::atr;
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::types::{Candle, Direction, Signal};
use rust_decimal::Decimal;

pub fn cross_action(prev: Decimal, curr: Decimal, cfg: &Config) -> Option<Direction> {
    if prev <= cfg.rsi_long_threshold && curr > cfg.rsi_long_threshold {
        Some(Direction::Long)
    } else if prev >= cfg.rsi_short_threshold && curr < cfg.rsi_short_threshold {
        Some(Direction::Short)
    } else {
        None
    }
}

pub fn signals(candles: &[Candle], ticker: &str, cfg: &Config) -> Result<Option<Signal>> {
    if candles.len() < 2 {
        return Ok(None);
    }
    if candles.len() < cfg.min_window() {
        return Ok(None);
    }
    let closes: Vec<Option<Decimal>> = candles.iter().map(|c| Some(c.close)).collect();
    let rsi_values = rsi(&closes, cfg.rsi_period)?;
    let last = candles.len() - 1;
    let (Some(prev), Some(curr)) = (rsi_values[last - 1], rsi_values[last]) else {
        return Ok(None);
    };
    let Some(action) = cross_action(prev, curr, cfg) else {
        return Ok(None);
    };
    let (sl_price, tp_price) = if cfg.atr_enabled {
        match levels(candles, cfg, action)? {
            (Some(sl), Some(tp)) if sl > Decimal::ZERO && tp > Decimal::ZERO => {
                (Some(sl), Some(tp))
            }
            _ => return Ok(None),
        }
    } else {
        (None, None)
    };
    Ok(Some(Signal {
        ticker: ticker.to_string(),
        action,
        confidence: Decimal::ONE,
        entry_price: candles[last].close,
        reason: match action {
            Direction::Long => "rsi_long_cross",
            Direction::Short => "rsi_short_cross",
        }
        .to_string(),
        sl_price,
        tp_price,
        signal_candle_open_time_ms: candles[last].open_time_ms,
    }))
}

fn levels(
    candles: &[Candle],
    cfg: &Config,
    action: Direction,
) -> Result<(Option<Decimal>, Option<Decimal>)> {
    let last = candles.len() - 1;
    let close = candles[last].close;
    if candles.len() < cfg.atr_period {
        return Ok((None, None));
    }
    let atr_series = atr(candles, cfg.atr_period)?;
    let atr_value = match atr_series[last] {
        Some(value) => value,
        None => return Ok((None, None)),
    };
    if atr_value <= Decimal::ZERO || close <= Decimal::ZERO {
        return Ok((None, None));
    }

    let atr_pct = mul(div(atr_value, close)?, Decimal::from(100))?;
    let mut sl_mult = cfg.atr_sl_multiplier;
    let mut tp_mult = cfg.atr_tp_multiplier;
    if atr_pct > cfg.atr_pct_high {
        sl_mult = mul(sl_mult, cfg.atr_high_adjust)?;
        tp_mult = mul(tp_mult, cfg.atr_high_adjust)?;
    }
    let sl_distance = mul(atr_value, sl_mult)?;
    let tp_distance = mul(atr_value, tp_mult)?;
    let (mut sl, tp) = match action {
        Direction::Long => (sub(close, sl_distance)?, add(close, tp_distance)?),
        Direction::Short => (add(close, sl_distance)?, sub(close, tp_distance)?),
    };

    if cfg.atr_sl_max_pct > Decimal::ZERO {
        match action {
            Direction::Long => {
                let max_sl = mul(close, sub(Decimal::ONE, cfg.atr_sl_max_pct)?)?;
                if sl < max_sl {
                    sl = max_sl;
                }
            }
            Direction::Short => {
                let max_sl = mul(close, add(Decimal::ONE, cfg.atr_sl_max_pct)?)?;
                if sl > max_sl {
                    sl = max_sl;
                }
            }
        }
    }

    Ok((Some(sl), Some(tp)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Candle;

    fn d(value: i64) -> Decimal {
        Decimal::from(value)
    }

    fn candle(i: usize, open: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            open_time_ms: i as i64 * 86_400_000,
            open: d(open),
            high: d(high),
            low: d(low),
            close: d(close),
            volume: Decimal::ONE,
        }
    }

    #[test]
    fn long_cross_is_strictly_above_the_upper_threshold() {
        let cfg = Config::default();
        assert_eq!(cross_action(d(60), d(61), &cfg), Some(Direction::Long));
        assert_eq!(cross_action(d(60), d(60), &cfg), None);
        assert_eq!(cross_action(d(61), d(62), &cfg), None);
    }

    #[test]
    fn short_cross_is_strictly_below_the_lower_threshold() {
        let cfg = Config::default();
        assert_eq!(cross_action(d(50), d(49), &cfg), Some(Direction::Short));
        assert_eq!(cross_action(d(50), d(50), &cfg), None);
        assert_eq!(cross_action(d(49), d(48), &cfg), None);
    }

    #[test]
    fn neutral_band_holds() {
        let cfg = Config::default();
        assert_eq!(cross_action(d(40), d(55), &cfg), None);
        assert_eq!(cross_action(d(70), d(55), &cfg), None);
    }

    #[test]
    fn flat_history_has_no_signal() {
        let candles: Vec<Candle> = (0..40).map(|i| candle(i, 100, 100, 100, 100)).collect();
        assert_eq!(
            signals(&candles, "TEST-USDT", &Config::default()).unwrap(),
            None
        );
    }

    #[test]
    fn an_up_bar_after_flat_history_crosses_above_sixty() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 100, 100, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));
        let signal = signals(&candles, "TEST-USDT", &Config::default())
            .unwrap()
            .expect("flat-to-up must produce a long cross");
        assert_eq!(signal.action, Direction::Long);
        assert_eq!(signal.reason, "rsi_long_cross");
        assert_eq!(signal.entry_price, d(101));
        assert_eq!(signal.sl_price, None);
        assert_eq!(signal.tp_price, None);
        assert_eq!(signal.signal_candle_open_time_ms, 30 * 86_400_000);
    }

    #[test]
    fn atr_levels_are_present_only_when_enabled() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));

        let cfg = Config {
            atr_enabled: true,
            ..Config::default()
        };
        let signal = signals(&candles, "TEST-USDT", &cfg).unwrap().unwrap();
        assert_eq!(signal.sl_price, Some(Decimal::from(97)));
        assert_eq!(signal.tp_price, Some(Decimal::from(107)));
    }

    #[test]
    fn atr_disabled_emits_no_levels() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));
        let signal = signals(&candles, "TEST-USDT", &Config::default())
            .unwrap()
            .unwrap();
        assert_eq!(signal.sl_price, None);
        assert_eq!(signal.tp_price, None);
    }

    #[test]
    fn short_cross_emits_short_signal_with_mirrored_atr_levels() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 100, 100, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));
        for (offset, close) in (81..=100).rev().enumerate() {
            let i = 31 + offset;
            candles.push(candle(i, close, close + 1, close - 1, close));
        }
        let cfg = Config {
            atr_enabled: true,
            ..Config::default()
        };
        let signal = (31..candles.len())
            .find_map(|end| {
                signals(&candles[..=end], "TEST-USDT", &cfg)
                    .unwrap()
                    .filter(|s| s.action == Direction::Short)
            })
            .expect("falling sequence must produce a short cross");
        assert_eq!(signal.reason, "rsi_short_cross");
        let (Some(sl), Some(tp)) = (signal.sl_price, signal.tp_price) else {
            panic!("atr levels must be present for a short signal");
        };
        assert!(
            sl > signal.entry_price,
            "short sl {sl} must sit above entry {}",
            signal.entry_price
        );
        assert!(
            tp < signal.entry_price,
            "short tp {tp} must sit below entry {}",
            signal.entry_price
        );
    }

    #[test]
    fn atr_high_volatility_adjusts_the_levels() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));
        let cfg = Config {
            atr_enabled: true,
            atr_pct_high: Decimal::from(1),
            atr_high_adjust: Decimal::new(15, 1),
            ..Config::default()
        };
        let signal = signals(&candles, "TEST-USDT", &cfg).unwrap().unwrap();
        assert_eq!(signal.sl_price, Some(Decimal::from(95)));
        assert_eq!(signal.tp_price, Some(Decimal::from(110)));
    }

    #[test]
    fn atr_sl_cap_clamps_long_and_short() {
        let candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        let cfg = Config {
            atr_enabled: true,
            atr_sl_max_pct: Decimal::new(1, 2),
            ..Config::default()
        };
        let (long_sl, _) = levels(&candles, &cfg, Direction::Long).unwrap();
        assert_eq!(long_sl, Some(Decimal::from(99)));
        let (short_sl, _) = levels(&candles, &cfg, Direction::Short).unwrap();
        assert_eq!(short_sl, Some(Decimal::from(101)));
    }

    #[test]
    fn atr_enabled_with_insufficient_history_returns_no_signal() {
        let candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        let cfg = Config {
            atr_enabled: true,
            ..Config::default()
        };
        assert_eq!(signals(&candles[..10], "TEST-USDT", &cfg).unwrap(), None);
    }
}
