use std::time::Duration;

use share::engine_service_client::EngineServiceClient;
use share::{Empty, ProcessError};
use tonic::transport::Endpoint;

pub fn normalize_symbols(raw: Vec<String>) -> Vec<String> {
    let mut symbols: Vec<String> = raw
        .into_iter()
        .map(|symbol| symbol.trim().replace('/', "-").to_uppercase())
        .filter(|symbol| !symbol.is_empty())
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

pub async fn fetch_active_symbols(
    endpoint: &str,
    timeout: Duration,
) -> Result<Vec<String>, ProcessError> {
    let channel = Endpoint::from_shared(endpoint.to_string())
        .map_err(|e| {
            ProcessError::InvalidConfig(format!("invalid engine endpoint {endpoint:?}: {e}"))
        })?
        .timeout(timeout)
        .connect_timeout(timeout)
        .connect_lazy();
    let mut client = EngineServiceClient::new(channel);
    let response = tokio::time::timeout(timeout, client.get_active_symbols(Empty {}))
        .await
        .map_err(|_| ProcessError::Network("GetActiveSymbols timed out".into()))?
        .map_err(|e| ProcessError::Network(format!("GetActiveSymbols: {e}")))?;
    Ok(normalize_symbols(response.into_inner().tickers))
}

pub async fn fetch_active_symbols_with_retry(
    endpoint: &str,
    timeout: Duration,
    attempts: usize,
) -> Result<Vec<String>, ProcessError> {
    let attempts = attempts.max(1);
    let mut last_error = None;
    for attempt in 0..attempts {
        match fetch_active_symbols(endpoint, timeout).await {
            Ok(symbols) => return Ok(symbols),
            Err(e) => {
                last_error = Some(e);
                if attempt + 1 < attempts {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                }
            }
        }
    }
    Err(last_error.expect("at least one attempt"))
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
}
