//! Ethereum JSON-RPC helpers for verifying deposit transactions.
//!
//! `ETH_RPC_URLS` may contain a comma-separated list of independent providers;
//! calls are tried in order. `ETH_CONFIRMATIONS` (default 12) is required before
//! a deposit is accepted.

use std::sync::OnceLock;

use ethers::types::U64;
use ethers::{providers::Middleware, types::Transaction as EthTx};

type Provider = ethers::providers::Provider<ethers::providers::Http>;

const DEFAULT_RPC: &str = "https://ethereum-rpc.publicnode.com";
const DEFAULT_CONFIRMATIONS: u64 = 12;

/// Parse a comma-separated provider list, falling back to the default.
pub fn parse_urls(raw: Option<&str>) -> Vec<String> {
    let mut urls: Vec<String> = raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect();

    urls.dedup();
    if urls.is_empty() {
        urls.push(DEFAULT_RPC.to_string());
    }
    urls
}

/// Parse the confirmation requirement, falling back to 12.
pub fn parse_confirmations(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_CONFIRMATIONS)
}

fn providers() -> &'static [Provider] {
    static PROVIDERS: OnceLock<Vec<Provider>> = OnceLock::new();
    PROVIDERS.get_or_init(|| {
        let raw = std::env::var("ETH_RPC_URLS")
            .ok()
            .or_else(|| std::env::var("ETH_RPC_URL").ok());
        parse_urls(raw.as_deref())
            .into_iter()
            .filter_map(|url| Provider::try_from(url.as_str()).ok())
            .collect()
    })
}

fn confirmations() -> u64 {
    parse_confirmations(std::env::var("ETH_CONFIRMATIONS").ok().as_deref())
}

/// Ethereum JSON-RPC helpers for verifying deposit transactions.
pub struct Rpc;

impl Rpc {
    /// Fetch a transaction by hash, trying each configured provider in order.
    pub async fn get_transaction(tx_hash: &str) -> Result<EthTx, String> {
        if tx_hash.trim().is_empty() {
            return Err("Transaction hash is empty".to_string());
        }

        let hash: ethers::types::TxHash = tx_hash
            .parse()
            .map_err(|e| format!("Invalid tx hash format: {e}"))?;

        let mut last_error = "no Ethereum RPC provider configured".to_string();
        for provider in providers() {
            match provider.get_transaction(hash).await {
                Ok(Some(tx)) => return Ok(tx),
                Ok(None) => last_error = "Transaction not found".to_string(),
                Err(e) => last_error = format!("RPC error: {e}"),
            }
        }
        Err(last_error)
    }

    /// Confirm a transaction was mined successfully and has enough confirmations.
    pub async fn confirm_onchain_success(tx_hash: &str) -> Result<(), String> {
        Self::confirm_onchain_success_at(tx_hash, confirmations()).await
    }

    /// Confirmation check with an explicit depth. `0` means "mined, ignore depth",
    /// which is useful for early intake validation before the worker enforces
    /// the configured depth.
    pub async fn confirm_onchain_success_at(tx_hash: &str, required: u64) -> Result<(), String> {
        let hash: ethers::types::TxHash = tx_hash
            .parse()
            .map_err(|e| format!("Invalid tx hash format: {e}"))?;
        let mut last_error = "no Ethereum RPC provider configured".to_string();

        for provider in providers() {
            let receipt = match provider.get_transaction_receipt(hash).await {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    last_error = "Transaction receipt not found — tx may be pending".to_string();
                    continue;
                }
                Err(e) => {
                    last_error = format!("RPC error fetching receipt: {e}");
                    continue;
                }
            };

            match receipt.status {
                Some(status) if status == U64::from(1) => {}
                Some(status) if status == U64::from(0) => {
                    return Err("Transaction reverted on-chain (status=0)".to_string());
                }
                Some(_) => return Err("Transaction status unknown".to_string()),
                None => return Err("Transaction status unknown (pre-Byzantium)".to_string()),
            }

            let Some(block) = receipt.block_number else {
                last_error = "Transaction receipt has no block number".to_string();
                continue;
            };

            let current = provider
                .get_block_number()
                .await
                .map_err(|e| format!("RPC error fetching block number: {e}"))?;

            if current < block + U64::from(required) {
                return Err(format!(
                    "Waiting for confirmations: {current} of {} (need {required})",
                    block + U64::from(required)
                ));
            }

            return Ok(());
        }

        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_provider_list() {
        assert_eq!(
            parse_urls(Some("https://a.example, https://b.example")),
            vec!["https://a.example", "https://b.example"]
        );
        assert_eq!(parse_urls(Some("")), vec![DEFAULT_RPC]);
        assert_eq!(parse_urls(None), vec![DEFAULT_RPC]);
    }

    #[test]
    fn parses_confirmations() {
        assert_eq!(parse_confirmations(Some("24")), 24);
        assert_eq!(parse_confirmations(None), DEFAULT_CONFIRMATIONS);
        assert_eq!(parse_confirmations(Some("nope")), DEFAULT_CONFIRMATIONS);
    }
}
