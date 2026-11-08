//! Signal gateway: turns agent predictions into trade-engine signals.

pub mod agent;
pub mod config;
pub mod core;
pub mod exchange;
pub mod persistence;
pub mod rpc;
pub mod signal;

// Flat re-exports keep the historical module paths (`crate::runner`,
// `signal_gateway::price`, ...) working after grouping files into folders.
pub use agent::{agent_client, prediction};
pub use core::{runner, scheduler};
pub use exchange::price;
pub use persistence as state;
pub use persistence::{audit, cursor};
pub use rpc::{grpc, publisher, symbols};
pub use signal::mapper;
