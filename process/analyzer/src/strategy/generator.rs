use crate::strategy::config::Config;
use crate::strategy::decimal::{add, div, mul, neg, sub};
use crate::strategy::divergence::detect;
use crate::strategy::error::Result;
use crate::strategy::indicators::sma::sma;
use crate::strategy::types::{Candle, Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;
use std::cmp::{min, Reverse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub index: usize,
    pub kind: DivergenceKind,
    pub confidence: Decimal,
}

pub fn price_diff(closes: &[Decimal]) -> Result<Decimal> {
    if closes.len() < 7 {
        return Ok(Decimal::ZERO);
    }
    let last = closes[closes.len() - 1];
    let past = closes[closes.len() - 7];
    if past == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }
    div(sub(last, past)?, past)
}

pub fn candidates(candles: &[Candle], cfg: &Config) -> Result<Vec<Candidate>> {
    let divs = detect(candles, cfg)?;
    let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
    let diff = price_diff(&closes)?;
    let latest_rsi = divs.rsi.last().copied().flatten();

    let mut regular_bullish = None;
    let mut regular_bearish = None;
    let mut hidden_bullish = None;
    let mut hidden_bearish = None;

    for idx in (0..divs.regular_bullish.len()).rev() {
        if regular_bullish.is_none() && divs.regular_bullish[idx] {
            regular_bullish = Some(idx);
        }
        if regular_bearish.is_none() && divs.regular_bearish[idx] {
            regular_bearish = Some(idx);
        }
        if hidden_bullish.is_none() && divs.hidden_bullish[idx] {
            hidden_bullish = Some(idx);
        }
        if hidden_bearish.is_none() && divs.hidden_bearish[idx] {
            hidden_bearish = Some(idx);
        }
        if regular_bullish.is_some()
            && regular_bearish.is_some()
            && hidden_bullish.is_some()
            && hidden_bearish.is_some()
        {
            break;
        }
    }

    let mut out = Vec::new();
    let tenth = Decimal::new(1, 1);
    let threshold = cfg.price_diff_threshold;

    if let Some(index) = regular_bullish {
        let mut confidence = if latest_rsi.is_some_and(|r| r < Decimal::from(30)) {
            Decimal::new(8, 1)
        } else {
            Decimal::new(6, 1)
        };
        if diff < neg(threshold)? {
            confidence = min(add(confidence, tenth)?, Decimal::ONE);
        }
        out.push(Candidate {
            index,
            kind: DivergenceKind::RegularBullish,
            confidence,
        });
    }
    if let Some(index) = regular_bearish {
        let mut confidence = if latest_rsi.is_some_and(|r| r > Decimal::from(70)) {
            Decimal::new(8, 1)
        } else {
            Decimal::new(6, 1)
        };
        if diff > threshold {
            confidence = min(add(confidence, tenth)?, Decimal::ONE);
        }
        out.push(Candidate {
            index,
            kind: DivergenceKind::RegularBearish,
            confidence,
        });
    }
    if let Some(index) = hidden_bullish {
        let mut confidence = Decimal::new(5, 1);
        if diff < neg(threshold)? {
            confidence = min(add(confidence, tenth)?, Decimal::ONE);
        }
        out.push(Candidate {
            index,
            kind: DivergenceKind::HiddenBullish,
            confidence,
        });
    }
    if let Some(index) = hidden_bearish {
        let mut confidence = Decimal::new(5, 1);
        if diff > threshold {
            confidence = min(add(confidence, tenth)?, Decimal::ONE);
        }
        out.push(Candidate {
            index,
            kind: DivergenceKind::HiddenBearish,
            confidence,
        });
    }

    Ok(out)
}

pub fn signals(candles: &[Candle], ticker: &str, cfg: &Config) -> Result<Option<Signal>> {
    if candles.is_empty() {
        return Ok(None);
    }
    let mut candidates = candidates(candles, cfg)?;
    if candidates.is_empty() {
        return Ok(None);
    }

    let last = candles.len() - 1;
    let latest_close = candles[last].close;

    if cfg.sma_enabled && candles.len() >= cfg.sma_period {
        let closes: Vec<Option<Decimal>> = candles.iter().map(|c| Some(c.close)).collect();
        let sma_series = sma(&closes, cfg.sma_period)?;
        let sma_value = sma_series[last];
        candidates.retain(|candidate| match (candidate.kind.action(), sma_value) {
            (Direction::Long, Some(value)) => latest_close > value,
            (Direction::Short, Some(value)) => latest_close < value,
            _ => false,
        });
        if candidates.is_empty() {
            return Ok(None);
        }
    }

    candidates.sort_by_key(|candidate| Reverse(candidate.index));
    let best = &candidates[0];
    let (sl_price, tp_price) = levels(candles, cfg, best.kind.action())?;
    let divergence_open_time_ms = candles[best.index].open_time_ms;
    let signal_candle_open_time_ms = candles[last].open_time_ms;

    Ok(Some(Signal {
        ticker: ticker.to_string(),
        action: best.kind.action(),
        confidence: best.confidence,
        entry_price: latest_close,
        reason: best.kind.reason().to_string(),
        sl_price,
        tp_price,
        divergence_index: best.index,
        divergence_kind: best.kind,
        divergence_open_time_ms,
        divergence_age_candles: last - best.index,
        signal_candle_open_time_ms,
    }))
}

fn levels(candles: &[Candle], cfg: &Config, action: Direction) -> Result<(Decimal, Decimal)> {
    use crate::strategy::indicators::atr::atr;

    let last = candles.len() - 1;
    let close = candles[last].close;
    if candles.len() < cfg.atr_period {
        return Ok((Decimal::ZERO, Decimal::ZERO));
    }
    let atr_series = atr(candles, cfg.atr_period)?;
    let atr_value = match atr_series[last] {
        Some(value) => value,
        None => return Ok((Decimal::ZERO, Decimal::ZERO)),
    };
    if atr_value <= Decimal::ZERO || close <= Decimal::ZERO {
        return Ok((Decimal::ZERO, Decimal::ZERO));
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

    Ok((sl, tp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Candle;
    use rust_decimal::Decimal;

    fn flat_candles(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                open_time_ms: i as i64 * 86_400_000,
                open: Decimal::from(100),
                high: Decimal::from(100),
                low: Decimal::from(100),
                close: Decimal::from(100),
                volume: Decimal::ONE,
            })
            .collect()
    }

    #[test]
    fn price_diff_is_zero_below_seven_candles() {
        let closes = vec![Decimal::from(1); 6];
        assert_eq!(price_diff(&closes).unwrap(), Decimal::ZERO);
    }

    #[test]
    fn price_diff_uses_the_seventh_previous_close() {
        let mut closes = vec![Decimal::from(100); 6];
        closes.push(Decimal::from(110));
        assert_eq!(price_diff(&closes).unwrap(), Decimal::new(1, 1));
    }

    #[test]
    fn no_divergence_means_no_signal() {
        let candles = flat_candles(40);
        assert_eq!(
            signals(&candles, "TEST-USDT", &Config::default()).unwrap(),
            None
        );
    }

    fn candles_with_flat_history(n: usize, close: &str) -> Vec<Candle> {
        let value = crate::strategy::config::decimal_from_str(close).unwrap();
        (0..n)
            .map(|i| Candle {
                open_time_ms: i as i64 * 86_400_000,
                open: value,
                high: crate::strategy::decimal::add(value, Decimal::ONE).unwrap(),
                low: crate::strategy::decimal::sub(value, Decimal::ONE).unwrap(),
                close: value,
                volume: Decimal::ONE,
            })
            .collect()
    }

    #[test]
    fn long_levels_use_atr_multiples() {
        let candles = candles_with_flat_history(20, "100");
        let (sl, tp) = levels(&candles, &Config::default(), Direction::Long).unwrap();
        assert_eq!(sl, Decimal::from(96));
        assert_eq!(tp, Decimal::from(106));
    }

    #[test]
    fn short_levels_are_mirrored() {
        let candles = candles_with_flat_history(20, "100");
        let (sl, tp) = levels(&candles, &Config::default(), Direction::Short).unwrap();
        assert_eq!(sl, Decimal::from(104));
        assert_eq!(tp, Decimal::from(94));
    }
}
