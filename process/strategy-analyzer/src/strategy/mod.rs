#![deny(clippy::float_arithmetic)]

pub const STATE_SCHEMA_VERSION: u32 = 1;

pub mod decimal;
pub mod error;
pub mod config;
pub mod types;
pub mod step;
pub mod indicators;
pub mod structure;
pub mod zones;
pub mod signals;
