//! Minimal cross-margin account/interest/repay client.
//!
//! The `kucoin` crate does not expose these endpoints, so they go through the
//! public `KuCoinClient::send`. All amounts are returned as exchange strings and
//! parsed by callers.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::KuCoinResponse;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Cross-margin account snapshot.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginAccount {
    pub status: Option<String>,
    pub accounts: Vec<MarginAsset>,
}

/// One currency row of a cross-margin account.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MarginAsset {
    pub currency: String,
    #[serde(default)]
    pub total: Option<String>,
    #[serde(default)]
    pub available: Option<String>,
    #[serde(default)]
    pub hold: Option<String>,
    #[serde(default)]
    pub liability: Option<String>,
    #[serde(default)]
    pub liability_principal: Option<String>,
    #[serde(default)]
    pub liability_interest: Option<String>,
}

/// One interest-history row.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct InterestItem {
    pub currency: String,
    #[serde(default)]
    pub day_ratio: Option<String>,
    pub interest_amount: String,
    pub created_time: i64,
}

/// One page of interest history.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct InterestPage {
    pub current_page: i64,
    pub page_size: i64,
    pub total_num: i64,
    pub total_page: i64,
    pub items: Vec<InterestItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepayRequest<'a> {
    currency: &'a str,
    size: String,
    is_isolated: bool,
    is_hf: bool,
}

/// Repay acknowledgement.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RepayData {
    pub order_no: String,
    pub actual_size: String,
}

fn err<T>(code: &str, msg: &Option<String>) -> Result<T, String> {
    Err(format!("kucoin margin error: code={code} msg={msg:?}"))
}

/// Whether an error message represents KuCoin throttling.
pub fn is_rate_limited(message: &str) -> bool {
    message.contains("rate limited") || message.contains("429")
}

/// Fetch the cross-margin (MARGIN_V2) account.
pub async fn account(kcc: &KuCoinClient) -> Result<MarginAccount, String> {
    let res = kcc
        .send::<KuCoinResponse<MarginAccount>>(
            "GET",
            "",
            "/api/v3/margin/accounts?quoteCurrency=USDT&queryType=MARGIN",
        )
        .await
        .map_err(|e| e.to_string())?;
    match res.data {
        Some(data) => Ok(data),
        None => err(&res.code, &res.msg),
    }
}

/// Fetch one page of interest history for `currency`.
pub async fn interest_page(
    kcc: &KuCoinClient,
    currency: &str,
    start_ms: i64,
    end_ms: i64,
    page: i64,
) -> Result<InterestPage, String> {
    let endpoint = format!(
        "/api/v3/margin/interest?currency={currency}&isIsolated=false&startTime={start_ms}&endTime={end_ms}&currentPage={page}&pageSize=500"
    );
    let res = kcc
        .send::<KuCoinResponse<InterestPage>>("GET", "", &endpoint)
        .await
        .map_err(|e| e.to_string())?;
    match res.data {
        Some(data) => Ok(data),
        None => err(&res.code, &res.msg),
    }
}

/// One repay-history row.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepayHistoryItem {
    pub order_no: String,
    pub currency: String,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub actual_size: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    pub created_time: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepayHistoryPage {
    pub current_page: i64,
    pub page_size: i64,
    pub total_num: i64,
    pub total_page: i64,
    pub items: Vec<RepayHistoryItem>,
}

/// Fetch one page of repay history for `currency`.
pub async fn repay_history(
    kcc: &KuCoinClient,
    currency: &str,
    start_ms: i64,
    end_ms: i64,
    page: i64,
) -> Result<RepayHistoryPage, String> {
    let endpoint = format!(
        "/api/v3/margin/repay?currency={currency}&isIsolated=false&startTime={start_ms}&endTime={end_ms}&currentPage={page}&pageSize=500"
    );
    let res = kcc
        .send::<KuCoinResponse<RepayHistoryPage>>("GET", "", &endpoint)
        .await
        .map_err(|e| e.to_string())?;
    match res.data {
        Some(data) => Ok(data),
        None => err(&res.code, &res.msg),
    }
}

/// Repay `size` of `currency` debt.
pub async fn repay(kcc: &KuCoinClient, currency: &str, size: String) -> Result<RepayData, String> {
    let body = serde_json::to_string(&RepayRequest {
        currency,
        size,
        is_isolated: false,
        is_hf: true,
    })
    .map_err(|e| e.to_string())?;
    let res = kcc
        .send::<KuCoinResponse<RepayData>>("POST", &body, "/api/v3/margin/repay")
        .await
        .map_err(|e| e.to_string())?;
    match res.data {
        Some(data) => Ok(data),
        None => err(&res.code, &res.msg),
    }
}

/// Minimum order notional (quote currency) for `currency`'s `-USDT` pair.
pub async fn min_notional(kcc: &KuCoinClient, currency: &str) -> Result<Decimal, String> {
    let symbol = format!("{currency}-USDT");
    let endpoint = format!("/api/v2/symbols/{symbol}");
    let res = kcc
        .send::<KuCoinResponse<SymbolMinimums>>("GET", "", &endpoint)
        .await
        .map_err(|e| e.to_string())?;
    let row = res
        .data
        .ok_or_else(|| format!("symbols lookup failed: {}", res.code))?;
    let min = [row.min_funds.as_deref(), row.quote_min_size.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.parse::<Decimal>().ok())
        .max();
    min.ok_or_else(|| format!("no minFunds for {symbol}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolMinimums {
    #[serde(default)]
    quote_min_size: Option<String>,
    #[serde(default)]
    min_funds: Option<String>,
}

/// Last price for `BASE-USDT`.
pub async fn last_price(
    kcc: &KuCoinClient,
    currency: &str,
) -> Result<rust_decimal::Decimal, String> {
    let symbol = format!("{currency}-USDT");
    let res = kcc
        .market()
        .get_last_price(&symbol)
        .await
        .map_err(|e| e.to_string())?;
    res.data
        .and_then(|d| d.price.parse().ok())
        .ok_or_else(|| format!("no price for {symbol}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_margin_account_with_liability_interest() {
        let json = r#"{"code":"200000","data":{
            "status":"EFFECTIVE","accounts":[
              {"currency":"SOL","total":"1.5","available":"1.0","hold":"0.5",
               "liability":"0.1","liabilityPrincipal":"0.09","liabilityInterest":"0.01"}
            ]}}"#;
        let res: KuCoinResponse<MarginAccount> = serde_json::from_str(json).unwrap();
        let acc = res.data.unwrap();
        assert_eq!(acc.status.as_deref(), Some("EFFECTIVE"));
        assert_eq!(acc.accounts[0].liability_interest.as_deref(), Some("0.01"));
    }

    #[test]
    fn parses_interest_page() {
        let json = r#"{"code":"200000","data":{
            "currentPage":1,"pageSize":50,"totalNum":1,"totalPage":1,
            "items":[{"currency":"SOL","dayRatio":"0.0003",
                      "interestAmount":"0.0001","createdTime":1729663213375}]}}"#;
        let res: KuCoinResponse<InterestPage> = serde_json::from_str(json).unwrap();
        let page = res.data.unwrap();
        assert_eq!(page.items[0].interest_amount, "0.0001");
    }

    #[test]
    fn parses_repay_history() {
        let json = r#"{"code":"200000","data":{
            "currentPage":1,"pageSize":50,"totalNum":1,"totalPage":1,
            "items":[{"orderNo":"r-1","currency":"SOL","size":"0.5",
                      "actualSize":"0.5","status":"SUCCESS","createdTime":1729663213375}]}}"#;
        let res: KuCoinResponse<RepayHistoryPage> = serde_json::from_str(json).unwrap();
        let page = res.data.unwrap();
        assert_eq!(page.items[0].order_no, "r-1");
        assert_eq!(page.items[0].actual_size.as_deref(), Some("0.5"));
    }

    #[test]
    fn parses_symbol_minimums() {
        let json = r#"{"code":"200000","data":{"symbol":"SOL-USDT","minFunds":"0.1","quoteMinSize":"0.1"}}"#;
        let res: KuCoinResponse<SymbolMinimums> = serde_json::from_str(json).unwrap();
        let row = res.data.unwrap();
        assert_eq!(row.min_funds.as_deref(), Some("0.1"));
        assert_eq!(row.quote_min_size.as_deref(), Some("0.1"));
    }

    #[test]
    fn symbol_minimums_allow_missing_fields() {
        let json = r#"{"code":"200000","data":{"symbol":"SOL-USDT"}}"#;
        let res: KuCoinResponse<SymbolMinimums> = serde_json::from_str(json).unwrap();
        let row = res.data.unwrap();
        assert!(row.min_funds.is_none());
        assert!(row.quote_min_size.is_none());
    }

    #[tokio::test]
    #[ignore = "live: needs margin credentials"]
    async fn live_margin_endpoints_parse() {
        let key = std::env::var("KC_KEY").ok();
        let secret = std::env::var("KC_SECRET").ok();
        let pass = std::env::var("KC_PASSPHRASE").ok();
        let (Some(key), Some(secret), Some(pass)) = (key, secret, pass) else {
            return;
        };
        let kcc = KuCoinClient::new(kucoin::client::rest::Credentials::new(&key, &secret, &pass));
        let acc = account(&kcc).await.expect("account");
        assert!(!acc.accounts.is_empty());
        let now = chrono::Utc::now().timestamp_millis();
        let page = interest_page(&kcc, "USDT", now - 86_400_000, now, 1)
            .await
            .expect("interest");
        assert!(page.page_size > 0);

        // Verify the repair-buy order shape without executing it.
        let req = crate::trader::margin_market_order(
            "SOL-USDT",
            kucoin::types::spot::Side::Buy,
            "0.001".to_string(),
            &uuid::Uuid::new_v4().to_string(),
            crate::trader::OrderIntent::Repay,
        );
        let res = kcc
            .margin()
            .place_order_test(req)
            .await
            .expect("order/test should reach the exchange");
        assert_eq!(res.code, "200000", "order/test rejected: {:?}", res.msg);
    }
}
