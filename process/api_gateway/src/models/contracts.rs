#[derive(serde::Deserialize)]
pub struct SignRequest {
    pub nonce: String,
    pub message: String,
    pub signature: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct MessagePayload {
    pub nonce: String,
    pub settings: serde_json::Value,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SignedSettings {
    ticker: String,
    amount: String,
    take_profit: String,
    stop_loss: String,
}

pub struct BotSettings {
    pub ticker: String,
    pub amount: String,
    pub sl_pct: f32,
    pub tp_pct: f32,
}

impl BotSettings {
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
