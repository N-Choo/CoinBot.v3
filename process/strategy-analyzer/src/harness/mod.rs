//! Backtest/replay harness. Depends on `strategy`; `strategy` never depends on this.

pub mod dataset;
pub mod tick_source;
pub mod replay;
pub mod sim;
pub mod report;
pub mod lock;
pub mod fetch;
