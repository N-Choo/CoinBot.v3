//! Public KuCoin currency quote for USDT-ERC20 withdrawals.
//!
//! The quote is public and needs no API credentials, so the gateway can show
//! the live fee without touching the exchange keys used by the workers.

use std::str::FromStr;
use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use serde::Deserialize;

/// Live ERC20 withdrawal values.
#[derive(Debug, Clone)]
pub struct WithdrawQuote {
    pub chain_name: String,
    pub fee: Decimal,
    pub min: Decimal,
    pub precision: u32,
}

#[derive(Deserialize)]
struct Envelope {
    data: Option<Currency>,
}

#[derive(Deserialize)]
struct Currency {
    chains: Vec<Chain>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Chain {
    chain_id: String,
    chain_name: String,
    #[serde(default)]
    withdrawal_min_fee: Option<String>,
    #[serde(default)]
    withdraw_min_fee: Option<String>,
    #[serde(default)]
    withdrawal_min_size: Option<String>,
    #[serde(default)]
    withdraw_min_size: Option<String>,
    #[serde(default)]
    is_withdraw_enabled: bool,
    #[serde(default)]
    withdraw_precision: Option<u32>,
}

/// Parse the public currency payload and pick the enabled `eth` chain.
pub fn parse_usdt_quote(body: &str) -> Result<WithdrawQuote, String> {
    let envelope: Envelope = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let currency = envelope
        .data
        .ok_or_else(|| "currency payload missing data".to_string())?;

    let chain = currency
        .chains
        .into_iter()
        .find(|chain| chain.chain_id == "eth")
        .ok_or_else(|| "USDT has no ERC20 chain".to_string())?;

    if !chain.is_withdraw_enabled {
        return Err("USDT ERC20 withdrawals are disabled".to_string());
    }

    let fee = chain
        .withdrawal_min_fee
        .or(chain.withdraw_min_fee)
        .ok_or_else(|| "missing withdrawal fee".to_string())?;
    let min = chain
        .withdrawal_min_size
        .or(chain.withdraw_min_size)
        .ok_or_else(|| "missing withdrawal minimum".to_string())?;

    Ok(WithdrawQuote {
        chain_name: chain.chain_name,
        fee: Decimal::from_str(&fee).map_err(|_| format!("bad fee: {fee}"))?,
        min: Decimal::from_str(&min).map_err(|_| format!("bad minimum: {min}"))?,
        precision: chain.withdraw_precision.unwrap_or(6),
    })
}

/// Fetch the live quote from the public endpoint.
pub async fn fetch_usdt_quote(client: &reqwest::Client) -> Result<WithdrawQuote, String> {
    let body = client
        .get("https://api.kucoin.com/api/v3/currencies/USDT")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_usdt_quote(&body)
}

/// Per-process quote cache with a configurable TTL.
///
/// Cloning shares the same cache, matching the other `web::Data` values held in
/// [`AppState`](crate::state::AppState).
#[derive(Clone)]
pub struct WithdrawQuoteCache {
    inner: std::sync::Arc<tokio::sync::Mutex<Option<(Instant, WithdrawQuote)>>>,
}

impl Default for WithdrawQuoteCache {
    fn default() -> Self {
        Self::new()
    }
}

impl WithdrawQuoteCache {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Return a cached quote when fresh, otherwise fetch and cache a new one.
    ///
    /// The network call happens outside the lock so a slow exchange cannot
    /// serialize every quote request (or withdrawal submission) behind it.
    pub async fn get_or_fetch(
        &self,
        client: &reqwest::Client,
        ttl: Duration,
    ) -> Result<WithdrawQuote, String> {
        {
            let guard = self.inner.lock().await;
            if let Some((fetched_at, quote)) = guard.as_ref()
                && fetched_at.elapsed() < ttl
            {
                return Ok(quote.clone());
            }
        }

        let quote = fetch_usdt_quote(client).await?;
        let mut guard = self.inner.lock().await;
        *guard = Some((Instant::now(), quote.clone()));
        Ok(quote)
    }

    /// Seed the cache in tests so handler tests do not call KuCoin.
    #[cfg(test)]
    pub async fn set_for_test(&self, quote: WithdrawQuote) {
        *self.inner.lock().await = Some((Instant::now(), quote));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_erc20_chain() {
        let body = r#"{"code":"200000","data":{"currency":"USDT","precision":8,"chains":[
            {"chainId":"bsc","chainName":"BEP20","withdrawalMinFee":"1","withdrawalMinSize":"10","isWithdrawEnabled":true,"withdrawPrecision":8},
            {"chainId":"eth","chainName":"ERC20","withdrawalMinFee":"5.5","withdrawalMinSize":"30","isWithdrawEnabled":true,"withdrawPrecision":6}
        ]}}"#;

        let quote = parse_usdt_quote(body).unwrap();
        assert_eq!(quote.chain_name, "ERC20");
        assert_eq!(quote.fee.to_string(), "5.5");
        assert_eq!(quote.min.to_string(), "30");
        assert_eq!(quote.precision, 6);
    }

    #[test]
    fn rejects_a_disabled_chain() {
        let body = r#"{"code":"200000","data":{"currency":"USDT","precision":8,"chains":[
            {"chainId":"eth","chainName":"ERC20","withdrawalMinFee":"5.5","withdrawalMinSize":"30","isWithdrawEnabled":false,"withdrawPrecision":6}
        ]}}"#;

        assert!(parse_usdt_quote(body).is_err());
    }

    #[test]
    fn accepts_the_alternate_fee_field_name() {
        let body = r#"{"code":"200000","data":{"currency":"USDT","precision":8,"chains":[
            {"chainId":"eth","chainName":"ERC20","withdrawMinFee":"5.5","withdrawMinSize":"30","isWithdrawEnabled":true,"withdrawPrecision":6}
        ]}}"#;

        assert_eq!(parse_usdt_quote(body).unwrap().fee.to_string(), "5.5");
    }
}
