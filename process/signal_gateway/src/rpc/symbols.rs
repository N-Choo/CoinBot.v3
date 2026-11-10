//! Fetches active symbols from the trade engine.

use std::time::Duration;

use share::engine_service_client::EngineServiceClient;
use share::{Empty, ProcessError};

use crate::grpc::{lazy_channel, status_error};
use crate::prediction::normalize_symbol;

/// Normalize, sort, and de-duplicate a list of raw ticker symbols.
pub fn normalize_symbols(raw: Vec<String>) -> Vec<String> {
    let mut symbols: Vec<String> = raw
        .into_iter()
        .map(|symbol| normalize_symbol(&symbol))
        .filter(|symbol| !symbol.is_empty())
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

/// Fetch the trade engine's active symbols over gRPC.
pub async fn fetch_active_symbols(
    endpoint: &str,
    timeout: Duration,
    token: &str,
) -> Result<Vec<String>, ProcessError> {
    let channel = lazy_channel(endpoint, timeout)?;
    let mut client = EngineServiceClient::new(channel);
    let request = share::grpc_auth::with_token(Empty {}, token)
        .map_err(|status| ProcessError::Internal(status.to_string()))?;
    let response = tokio::time::timeout(timeout, client.get_active_symbols(request))
        .await
        .map_err(|_| ProcessError::Network("GetActiveSymbols timed out".into()))?
        .map_err(|status| status_error("GetActiveSymbols", status))?;
    Ok(normalize_symbols(response.into_inner().tickers))
}

/// Backoff delay for a retry attempt, capped at 30 seconds.
pub fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs((1u64 << attempt.min(5)).min(30))
}

/// Fetch active symbols, retrying transient network errors only.
pub async fn fetch_active_symbols_with_retry(
    endpoint: &str,
    timeout: Duration,
    attempts: usize,
    token: &str,
) -> Result<Vec<String>, ProcessError> {
    let attempts = attempts.max(1);
    let mut last_error = None;
    for attempt in 0..attempts {
        match fetch_active_symbols(endpoint, timeout, token).await {
            Ok(symbols) => return Ok(symbols),
            Err(e @ ProcessError::Network(_)) => {
                last_error = Some(e);
                if attempt + 1 < attempts {
                    tokio::time::sleep(retry_delay(attempt)).await;
                }
            }
            // Permanent errors (e.g. a rejected request or bad configuration)
            // are not retried.
            Err(e) => return Err(e),
        }
    }
    match last_error {
        Some(e) => Err(e),
        None => Err(ProcessError::Internal(
            "no symbol fetch attempt was made".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dedupes_and_sorts() {
        let symbols = normalize_symbols(vec![
            "btc/usdt".into(),
            " ETH-USDT ".into(),
            "BTC-USDT".into(),
            "".into(),
        ]);
        assert_eq!(symbols, vec!["BTC-USDT", "ETH-USDT"]);
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let delays: Vec<u64> = (0..=5)
            .map(|attempt| retry_delay(attempt).as_secs())
            .collect();
        assert_eq!(delays, vec![1, 2, 4, 8, 16, 30]);
        assert_eq!(retry_delay(6).as_secs(), 30);
        assert_eq!(retry_delay(usize::MAX).as_secs(), 30);
    }
}
