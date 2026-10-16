//! Ledger: the only module that writes trade-order rows and contract funds.
//!
//! Every public operation is one transaction, so an order row and the fund it
//! reserves, releases, or finalizes always change together. Callers decide the
//! policy (log, retry, propagate) on the returned error; the ledger itself never
//! swallows errors.

mod funds;
mod orders;

pub use orders::{
    Fill, create_pending_open, record_cancelled, record_failed, record_filled, record_placed,
    release_reserved, reserve_exit,
};

pub type Result<T> = std::result::Result<T, sqlx::Error>;
