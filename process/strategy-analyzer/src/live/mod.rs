//! Phase 1 (shadow) live adapter (live spec §3, §3a, §3c, §4, §5, §5a).
//!
//! This module wires the pure [`crate::strategy`] core to a continuous, per-symbol loop that reads
//! KuCoin market data, carries state forward on disk, and **logs** signals. It is deliberately
//! additive: the core is untouched, and Phase 1 has **no gRPC server** and **publishes nothing** —
//! it never touches the engine or `signals:result`.
//!
//! Layout:
//! - [`config`] — [`LiveConfig`] (explicit symbol list, intervals, paths; env/config-file loaded).
//! - [`clock`] — [`Clock`] with a monotonic exchange offset and an injectable test clock.
//! - [`state_store`] — per-symbol file state: atomic writes, a single-writer lock,
//!   validate-or-re-warm, and pruning.
//! - [`poller`] — the network boundary ([`MarketData`]), the shared `allTickers` poller, and the
//!   per-symbol 4H candle poller (closed-candle rule + in-order backfill).
//! - [`sym_loop`] — the per-symbol loop that advances [`StrategyState`] and records its inputs.
//! - [`supervisor`] — spawns the loops and handles shutdown.

pub mod clock;
pub mod config;
pub mod poller;
pub mod state_store;
pub mod supervisor;
pub mod sym_loop;

pub use clock::{Clock, ExchangeClock, ManualClock};
pub use config::{LiveConfig, LiveConfigError};
pub use poller::{
    candle_is_closed, select_closed_candles, MarketData, SharedTickerPoller, TickerSnapshot,
    UreqMarketData,
};
pub use state_store::{ColdReason, LoadOutcome, StateStore, StateStoreError};
pub use supervisor::{shutdown_signal, LiveError, Supervisor};
pub use sym_loop::{LiveEvent, LiveInput, SymbolLoop};
