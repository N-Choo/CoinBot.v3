use std::fmt::{self, Display};

pub enum Status {
    Pending = 0,
    Confirmed = 1,
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
