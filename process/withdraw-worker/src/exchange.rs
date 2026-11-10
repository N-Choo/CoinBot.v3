//! KuCoin calls used by the withdrawal worker.
//!
//! All exchange-specific parsing lives here so the worker loop stays focused on
//! state transitions and money movement.

use std::str::FromStr;

use kucoin::client::rest::KuCoinClient;
use kucoin::types::transfer::{AccountType, TransferRequest, TransferType};
use kucoin::types::withdraw::{WithdrawRequest, WithdrawResponse, WithdrawType};
use kucoin::types::KuCoinResponse;
use rust_decimal::Decimal;
use serde::Deserialize;

/// Live ERC20 withdrawal values used by the worker.
#[derive(Debug, Clone)]
pub struct Quote {
    pub fee: Decimal,
    pub min: Decimal,
    pub precision: u32,
}

#[derive(Deserialize)]
struct CurrencyEnvelope {
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

/// Live ERC20 quote from the public currency endpoint.
pub async fn quote(kcc: &KuCoinClient) -> Result<Quote, String> {
    let response = kcc
        .send::<CurrencyEnvelope>("GET", "", "/api/v3/currencies/USDT")
        .await
        .map_err(|e| e.to_string())?;

    let chain = response
        .data
        .ok_or_else(|| "currency payload missing data".to_string())?
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

    Ok(Quote {
        fee: Decimal::from_str(&fee).map_err(|_| format!("bad fee: {fee}"))?,
        min: Decimal::from_str(&min).map_err(|_| format!("bad minimum: {min}"))?,
        precision: chain.withdraw_precision.unwrap_or(6),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountRow {
    currency: String,
    #[serde(default)]
    balance: Option<String>,
    #[serde(default)]
    available: Option<String>,
}

/// Free USDT in the Main (spot/funding) account.
pub async fn main_available(kcc: &KuCoinClient) -> Result<Decimal, String> {
    let response = kcc
        .send::<KuCoinResponse<Vec<AccountRow>>>(
            "GET",
            "",
            "/api/v1/accounts?type=main&currency=USDT",
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(response
        .data
        .and_then(|rows| rows.into_iter().find(|row| row.currency == "USDT"))
        .and_then(|row| row.available.or(row.balance))
        .and_then(|value| Decimal::from_str(&value).ok())
        .unwrap_or(Decimal::ZERO))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarginAccount {
    accounts: Vec<MarginAsset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarginAsset {
    currency: String,
    #[serde(default)]
    available: Option<String>,
}

/// Free USDT in the cross-margin (MARGIN_V2) account.
pub async fn margin_available(kcc: &KuCoinClient) -> Result<Decimal, String> {
    let response = kcc
        .send::<KuCoinResponse<MarginAccount>>(
            "GET",
            "",
            "/api/v3/margin/accounts?quoteCurrency=USDT&queryType=MARGIN",
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(response
        .data
        .and_then(|account| {
            account
                .accounts
                .into_iter()
                .find(|row| row.currency == "USDT")
        })
        .and_then(|row| row.available)
        .and_then(|value| Decimal::from_str(&value).ok())
        .unwrap_or(Decimal::ZERO))
}

/// Move `amount` USDT from MarginV2 to Main. `client_oid` makes it idempotent.
pub async fn transfer_to_main(
    kcc: &KuCoinClient,
    amount: Decimal,
    client_oid: &str,
) -> Result<(), String> {
    let mut request = TransferRequest::new(
        "USDT",
        0.0,
        AccountType::MarginV2,
        AccountType::Main,
        TransferType::Internal,
    );
    request.amount = amount.normalize().to_string();
    request.client_oid = client_oid.to_string();

    let response = kcc
        .transfer()
        .execute(request)
        .await
        .map_err(|e| e.to_string())?;

    if response.data.is_some() {
        Ok(())
    } else {
        Err(format!(
            "transfer rejected: code={} msg={:?}",
            response.code, response.msg
        ))
    }
}

/// Why a create call failed.
#[derive(Debug)]
pub enum CreateError {
    /// The exchange explicitly rejected the request; no money moved.
    Rejected(String),
    /// A retryable response (rate limit, auth, gateway) where the request was
    /// not accepted. The row may be returned to `pending`.
    Retry(String),
    /// The outcome is unknown (transport or decoding failure).
    Ambiguous(String),
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(message) | Self::Retry(message) | Self::Ambiguous(message) => {
                f.write_str(message)
            }
        }
    }
}

/// Create the withdrawal. `remark` is the local withdrawal id.
pub async fn create(
    kcc: &KuCoinClient,
    amount: Decimal,
    destination: &str,
    remark: &str,
) -> Result<String, CreateError> {
    let mut request = WithdrawRequest::new("USDT", destination, 0.0, WithdrawType::Address);
    request.amount = amount.normalize().to_string();
    request.chain = Some("eth".into());
    request.remark = Some(remark.into());
    request.fee_deduct_type = Some("EXTERNAL".into());

    let response = kcc
        .withdraw()
        .execute(request)
        .await
        .map_err(|e| match e.status() {
            // Rate limiting and auth failures mean "not accepted": keep the
            // row pending and retry instead of rejecting the user.
            Some(status)
                if status.as_u16() == 429 || status.as_u16() == 401 || status.as_u16() == 403 =>
            {
                CreateError::Retry(e.to_string())
            }
            // Other client errors are definite rejections.
            Some(status) if status.is_client_error() => CreateError::Rejected(e.to_string()),
            // 5xx and transport errors may have reached the exchange.
            _ => CreateError::Ambiguous(e.to_string()),
        })?;

    match response.data {
        Some(WithdrawResponse { withdrawal_id }) => Ok(withdrawal_id),
        None => Err(CreateError::Rejected(format!(
            "withdraw rejected: code={} msg={:?}",
            response.code, response.msg
        ))),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub status: String,
    pub amount: String,
    #[serde(default)]
    pub fee: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
    #[serde(default)]
    pub wallet_tx_id: Option<String>,
}

#[derive(Deserialize)]
struct HistoryPage {
    items: Vec<HistoryItem>,
    #[serde(default)]
    current_page: i64,
    #[serde(default)]
    total_page: i64,
}

/// Withdrawal history since `start_ms`, newest first.
///
/// Reads up to five pages so a match is not missed merely because it is not on
/// the first page.
pub async fn history_since(kcc: &KuCoinClient, start_ms: i64) -> Result<Vec<HistoryItem>, String> {
    let mut items = Vec::new();

    for page in 1..=5 {
        let endpoint = format!(
            "/api/v1/withdrawals?currency=USDT&startAt={start_ms}&currentPage={page}&pageSize=100"
        );
        let response = kcc
            .send::<KuCoinResponse<HistoryPage>>("GET", "", &endpoint)
            .await
            .map_err(|e| e.to_string())?;

        let page = response.data.ok_or_else(|| {
            format!(
                "history rejected: code={} msg={:?}",
                response.code, response.msg
            )
        })?;

        let short_page = page.items.len() < 100;
        let last_page = page.total_page > 0 && page.current_page >= page.total_page;
        items.extend(page.items);
        if short_page || last_page {
            break;
        }
    }

    Ok(items)
}

/// One history row by exchange id.
pub async fn history_by_id(kcc: &KuCoinClient, id: &str) -> Result<HistoryItem, String> {
    let endpoint = format!("/api/v1/withdrawals/{id}");
    let response = kcc
        .send::<KuCoinResponse<HistoryItem>>("GET", "", &endpoint)
        .await
        .map_err(|e| e.to_string())?;

    response.data.ok_or_else(|| {
        format!(
            "history rejected: code={} msg={:?}",
            response.code, response.msg
        )
    })
}
