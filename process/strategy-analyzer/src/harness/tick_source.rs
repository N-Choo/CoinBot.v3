use crate::strategy::types::{Candle, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicy { LowFirst, HighFirst }

/// Ordered intrabar path for a candle under a global policy. Deterministic (no RNG).
pub fn synth_ticks(c: &Candle, policy: PathPolicy) -> Vec<Tick> {
    let (a, b) = match policy { PathPolicy::LowFirst => (c.low, c.high), PathPolicy::HighFirst => (c.high, c.low) };
    vec![
        Tick { time_ms: c.open_time_ms, price: c.open, is_bar_open: true },
        Tick { time_ms: c.open_time_ms + 1, price: a, is_bar_open: false },
        Tick { time_ms: c.open_time_ms + 2, price: b, is_bar_open: false },
        Tick { time_ms: c.open_time_ms + 3, price: c.close, is_bar_open: false },
    ]
}

pub trait TickSource {
    fn policy(&self) -> PathPolicy;
    /// Yield (ticks, closed_candle) per candle; the caller delivers ticks first.
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)>;
}

pub struct Ohlcv4hSynthesis { candles: Vec<Candle>, idx: usize, policy: PathPolicy }

impl Ohlcv4hSynthesis {
    pub fn new(candles: Vec<Candle>, policy: PathPolicy) -> Self { Self { candles, idx: 0, policy } }
}

impl TickSource for Ohlcv4hSynthesis {
    fn policy(&self) -> PathPolicy { self.policy }
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)> {
        let c = *self.candles.get(self.idx)?;
        self.idx += 1;
        Some((synth_ticks(&c, self.policy), c))
    }
}

/// Aggregate a contiguous run of 1m candles into one 4H candle.
pub fn aggregate_4h(minutes: &[Candle]) -> Candle {
    let first = &minutes[0];
    let (mut high, mut low) = (first.high, first.low);
    for m in minutes {
        if m.high > high { high = m.high; }
        if m.low < low { low = m.low; }
    }
    Candle { open_time_ms: first.open_time_ms, open: first.open, high, low, close: minutes.last().unwrap().close }
}

/// 1m source (Phase 0): each 1m candle becomes a path; every 240 minutes are emitted as one
/// 4H `ClosedCandle` (ticks first, §4 ordering). Boundaries align to the 4H epoch grid; the
/// caller validates the aggregates against the fetched 4H series before trusting a 1m run.
pub struct OneMinuteExpansion { minutes: Vec<Candle>, idx: usize, policy: PathPolicy }
impl OneMinuteExpansion {
    pub fn new(minutes: Vec<Candle>, policy: PathPolicy) -> Self { Self { minutes, idx: 0, policy } }
}
impl TickSource for OneMinuteExpansion {
    fn policy(&self) -> PathPolicy { self.policy }
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)> {
        let end = (self.idx + 240).min(self.minutes.len());
        if self.idx >= end { return None; }
        let slice = &self.minutes[self.idx..end];
        let ticks: Vec<Tick> = slice.iter().flat_map(|m| synth_ticks(m, self.policy)).collect();
        let agg = aggregate_4h(slice);
        self.idx = end;
        Some((ticks, agg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle() -> Candle {
        Candle { open_time_ms: 0, open: dec("1.0"), high: dec("1.3"), low: dec("0.9"), close: dec("1.1") }
    }

    #[test]
    fn low_first_visits_low_before_high() {
        let t = synth_ticks(&candle(), PathPolicy::LowFirst);
        assert!(t.iter().position(|x| x.price == dec("0.9")).unwrap() < t.iter().position(|x| x.price == dec("1.3")).unwrap());
    }

    #[test]
    fn high_first_visits_high_before_low() {
        let t = synth_ticks(&candle(), PathPolicy::HighFirst);
        assert!(t.iter().position(|x| x.price == dec("1.3")).unwrap() < t.iter().position(|x| x.price == dec("0.9")).unwrap());
    }

    #[test]
    fn source_streams_ticks_before_candle() {
        let mut src = Ohlcv4hSynthesis::new(vec![candle()], PathPolicy::LowFirst);
        let (ticks, c) = src.next_candle().unwrap();
        assert!(!ticks.is_empty());
        assert_eq!(c.high, dec("1.3"));
        assert!(src.next_candle().is_none());
    }
}
