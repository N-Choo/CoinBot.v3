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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DivergenceKind {
    RegularBullish,
    RegularBearish,
    HiddenBullish,
    HiddenBearish,
}

impl DivergenceKind {
    pub fn action(self) -> Direction {
        match self {
            DivergenceKind::RegularBullish | DivergenceKind::HiddenBullish => Direction::Long,
            DivergenceKind::RegularBearish | DivergenceKind::HiddenBearish => Direction::Short,
        }
    }

    pub fn reason(self) -> &'static str {
        match self {
            DivergenceKind::RegularBullish => "regular_bullish_divergence",
            DivergenceKind::RegularBearish => "regular_bearish_divergence",
            DivergenceKind::HiddenBullish => "hidden_bullish_divergence",
            DivergenceKind::HiddenBearish => "hidden_bearish_divergence",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DivergenceKind::RegularBullish => "regular_bullish",
            DivergenceKind::RegularBearish => "regular_bearish",
            DivergenceKind::HiddenBullish => "hidden_bullish",
            DivergenceKind::HiddenBearish => "hidden_bearish",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub ticker: String,
    pub action: Direction,
    pub confidence: Decimal,
    pub entry_price: Decimal,
    pub reason: String,
    pub sl_price: Decimal,
    pub tp_price: Decimal,
    pub divergence_index: usize,
    pub divergence_kind: DivergenceKind,
    pub divergence_open_time_ms: i64,
    pub divergence_age_candles: usize,
    pub signal_candle_open_time_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_maps_to_action_reason_and_str() {
        let cases = [
            (
                DivergenceKind::RegularBullish,
                Direction::Long,
                "regular_bullish_divergence",
                "regular_bullish",
            ),
            (
                DivergenceKind::RegularBearish,
                Direction::Short,
                "regular_bearish_divergence",
                "regular_bearish",
            ),
            (
                DivergenceKind::HiddenBullish,
                Direction::Long,
                "hidden_bullish_divergence",
                "hidden_bullish",
            ),
            (
                DivergenceKind::HiddenBearish,
                Direction::Short,
                "hidden_bearish_divergence",
                "hidden_bearish",
            ),
        ];

        for (kind, action, reason, as_str) in cases {
            assert_eq!(kind.action(), action);
            assert_eq!(kind.reason(), reason);
            assert_eq!(kind.as_str(), as_str);
        }
    }
}
