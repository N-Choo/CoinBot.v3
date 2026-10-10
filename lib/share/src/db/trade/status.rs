use std::fmt::{self, Display};

/// Lifecycle status of a [`TradeOrder`](super::TradeOrder).
pub enum Status {
    /// Recorded locally, not yet submitted to the exchange.
    Pending = 0,
    /// Accepted by the exchange and working.
    Open = 1,
    /// Fully executed.
    Filled = 2,
    /// Cancelled before execution.
    Cancelled = 3,
    /// Rejected or otherwise failed.
    Failed = 4,
}

impl Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::Open => write!(f, "open"),
            Status::Filled => write!(f, "filled"),
            Status::Cancelled => write!(f, "cancelled"),
            Status::Failed => write!(f, "failed"),
        }
    }
}

/// What a trade order does to the position.
pub enum Action {
    /// Enter the position.
    Open = 0,
    /// Exit the position.
    Close = 1,
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Open => write!(f, "open"),
            Action::Close => write!(f, "close"),
        }
    }
}
