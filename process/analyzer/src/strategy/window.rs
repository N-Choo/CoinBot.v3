use crate::strategy::error::{Error, Result};
use crate::strategy::types::Candle;

#[derive(Debug, Clone)]
pub struct Window {
    candles: Vec<Candle>,
    capacity: usize,
}

impl Window {
    pub fn new(capacity: usize) -> Self {
        Window {
            candles: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_closed(&mut self, candle: Candle) -> Result<()> {
        if self.capacity == 0 {
            return Err(Error::InvalidWindowCapacity);
        }
        if let Some(last) = self.candles.last() {
            if candle.open_time_ms == last.open_time_ms {
                return Err(Error::DuplicateCandle(candle.open_time_ms));
            }
            if candle.open_time_ms < last.open_time_ms {
                return Err(Error::OutOfOrderCandle {
                    got_ms: candle.open_time_ms,
                    last_ms: last.open_time_ms,
                });
            }
        }
        if self.candles.len() == self.capacity {
            self.candles.remove(0);
        }
        self.candles.push(candle);
        Ok(())
    }

    pub fn as_slice(&self) -> &[Candle] {
        &self.candles
    }

    pub fn len(&self) -> usize {
        self.candles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.candles.len() == self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn candle(ms: i64) -> Candle {
        Candle {
            open_time_ms: ms,
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
            volume: Decimal::ONE,
        }
    }

    #[test]
    fn evicts_oldest_at_capacity() {
        let mut w = Window::new(2);
        w.push_closed(candle(1)).unwrap();
        w.push_closed(candle(2)).unwrap();
        w.push_closed(candle(3)).unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w.as_slice()[0].open_time_ms, 2);
        assert_eq!(w.as_slice()[1].open_time_ms, 3);
    }

    #[test]
    fn rejects_duplicates_and_out_of_order() {
        let mut w = Window::new(4);
        w.push_closed(candle(10)).unwrap();
        assert_eq!(w.push_closed(candle(10)), Err(Error::DuplicateCandle(10)));
        assert_eq!(
            w.push_closed(candle(9)),
            Err(Error::OutOfOrderCandle {
                got_ms: 9,
                last_ms: 10
            })
        );
    }

    #[test]
    fn reports_full() {
        let mut w = Window::new(2);
        assert!(!w.is_full());
        w.push_closed(candle(1)).unwrap();
        assert!(!w.is_full());
        w.push_closed(candle(2)).unwrap();
        assert!(w.is_full());
    }

    #[test]
    fn zero_capacity_push_is_a_typed_error() {
        let mut w = Window::new(0);
        assert_eq!(w.push_closed(candle(1)), Err(Error::InvalidWindowCapacity));
        assert!(w.as_slice().is_empty());
    }

    #[test]
    fn capacity_one_replaces_the_single_slot() {
        let mut w = Window::new(1);
        w.push_closed(candle(1)).unwrap();
        w.push_closed(candle(2)).unwrap();
        assert_eq!(w.len(), 1);
        assert_eq!(w.as_slice()[0].open_time_ms, 2);
        assert_eq!(w.push_closed(candle(2)), Err(Error::DuplicateCandle(2)));
        assert_eq!(
            w.push_closed(candle(1)),
            Err(Error::OutOfOrderCandle {
                got_ms: 1,
                last_ms: 2
            })
        );
    }

    #[test]
    fn rejected_push_leaves_state_unchanged() {
        let mut w = Window::new(4);
        w.push_closed(candle(10)).unwrap();
        assert_eq!(w.push_closed(candle(10)), Err(Error::DuplicateCandle(10)));
        assert_eq!(
            w.push_closed(candle(9)),
            Err(Error::OutOfOrderCandle {
                got_ms: 9,
                last_ms: 10
            })
        );
        assert_eq!(w.len(), 1);
        assert_eq!(w.as_slice()[0].open_time_ms, 10);
        w.push_closed(candle(11)).unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w.as_slice()[1].open_time_ms, 11);
    }

    #[test]
    fn gaps_are_allowed() {
        let mut w = Window::new(4);
        w.push_closed(candle(10)).unwrap();
        w.push_closed(candle(20)).unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w.as_slice()[0].open_time_ms, 10);
        assert_eq!(w.as_slice()[1].open_time_ms, 20);
    }

    #[test]
    fn is_empty_lifecycle() {
        let mut w = Window::new(2);
        assert!(w.is_empty());
        w.push_closed(candle(1)).unwrap();
        assert!(!w.is_empty());
    }
}
