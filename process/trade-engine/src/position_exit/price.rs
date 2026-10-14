//! Last-price lookups for held symbols.

use std::collections::HashMap;
use std::str::FromStr;

use kucoin::client::rest::KuCoinClient;
use rust_decimal::Decimal;

use super::SweepError;

/// Fetch `symbol -> last price` for the given margin symbols.
///
/// KuCoin margin has no last-price endpoint of its own (its mark-price feed is
/// BTC-denominated and BTC-pairs only) — margin trades on the same order book as
/// spot, so this uses the shared market-data endpoint. One light
/// `GET /api/v1/market/orderbook/level1` request per held symbol (weight 1)
/// instead of the heavy `allTickers` endpoint, so we only pay for the symbols
/// actually held and avoid tripping KuCoin's per-IP rate limit.
pub(super) async fn fetch_last_prices(
    kcc: &KuCoinClient,
    symbols: &[String],
) -> Result<HashMap<String, Decimal>, SweepError> {
    let mut prices = HashMap::with_capacity(symbols.len());
    for symbol in symbols {
        let res = kcc.market().get_last_price(symbol).await.map_err(|e| {
            if e.is_rate_limited() {
                SweepError::RateLimited
            } else {
                SweepError::Other(format!("level1 request failed for {symbol}: {e}"))
            }
        })?;

        let Some(data) = res.data else {
            continue;
        };
        if let Ok(last) = Decimal::from_str(&data.price) {
            prices.insert(symbol.clone(), last);
        }
    }
    Ok(prices)
}
