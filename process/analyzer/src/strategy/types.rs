use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candle {
    pub open_time_ms: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub ticker: String,
    pub action: Direction,
    pub confidence: Decimal,
    pub entry_price: Decimal,
    pub reason: String,
    pub signal_candle_open_time_ms: i64,
}
