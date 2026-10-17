#![deny(clippy::float_arithmetic)]

pub const STATE_SCHEMA_VERSION: u32 = 1;

pub mod config;
pub mod decimal;
pub mod error;
pub mod indicators;
pub mod signals;
pub mod step;
pub mod structure;
pub mod types;
pub mod zones;
