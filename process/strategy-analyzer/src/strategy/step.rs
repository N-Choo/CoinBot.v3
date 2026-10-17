use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::types::*;
use rust_decimal::Decimal;

pub(crate) const TIMEFRAME_MS: i64 = 4 * 60 * 60 * 1000;

/// Pure step. `state` is borrowed; on error the caller's state is untouched
/// (validate-then-commit: a new state is produced only on success).
pub fn step(
    state: &StrategyState,
    input: Input,
    cfg: &Config,
) -> Result<(StrategyState, Vec<Output>), StrategyError> {
    let mut next = state.clone();
    let outputs = match input {
        Input::PriceTick(t) => on_tick(&mut next, t)?,
        Input::ClosedCandle(c) => on_candle(&mut next, c, cfg)?,
    };
    Ok((next, outputs))
}

fn on_tick(state: &mut StrategyState, t: Tick) -> Result<Vec<Output>, StrategyError> {
    if t.price <= Decimal::ZERO {
        return Err(StrategyError::MalformedTick { price: t.price.to_string() });
    }
    let last = state.last_tick_time_ms.or(state.last_candle_time_ms);
    if let Some(prev) = last {
        if t.time_ms < prev {
            return Err(StrategyError::OutOfOrderTick { got_ms: t.time_ms, last_ms: prev });
        }
    }
    state.last_tick_time_ms = Some(t.time_ms);
    state.last_price = Some(t.price);
    Ok(vec![]) // touch detection added in Task 15
}

fn on_candle(
    state: &mut StrategyState,
    c: Candle,
    _cfg: &Config,
) -> Result<Vec<Output>, StrategyError> {
    if c.high < c.low || c.high <= Decimal::ZERO || c.low <= Decimal::ZERO {
        return Err(StrategyError::MalformedCandle { at_ms: c.open_time_ms, why: "bad range/price" });
    }
    if let Some(prev) = state.last_candle_time_ms {
        if c.open_time_ms == prev { return Err(StrategyError::DuplicateCandle(c.open_time_ms)); }
        if c.open_time_ms < prev { return Err(StrategyError::OutOfOrderCandle { got_ms: c.open_time_ms, prev_ms: prev }); }
        if c.open_time_ms != prev + TIMEFRAME_MS {
            return Err(StrategyError::GappedCandle { expected_ms: prev + TIMEFRAME_MS, got_ms: c.open_time_ms });
        }
    }
    state.window.push_back(c);
    if state.window.len() > crate::strategy::types::WINDOW_CANDLES { state.window.pop_front(); }
    state.candle_count += 1;
    state.last_candle_time_ms = Some(c.open_time_ms);
    Ok(vec![]) // structure/ATR wired in Task 16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle(ms: i64) -> Candle {
        Candle { open_time_ms: ms, open: dec("1.0"), high: dec("1.2"), low: dec("0.9"), close: dec("1.1") }
    }

    #[test]
    fn accepts_contiguous_candles() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let (s, _) = step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS)), &cfg).unwrap();
        assert_eq!(s.window.len(), 2);
    }

    #[test]
    fn rejects_duplicate_and_out_of_order() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(step(&s, Input::ClosedCandle(candle(0)), &cfg), Err(StrategyError::DuplicateCandle(_))));
        assert!(matches!(step(&s, Input::ClosedCandle(candle(-5)), &cfg), Err(StrategyError::OutOfOrderCandle { .. })));
    }

    #[test]
    fn rejects_gap() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS * 2)), &cfg), Err(StrategyError::GappedCandle { .. })));
    }

    #[test]
    fn error_leaves_state_unchanged() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let snapshot = s.clone();
        let _ = step(&s, Input::ClosedCandle(candle(0)), &cfg);
        assert_eq!(s, snapshot);
    }

    #[test]
    fn rejects_malformed_and_out_of_order_tick() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 1, price: dec("-1"), is_bar_open: false }), &cfg), Err(StrategyError::MalformedTick { .. })));
        let (s, _) = step(&s, Input::PriceTick(Tick { time_ms: 10, price: dec("1.0"), is_bar_open: true }), &cfg).unwrap();
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 5, price: dec("1.0"), is_bar_open: false }), &cfg), Err(StrategyError::OutOfOrderTick { .. })));
    }
}
