use crate::strategy::config::Config;
use crate::strategy::error::Result;
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
        signal_candle_open_time_ms: candles[last].open_time_ms,
    }))
}

#[derive(Debug, Clone, Default)]
pub struct SignalGenerator {
    last_accepted_side: Option<Direction>,
}

impl SignalGenerator {
    pub fn next(
        &mut self,
        candles: &[Candle],
        ticker: &str,
        cfg: &Config,
    ) -> Result<Option<Signal>> {
        let Some(signal) = signals(candles, ticker, cfg)? else {
            return Ok(None);
        };
        if !self.accept(signal.action) {
            return Ok(None);
        }
        Ok(Some(signal))
    }

    fn accept(&mut self, action: Direction) -> bool {
        if self.last_accepted_side == Some(action) {
            false
        } else {
            self.last_accepted_side = Some(action);
            true
        }
    }
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
        assert_eq!(signal.signal_candle_open_time_ms, 30 * 86_400_000);
    }

    #[test]
    fn short_cross_emits_short_signal() {
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 100, 100, 100)).collect();
        candles.push(candle(30, 100, 102, 100, 101));
        for (offset, close) in (81..=100).rev().enumerate() {
            let i = 31 + offset;
            candles.push(candle(i, close, close + 1, close - 1, close));
        }
        let signal = (31..candles.len())
            .find_map(|end| {
                signals(&candles[..=end], "TEST-USDT", &Config::default())
                    .unwrap()
                    .filter(|s| s.action == Direction::Short)
            })
            .expect("falling sequence must produce a short cross");
        assert_eq!(signal.reason, "rsi_short_cross");
    }

    #[test]
    fn signal_generator_accepts_only_alternating_sides() {
        let cfg = Config::default();
        let mut generator = SignalGenerator::default();

        let mut long_candles: Vec<Candle> =
            (0..30).map(|i| candle(i, 100, 100, 100, 100)).collect();
        long_candles.push(candle(30, 100, 102, 100, 101));
        let long = signals(&long_candles, "TEST-USDT", &cfg)
            .unwrap()
            .expect("flat-to-up must produce a long cross");
        assert_eq!(long.action, Direction::Long);

        assert_eq!(
            generator.next(&long_candles, "TEST-USDT", &cfg).unwrap(),
            Some(long)
        );
        assert_eq!(
            generator.next(&long_candles, "TEST-USDT", &cfg).unwrap(),
            None,
            "a repeated long must be rejected"
        );
        assert_eq!(
            generator.next(&long_candles, "TEST-USDT", &cfg).unwrap(),
            None,
            "a third long must be rejected"
        );

        let mut short_candles = long_candles.clone();
        for (offset, close) in (81..=100).rev().enumerate() {
            let i = 31 + offset;
            short_candles.push(candle(i, close, close + 1, close - 1, close));
        }
        let short_end = (31..short_candles.len())
            .find(|&end| {
                signals(&short_candles[..=end], "TEST-USDT", &cfg)
                    .unwrap()
                    .is_some_and(|s| s.action == Direction::Short)
            })
            .expect("falling sequence must produce a short cross");

        let short = generator
            .next(&short_candles[..=short_end], "TEST-USDT", &cfg)
            .unwrap()
            .expect("an opposite-side signal must be accepted");
        assert_eq!(short.action, Direction::Short);
    }

    #[test]
    fn insufficient_history_returns_no_signal() {
        let candles: Vec<Candle> = (0..30).map(|i| candle(i, 100, 101, 99, 100)).collect();
        assert_eq!(
            signals(&candles[..10], "TEST-USDT", &Config::default()).unwrap(),
            None
        );
    }
}
