//! Repay residual base-currency debt for closed short loans.

use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use kucoin::types::spot::Side;
use rust_decimal::Decimal;
use share::db::loan::MarginLoan;
use sqlx::PgPool;
use uuid::Uuid;

use crate::kucoin_margin;
use crate::symbol::margin_symbol;
use crate::trader::{OrderIntent, margin_market_order};

const POLL_SECS: u64 = 30;

/// Residual interest in the base currency.
pub(crate) fn residual(outstanding: Decimal, principal: Decimal) -> Decimal {
    (outstanding - principal).max(Decimal::ZERO)
}

/// Return `(cost_base, repay_amount)`.
///
/// `account_interest_if_only` is `Some` when this is the only unrepaid loan for
/// the currency; the exchange's liability is then authoritative.
pub(crate) fn split(
    tracked: Decimal,
    account_interest_if_only: Option<Decimal>,
    account_liability: Decimal,
) -> (Decimal, Decimal) {
    match account_interest_if_only {
        Some(actual) => (tracked.max(actual), account_liability),
        None => (tracked, tracked),
    }
}

/// Background loop.
pub async fn run(pool: PgPool, kcc: KuCoinClient) {
    loop {
        if let Err(e) = repay_pending(&pool, &kcc, None).await {
            log::error!("[REPAY] pass failed: {e}");
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS)).await;
    }
}

/// Repay every closed, unrepaid loan (optionally scoped to one contract).
pub async fn repay_pending(
    pool: &PgPool,
    kcc: &KuCoinClient,
    contract_id: Option<Uuid>,
) -> Result<(), String> {
    let loans = match contract_id {
        Some(id) => MarginLoan::unrepaid_for_contract(pool, id).await,
        None => MarginLoan::all_unrepaid(pool).await,
    }
    .map_err(|e| e.to_string())?;

    for loan in loans {
        if loan.position_closed_at.is_none() {
            continue;
        }
        if let Err(e) = repay_loan(pool, kcc, &loan).await {
            log::warn!("[REPAY] loan {} pending: {e}", loan.id);
        }
    }
    Ok(())
}

async fn repay_loan(pool: &PgPool, kcc: &KuCoinClient, loan: &MarginLoan) -> Result<(), String> {
    let Some(claimed) = MarginLoan::claim_for_repay(pool, loan.id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };

    let result = repay_claimed(pool, kcc, &claimed).await;
    if result.is_err() {
        let _ = MarginLoan::release_claim(pool, claimed.id).await;
    }
    result
}

async fn repay_claimed(pool: &PgPool, kcc: &KuCoinClient, loan: &MarginLoan) -> Result<(), String> {
    let account = kucoin_margin::account(kcc).await?;
    if account.status.as_deref() != Some("EFFECTIVE") {
        return Err(format!("account status {:?}", account.status));
    }
    let asset = account
        .accounts
        .iter()
        .find(|a| a.currency == loan.currency);

    let tracked = residual(loan.outstanding, loan.principal);
    let only = MarginLoan::unrepaid_count_for_currency(pool, &loan.currency)
        .await
        .map_err(|e| e.to_string())?
        == 1;
    let actual_interest = if only {
        asset
            .and_then(|a| a.liability_interest.as_deref())
            .and_then(|v| v.parse().ok())
    } else {
        None
    };
    let liability = asset
        .and_then(|a| a.liability.parse().ok())
        .unwrap_or(Decimal::ZERO);
    let (cost_base, repay_amount) = split(tracked, actual_interest, liability);
    let price = kucoin_margin::last_price(kcc, &loan.currency).await?;
    let cost_usdt = cost_base * price;

    if repay_amount > Decimal::ZERO {
        let available = asset
            .and_then(|a| a.available.parse().ok())
            .unwrap_or(Decimal::ZERO);
        if available < repay_amount {
            ensure_base(pool, kcc, loan, repay_amount - available).await?;
        }
    }

    let repay_order_no = if repay_amount > Decimal::ZERO {
        Some(
            kucoin_margin::repay(kcc, &loan.currency, repay_amount.normalize().to_string())
                .await?
                .order_no,
        )
    } else {
        None
    };

    MarginLoan::mark_repaid(
        pool,
        loan.id,
        cost_usdt,
        loan.repay_buy_order_id.as_deref(),
        repay_order_no.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Place a market buy for the base shortfall, or wait for a previous one.
///
/// Returns `Ok(())` only when the base should be available now. A freshly
/// placed or still-working buy returns `Err`, so the pass releases its claim and
/// retries after the order fills.
async fn ensure_base(
    pool: &PgPool,
    kcc: &KuCoinClient,
    loan: &MarginLoan,
    shortfall: Decimal,
) -> Result<(), String> {
    let symbol = margin_symbol(&format!("{}/USDT", loan.currency));
    if let Some(id) = &loan.repay_buy_order_id {
        let order = kcc
            .margin()
            .get_order(id, &symbol)
            .await
            .map_err(|e| e.to_string())?;
        match order.data {
            Some(d) if d.active => {
                return Err("waiting for shortfall buy to fill".to_string());
            }
            Some(d) => {
                let filled: Decimal = d.deal_size.parse().unwrap_or(Decimal::ZERO);
                if filled <= Decimal::ZERO {
                    MarginLoan::clear_repay_buy(pool, loan.id)
                        .await
                        .map_err(|e| e.to_string())?;
                    return Err("shortfall buy did not fill; retrying".to_string());
                }
                return Ok(());
            }
            None => {}
        }
    }

    let info = kcc
        .market()
        .get_symbol(&symbol)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("symbol {symbol} not found"))?;
    let qty = round_up(shortfall, &info.base_increment, &info.base_min_size)
        .ok_or_else(|| format!("shortfall {shortfall} below minimum for {symbol}"))?;
    let request = margin_market_order(
        &symbol,
        Side::Buy,
        qty,
        &Uuid::new_v4().to_string(),
        OrderIntent::Repay,
    );
    let res = kcc
        .margin()
        .place_order(request)
        .await
        .map_err(|e| e.to_string())?;
    let data = res
        .data
        .ok_or_else(|| format!("buy rejected: {}", res.code))?;
    MarginLoan::set_repay_buy(pool, loan.id, &data.order_id)
        .await
        .map_err(|e| e.to_string())?;
    Err(format!(
        "shortfall buy {} placed; retry next pass",
        data.order_id
    ))
}

/// Round `amount` up to `increment`, respecting `min_size`.
pub(crate) fn round_up(amount: Decimal, increment: &str, min_size: &str) -> Option<String> {
    let inc: Decimal = increment.parse().ok()?;
    if inc <= Decimal::ZERO {
        return None;
    }
    let steps = (amount / inc).ceil();
    let qty = (steps * inc).max(min_size.parse().ok()?);
    Some(qty.normalize().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn residual_is_interest_only() {
        assert_eq!(residual(dec("100.5"), dec("100")), dec("0.5"));
    }

    #[test]
    fn residual_cannot_go_negative() {
        assert_eq!(residual(dec("100"), dec("100")), Decimal::ZERO);
    }

    #[test]
    fn only_loan_uses_account_liability() {
        let (cost, repay) = split(dec("2"), Some(dec("3")), dec("4"));
        assert_eq!(cost, dec("3"));
        assert_eq!(repay, dec("4"));
    }

    #[test]
    fn pooled_loan_uses_tracked_interest() {
        let (cost, repay) = split(dec("2"), None, dec("999"));
        assert_eq!(cost, dec("2"));
        assert_eq!(repay, dec("2"));
    }

    #[test]
    fn rounds_up_to_the_increment() {
        assert_eq!(
            round_up(dec("1.001"), "0.01", "0.01").as_deref(),
            Some("1.01")
        );
    }

    #[test]
    fn respects_minimum_size() {
        assert_eq!(
            round_up(dec("0.001"), "0.01", "0.5").as_deref(),
            Some("0.5")
        );
    }
}
