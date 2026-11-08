//! KuCoin last-price source.

use async_trait::async_trait;
use serde::Deserialize;
use share::ProcessError;

/// Source of the latest traded price for a symbol.
#[async_trait]
pub trait LastPriceSource: Send + Sync {
    /// Return the latest price for `symbol`.
    async fn last_price(&self, symbol: &str) -> Result<f64, ProcessError>;
}

#[derive(Deserialize)]
struct Envelope {
    data: Level1,
}

#[derive(Deserialize)]
struct Level1 {
    price: String,
}

/// Parse a KuCoin level1 price response.
pub fn parse_last_price(json: &str) -> Result<f64, ProcessError> {
    let envelope: Envelope = serde_json::from_str(json)
        .map_err(|e| ProcessError::Internal(format!("bad KuCoin response: {e}")))?;
    let price = envelope
        .data
        .price
        .trim()
        .parse::<f64>()
        .map_err(|e| ProcessError::Internal(format!("bad KuCoin price: {e}")))?;
    if !price.is_finite() || price <= 0.0 {
        return Err(ProcessError::Internal(format!(
            "KuCoin returned unusable price {price}"
        )));
    }
    Ok(price)
}

/// HTTP-backed `LastPriceSource` for KuCoin.
pub struct KuCoinPrice {
    base_url: String,
    client: reqwest::Client,
}

impl KuCoinPrice {
    /// Build the client and validate the base URL.
    pub fn new(
        base_url: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Result<Self, ProcessError> {
        let base_url = base_url.into();
        let parsed = reqwest::Url::parse(&base_url).map_err(|e| {
            ProcessError::InvalidConfig(format!("invalid KUCOIN_BASE_URL {base_url:?}: {e}"))
        })?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(ProcessError::InvalidConfig(format!(
                "KUCOIN_BASE_URL must use http or https, got {}",
                parsed.scheme()
            )));
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| ProcessError::Internal(format!("build KuCoin client: {e}")))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }
}

#[async_trait]
impl LastPriceSource for KuCoinPrice {
    /// Fetch the level1 price for one symbol.
    async fn last_price(&self, symbol: &str) -> Result<f64, ProcessError> {
        let response = self
            .client
            .get(format!("{}/api/v1/market/orderbook/level1", self.base_url))
            .query(&[("symbol", symbol)])
            .send()
            .await
            .map_err(|e| {
                ProcessError::Network(format!("KuCoin price request for {symbol}: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(ProcessError::Network(format!(
                "KuCoin price status {} for {symbol}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| ProcessError::Network(format!("KuCoin price body for {symbol}: {e}")))?;
        parse_last_price(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kucoin_envelope() {
        let price = parse_last_price(r#"{"data":{"price":"68123.4"}}"#).unwrap();
        assert_eq!(price, 68123.4);
    }

    #[test]
    fn rejects_zero_prices() {
        assert!(parse_last_price(r#"{"data":{"price":"0"}}"#).is_err());
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_last_price("not json").is_err());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let price = parse_last_price(r#"{"data":{"price":" 68123.4 "}}"#).unwrap();
        assert_eq!(price, 68123.4);
    }

    #[test]
    fn rejects_invalid_base_urls() {
        assert!(KuCoinPrice::new("not a url", std::time::Duration::from_secs(1)).is_err());
        assert!(KuCoinPrice::new("ftp://example.com", std::time::Duration::from_secs(1)).is_err());
    }

    #[test]
    fn accepts_trailing_slash() {
        assert!(KuCoinPrice::new("http://example.com/", std::time::Duration::from_secs(1)).is_ok());
    }
}
