//! Backtest/replay harness. Depends on `strategy`; `strategy` never depends on this.

pub mod dataset;
pub mod fetch;
pub mod lock;
pub mod perturb;
pub mod publish;
pub mod replay;
pub mod report;
pub mod run;
pub mod sim;
pub mod tick_source;
pub mod verdict;
