use std::fmt::{self, Display};

/// Lifecycle status of a [`Contract`](super::Contract).
pub enum Status {
    /// Open and eligible for execution.
    Active = 0,
    /// Paused by the user.
    Inactive = 1,
    /// Closed with its target reached.
    Completed = 2,
    /// Closed by the user before completion.
    Cancelled = 3,
    /// Closed due to an error.
    Failed = 4,
}

impl Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Active => write!(f, "active"),
            Status::Inactive => write!(f, "inactive"),
            Status::Completed => write!(f, "completed"),
            Status::Cancelled => write!(f, "cancelled"),
            Status::Failed => write!(f, "failed"),
        }
    }
}
