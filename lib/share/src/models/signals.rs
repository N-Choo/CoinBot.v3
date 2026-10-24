use serde::Deserialize;

/// A trade signal emitted by the analyzer and consumed by the trade engine.
#[derive(Debug, Clone, Deserialize)]
pub struct TradeSignal {
    /// Trading pair (e.g. `BTC/USDT`).
    pub ticker: String,
    /// Direction, e.g. `BUY` or `SELL`.
    pub action: String,
    /// Signal confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// Suggested entry price.
    pub entry_price: f64,
    /// Human-readable rationale.
    pub reason: String,
}

impl TradeSignal {
    /// Log a one-line summary of the signal.
    pub fn log(&self) {
        log::info!(
            "signal: {} {} conf={:.1} entry=${:.2} ({})",
            self.ticker,
            self.action,
            self.confidence,
            self.entry_price,
            self.reason
        );
    }
}
