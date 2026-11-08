//! Local persistence: gateway state, round-robin cursor, and audit log.

pub mod audit;
pub mod cursor;
mod store;

pub use store::*;
