#![deny(clippy::float_arithmetic)]

/// State schema version. Bumped to 2 when `zones_touched_this_candle` (the candle-close wick
/// reconciliation ledger, §7.2) was added to `StrategyState`: a v1 snapshot has no such field, so a
/// v2 reader must not treat the two shapes as interchangeable.
pub const STATE_SCHEMA_VERSION: u32 = 2;

pub mod config;
pub mod decimal;
pub mod error;
pub mod indicators;
pub mod signals;
pub mod step;
pub mod structure;
pub mod types;
pub mod zones;
