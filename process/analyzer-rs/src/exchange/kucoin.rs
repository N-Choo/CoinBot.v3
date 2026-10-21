use crate::exchange::timeframes::Timeframe;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("kuCoin HTTP {0}")]
    Http(u16),
    #[error("kuCoin API error: {0}")]
    Api(String),
    #[error("kuCoin transport error: {0}")]
    Transport(String),
    #[error("kuCoin parse error: {0}")]
    Parse(String),
}

pub fn parse_klines(raw: &str) -> Result<Vec<Candle>, ExchangeError> {
    let body: Value = serde_json::from_str(raw)
        .map_err(|e| ExchangeError::Parse(format!("invalid JSON: {e}")))?;
    if body.get("code").and_then(Value::as_str) != Some("200000") {
        return Err(ExchangeError::Api(
            body.get("msg")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        ));
    }
    let rows = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ExchangeError::Parse("missing data array".into()))?;
    let mut candles = Vec::with_capacity(rows.len());
    for row in rows {
        let parts = row
            .as_array()
            .ok_or_else(|| ExchangeError::Parse("row is not an array".into()))?;
        if parts.len() < 6 {
            return Err(ExchangeError::Parse("row has fewer than 6 fields".into()));
        }
        let field = |index: usize| -> Result<Decimal, ExchangeError> {
            let text = parts[index]
                .as_str()
                .ok_or_else(|| ExchangeError::Parse("non-string OHLCV field".into()))?;
            Decimal::from_str(text)
                .map_err(|e| ExchangeError::Parse(format!("bad decimal {text:?}: {e}")))
        };
        candles.push(Candle {
            open_time_ms: parts[0]
                .as_str()
                .ok_or_else(|| ExchangeError::Parse("non-string timestamp".into()))?
                .parse::<i64>()
                .map_err(|e| ExchangeError::Parse(format!("bad timestamp: {e}")))?
                * 1_000,
            open: field(1)?,
            close: field(2)?,
            high: field(3)?,
            low: field(4)?,
            volume: field(5)?,
        });
    }
    candles.sort_by_key(|candle| candle.open_time_ms);
    candles.dedup_by_key(|candle| candle.open_time_ms);
    Ok(candles)
}

pub fn fetch_klines(
    base_url: &str,
    symbol: &str,
    timeframe: Timeframe,
    limit: usize,
) -> Result<Vec<Candle>, ExchangeError> {
    let now_s = chrono::Utc::now().timestamp();
    let span_s = ((limit as i64) * timeframe.millis() / 1_000) + 120;
    let start_s = now_s - span_s;
    let response = ureq::get(&format!("{base_url}/api/v1/market/candles"))
        .query("type", timeframe.as_kucoin())
        .query("symbol", symbol)
        .query("startAt", &start_s.to_string())
        .query("endAt", &now_s.to_string())
        .timeout(Duration::from_secs(30))
        .call();
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, _)) => return Err(ExchangeError::Http(code)),
        Err(ureq::Error::Transport(error)) => {
            return Err(ExchangeError::Transport(error.to_string()))
        }
    };
    let text = response
        .into_string()
        .map_err(|e| ExchangeError::Transport(e.to_string()))?;
    parse_klines(&text)
}
