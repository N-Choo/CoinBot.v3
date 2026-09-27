//! Shared library for CoinBot.v3 services.
//!
//! Common building blocks used by the API gateway, deposit worker, and trade
//! engine:
//!
//! - [`auth`] — EIP-191 wallet signature recovery.
//! - [`cache`] — Redis-backed key/value store plus pub/sub.
//! - [`config`] — environment configuration helpers.
//! - [`db`] — PostgreSQL models and query filters.
//! - [`erc20`] — ERC-20 `transfer` calldata decoding.
//! - [`error`] — shared error types.
//! - [`logger`] — logging initialisation.
//! - [`models`] — domain models (trade signals).
//! - [`rpc`] — Ethereum JSON-RPC helpers.
//!
//! Generated protobuf/gRPC types for the `wallet` and `analyzer` services are
//! re-exported at the crate root.
pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod erc20;
pub mod error;
pub mod logger;
pub mod models;
pub mod rpc;

#[allow(clippy::result_large_err)]
mod wallet {
    tonic::include_proto!("wallet");
}

#[allow(clippy::result_large_err)]
mod analyzer {
    tonic::include_proto!("analyzer");
}

pub use analyzer::*;
pub use config::{ProcessConfig, ServiceConfig};
pub use error::{ProcessError, ServiceError};
pub use wallet::*;
