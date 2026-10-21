use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error("candle out of order at {got_ms} (expected > {prev_ms})")]
    OutOfOrderCandle { got_ms: i64, prev_ms: i64 },
    #[error("duplicate candle at {0}")]
    DuplicateCandle(i64),
    #[error("gapped candles: expected {expected_ms}, got {got_ms}")]
    GappedCandle { expected_ms: i64, got_ms: i64 },
    #[error("malformed candle at {at_ms}: {why}")]
    MalformedCandle { at_ms: i64, why: &'static str },
    #[error("malformed tick: price {price}")]
    MalformedTick { price: String },
    #[error("tick out of order at {got_ms} (last {last_ms})")]
    OutOfOrderTick { got_ms: i64, last_ms: i64 },
    #[error("decimal overflow")]
    DecimalOverflow,
    #[error("division by zero")]
    DivideByZero,
    #[error("invalid config: {0}")]
    ConfigInvalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_are_equatable_and_displayable() {
        let e = StrategyError::MalformedTick { price: "-1".into() };
        assert_eq!(e.clone(), e);
        assert!(e.to_string().contains("malformed tick"));
    }
}
