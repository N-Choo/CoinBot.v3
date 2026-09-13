use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TradeSignal {
    pub ticker: String,
    pub action: String,
    pub confidence: f64,
    pub entry_price: f64,
    pub sl_price: f64,
    pub tp_price: f64,
    pub reason: String,
}

impl TradeSignal {
    pub fn log(&self) {
        log::info!(
            "signal: {} {} conf={:.1} entry=${:.2} sl=${:.2} tp=${:.2} ({})",
            self.ticker,
            self.action,
            self.confidence,
            self.entry_price,
            self.sl_price,
            self.tp_price,
            self.reason
        );
    }
}
