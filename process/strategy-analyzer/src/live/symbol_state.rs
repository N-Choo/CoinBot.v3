//! Shared set of engine-active symbols.
//!
//! Written by the gRPC activation poller (`grpc::run_activation`) and read by the supervisor to
//! decide which symbols to run. Cheap to share via `Arc`; reads/writes are short and non-async.

use std::collections::BTreeSet;
use std::sync::RwLock;

/// The engine's currently active ticker set.
#[derive(Debug, Default)]
pub struct SymbolState {
    active: RwLock<BTreeSet<String>>,
}

impl SymbolState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the active set with `symbols`.
    pub fn replace(&self, symbols: impl IntoIterator<Item = String>) {
        *self.active.write().unwrap() = symbols.into_iter().collect();
    }

    /// A sorted snapshot of the active set.
    pub fn snapshot(&self) -> Vec<String> {
        self.active.read().unwrap().iter().cloned().collect()
    }
}

/// The engine/DB and KuCoin use different separators: contracts store `BASE/QUOTE`, while KuCoin's
/// API and the state/stream file names use `BASE-QUOTE` (a `/` would also break the file path).
/// Convert an engine ticker to its KuCoin form.
pub fn to_kucoin(symbol: &str) -> String {
    symbol.replace('/', "-")
}

/// Inverse of [`to_kucoin`]: KuCoin `BASE-QUOTE` -> engine/DB `BASE/QUOTE`. The engine matches
/// signals against contract tickers, which are slash-separated.
pub fn to_contract(symbol: &str) -> String {
    symbol.replacen('-', "/", 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty_and_replaces() {
        let s = SymbolState::new();
        assert!(s.snapshot().is_empty());
        s.replace(["XRP-USDT".to_string(), "BTC-USDT".to_string()]);
        assert_eq!(s.snapshot(), vec!["BTC-USDT", "XRP-USDT"]); // sorted
        s.replace(["ETH-USDT".to_string()]);
        assert_eq!(s.snapshot(), vec!["ETH-USDT"]); // replaced, not merged
    }

    #[test]
    fn symbol_separators_round_trip() {
        assert_eq!(to_kucoin("BTC/USDT"), "BTC-USDT");
        assert_eq!(to_kucoin("XRP-USDT"), "XRP-USDT"); // already KuCoin form
        assert_eq!(to_contract("BTC-USDT"), "BTC/USDT");
        assert_eq!(to_contract("BTC/USDT"), "BTC/USDT"); // already contract form
    }
}
