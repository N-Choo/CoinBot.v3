use std::fmt::{self, Display};

/// Lifecycle status of a [`Deposit`](super::Deposit).
pub enum Status {
    /// Recorded but not yet swept/confirmed.
    Pending = 0,
    /// Swept and credited to the user.
    Confirmed = 1,
    /// Rejected or failed during processing.
    Failed = 2,
}

impl Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::Confirmed => write!(f, "confirmed"),
            Status::Failed => write!(f, "failed"),
        }
    }
}
