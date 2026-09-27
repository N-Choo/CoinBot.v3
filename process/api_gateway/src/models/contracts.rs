/// Signed contract submission (`POST /api/contracts/sign`).
#[derive(serde::Deserialize)]
pub struct SignRequest {
    /// One-time nonce previously issued by `/api/contracts/nonce`.
    pub nonce: String,
    /// JSON-encoded [`MessagePayload`] that was signed.
    pub message: String,
    /// Hex-encoded signature over `message`.
    pub signature: String,
}

/// Decoded contents of the signed message.
#[derive(serde::Deserialize)]
pub(crate) struct MessagePayload {
    /// Nonce embedded in the signed message.
    pub nonce: String,
    /// Bot settings, parsed into [`BotSettings`].
    pub settings: serde_json::Value,
}

/// Raw PascalCase bot settings as signed by the client.
#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SignedSettings {
    ticker: String,
    amount: String,
    take_profit: String,
    stop_loss: String,
}

/// Validated bot contract settings.
pub struct BotSettings {
    /// Trading pair, e.g. `BTC/USDT`.
    pub ticker: String,
    /// Position size as a decimal string.
    pub amount: String,
    /// Stop-loss distance, in percent.
    pub sl_pct: f32,
    /// Take-profit distance, in percent.
    pub tp_pct: f32,
}

impl BotSettings {
    /// Parse [`BotSettings`] from the signed JSON `settings` value.
    ///
    /// Returns a static error message when the shape or numeric fields are
    /// invalid.
    pub fn from_message(settings: serde_json::Value) -> Result<Self, &'static str> {
        let signed: SignedSettings = serde_json::from_value(settings)
            .map_err(|_| "Invalid bot settings in signed message")?;
        Ok(Self {
            ticker: signed.ticker,
            amount: signed.amount,
            sl_pct: signed
                .stop_loss
                .parse()
                .map_err(|_| "Invalid StopLoss value")?,
            tp_pct: signed
                .take_profit
                .parse()
                .map_err(|_| "Invalid TakeProfit value")?,
        })
    }

    /// Validate the settings.
    ///
    /// Requires a positive amount, SL/TP within 0–100, SL strictly below TP,
    /// and a `/USDT` pair. Returns a static error message otherwise.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self
            .amount
            .parse::<f64>()
            .map_err(|_| "Amount must be a number")?
            <= 0.0
        {
            return Err("Amount must be a positive number");
        }
        if !(0.0..=100.0).contains(&self.sl_pct) {
            return Err("Stop Loss must be between 0 and 100");
        }
        if !(0.0..=100.0).contains(&self.tp_pct) {
            return Err("Take Profit must be between 0 and 100");
        }
        if self.sl_pct >= self.tp_pct {
            return Err("Stop Loss must be less than Take Profit");
        }
        if !self.ticker.ends_with("/USDT") {
            return Err("Only USDT pairs are supported");
        }
        Ok(())
    }
}
