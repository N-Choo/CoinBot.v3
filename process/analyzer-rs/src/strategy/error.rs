use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    #[error("decimal overflow")]
    Overflow,
    #[error("division by zero")]
    DivByZero,
    #[error("invalid config: {0}")]
    ConfigInvalid(String),
    #[error("duplicate candle at {0}")]
    DuplicateCandle(i64),
    #[error("candle out of order at {got_ms} (last {last_ms})")]
    OutOfOrderCandle { got_ms: i64, last_ms: i64 },
    #[error("window too short: need {need}, got {got}")]
    WindowTooShort { need: usize, got: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
