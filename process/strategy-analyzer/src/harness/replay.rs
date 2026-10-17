use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::step::step;
use crate::strategy::types::{Input, Output, StrategyState};

pub fn replay(mut state: StrategyState, events: &[Input], cfg: &Config)
    -> Result<(StrategyState, Vec<Output>), StrategyError>
{
    let mut all = Vec::new();
    for ev in events {
        let (ns, mut outs) = step(&state, ev.clone(), cfg)?;
        state = ns;
        all.append(&mut outs);
    }
    Ok((state, all))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::{Candle, Tick};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    fn events() -> Vec<Input> {
        let mut e = Vec::new();
        for i in 0..20i64 {
            let c = Candle { open_time_ms: i * 4*3600*1000, open: dec("1.0"), high: dec("1.1"), low: dec("0.9"), close: dec("1.0") };
            e.push(Input::PriceTick(Tick { time_ms: c.open_time_ms, price: c.open, is_bar_open: true }));
            e.push(Input::ClosedCandle(c));
        }
        e
    }

    #[test]
    fn deterministic_same_outputs() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let (_, a) = replay(StrategyState::new("XRP-USDT", 1), &events(), &cfg).unwrap();
        let (_, b) = replay(StrategyState::new("XRP-USDT", 1), &events(), &cfg).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn snapshot_resume_matches_uninterrupted() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let ev = events();
        let (full, _) = replay(StrategyState::new("XRP-USDT", 1), &ev, &cfg).unwrap();
        let (mid, _) = replay(StrategyState::new("XRP-USDT", 1), &ev[..15], &cfg).unwrap();
        let json = serde_json::to_string(&mid).unwrap();
        let resumed: StrategyState = serde_json::from_str(&json).unwrap();
        let (tail, _) = replay(resumed, &ev[15..], &cfg).unwrap();
        assert_eq!(full, tail);
    }
}
