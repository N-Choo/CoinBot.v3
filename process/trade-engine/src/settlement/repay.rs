//! Repay residual base-currency debt for closed short loans.
//!
//! Closing a short repays the borrowed principal through the close order's
//! `auto_repay`. This module settles what is left: mostly hourly, capitalised
//! interest denominated in the borrowed currency (e.g. SOL).
//!
//! # Algorithm
//!
//! 1. Resolve a shortfall buy left by an earlier pass (by exchange id or the
//!    persisted `clientOid`), and learn how much base it bought.
//! 2. Read the cross-margin account. If the base balance is short, place a
//!    market buy for the exact shortfall. The buy is only an acquisition leg;
//!    it is never treated as the repayment itself.
//! 3. Call `POST /api/v3/margin/repay` with the real liability (or the tracked
//!    interest when other loans share the currency).
//! 4. Verify `actualSize` and the remaining liability; retry a genuine partial.
//! 5. Sell any rounding/min-size surplus back to USDT so no long base remains.
//! 6. Mark the loan repaid and charge its contract's `borrow_interest` in one
//!    transaction.
//!
//! KuCoin has no clientOid on repay, so the only-loan path is bounded by the
//! account's reported liability. See the README for the pooled-account caveat.

use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use kucoin::types::spot::Side;
use rust_decimal::Decimal;
use share::db::loan::MarginLoan;
use sqlx::PgPool;
use uuid::Uuid;

use crate::kucoin_margin::{self, RepayHistoryItem};
use crate::symbol::margin_symbol;
use crate::trader::{OrderIntent, margin_market_order};

const POLL_SECS: u64 = 30;

/// A repay intent younger than this is not re-sent while its history is pending.
const REPAY_RECONCILE_WINDOW_SECS: i64 = 60;

/// Amounts at or below this are treated as dust (`1e-8`).
const DUST: Decimal = Decimal::from_parts(1, 0, 0, false, 8);

/// Residual interest in the base currency.
pub(crate) fn residual(outstanding: Decimal, principal: Decimal) -> Decimal {
    (outstanding - principal).max(Decimal::ZERO)
}

/// Return `(cost_base, repay_amount)`.
///
/// When `only` is true the exchange's liability is authoritative for the amount
/// to repay, and its interest (or `0` when no row/field exists) is the cost.
/// Otherwise use the tracked interest, which is safe against other loans.
pub(crate) fn split(
    tracked: Decimal,
    only: bool,
    account_interest: Option<Decimal>,
    account_liability: Decimal,
) -> (Decimal, Decimal) {
    if only {
        let actual = account_interest.unwrap_or(Decimal::ZERO);
        (tracked.max(actual), account_liability)
    } else {
        (tracked, tracked.min(account_liability))
    }
}

/// Parse a required exchange field, failing rather than defaulting to zero.
fn parse_required(value: Option<&str>, field: &str) -> Result<Decimal, String> {
    let value = value.ok_or_else(|| format!("missing {field}"))?;
    value.parse().map_err(|_| format!("bad {field}: {value}"))
}

/// Parse an optional exchange field; an absent field is zero.
fn parse_optional_zero(value: Option<&str>, field: &str) -> Result<Decimal, String> {
    match value {
        Some(v) => v.parse().map_err(|_| format!("bad {field}: {v}")),
        None => Ok(Decimal::ZERO),
    }
}

/// How much base to repay, how much USDT it costs, and how much base is free.
#[derive(Debug, Clone, Copy)]
struct RepayPlan {
    cost_base: Decimal,
    repay_amount: Decimal,
    available: Decimal,
}

/// Build the repay plan from the account row.
///
/// `only` means this is the sole unrepaid loan for the currency, so the
/// exchange's `liability`/`liabilityInterest` can be trusted directly.
fn build_plan(
    loan: &MarginLoan,
    asset: Option<&kucoin_margin::MarginAsset>,
    only: bool,
) -> Result<RepayPlan, String> {
    let tracked = residual(loan.outstanding, loan.principal);
    let (liability, actual_interest, available) = match asset {
        Some(a) => (
            parse_required(a.liability.as_deref(), "liability")?,
            if only {
                Some(parse_optional_zero(
                    a.liability_interest.as_deref(),
                    "liabilityInterest",
                )?)
            } else {
                None
            },
            parse_required(a.available.as_deref(), "available")?,
        ),
        None => (
            Decimal::ZERO,
            if only { Some(Decimal::ZERO) } else { None },
            Decimal::ZERO,
        ),
    };
    let (cost_base, repay_amount) = split(tracked, only, actual_interest, liability);
    Ok(RepayPlan {
        cost_base,
        repay_amount,
        available,
    })
}

/// Background loop.
pub async fn run(pool: PgPool, kcc: KuCoinClient) {
    let mut backoff = 0u64;
    loop {
        match repay_pending(&pool, &kcc, None).await {
            Ok(()) => backoff = 0,
            Err(e) => {
                log::error!("[REPAY] pass failed: {e}");
                if kucoin_margin::is_rate_limited(&e) {
                    backoff = if backoff == 0 {
                        60
                    } else {
                        (backoff * 2).min(300)
                    };
                    log::warn!("[REPAY] backing off {backoff}s");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS.max(backoff))).await;
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
            if kucoin_margin::is_rate_limited(&e) {
                return Err(e);
            }
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
    // Concurrency caveat: when several loans for a currency are repaid at once,
    // each uses its tracked interest; the final partial-hour reconciliation is
    // approximate. Accepted by the design.
    let only = MarginLoan::unrepaid_count_for_currency(pool, &loan.currency)
        .await
        .map_err(|e| e.to_string())?
        == 1;

    // Resolve a shortfall buy left by an earlier pass and learn how much base it
    // actually bought, so any rounding/min-size surplus can be sold back.
    let bought = resolve_prior_buy(pool, kcc, loan).await?;

    // A prior repay intent means the exchange call may have succeeded while the
    // DB write did not. Reconcile against the repay history before re-sending.
    if loan.repay_attempted_at.is_some() {
        if let Some(order_no) = reconcile_prior_repay(kcc, loan).await? {
            let cost_usdt = match loan.repay_attempted_cost_usdt {
                Some(cost) => cost,
                None => {
                    let price = kucoin_margin::last_price(kcc, &loan.currency).await?;
                    residual(loan.outstanding, loan.principal) * price
                }
            };
            cleanup_after_reconcile(kcc, loan, bought).await;
            MarginLoan::mark_repaid(
                pool,
                loan.id,
                cost_usdt,
                loan.repay_buy_order_id.as_deref(),
                Some(&order_no),
            )
            .await
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        let age = chrono::Utc::now()
            .signed_duration_since(loan.repay_attempted_at.unwrap_or_else(chrono::Utc::now));
        if age.num_seconds() < REPAY_RECONCILE_WINDOW_SECS {
            return Err("waiting for repay history to catch up".to_string());
        }
        MarginLoan::clear_repay_attempt(pool, loan.id)
            .await
            .map_err(|e| e.to_string())?;
    }

    let account = kucoin_margin::account(kcc).await?;
    if account.status.as_deref() != Some("EFFECTIVE") {
        return Err(format!("account status {:?}", account.status));
    }
    let asset = account
        .accounts
        .iter()
        .find(|a| a.currency == loan.currency);
    if asset.is_none() && residual(loan.outstanding, loan.principal) > DUST {
        log::warn!(
            "[REPAY] account has no row for {}; treating its liability as zero",
            loan.currency
        );
    }
    let pending = build_plan(loan, asset, only)?;
    let available_before_buy = (pending.available - bought).max(Decimal::ZERO);

    if pending.repay_amount > Decimal::ZERO && pending.available < pending.repay_amount {
        // The buy is only an acquisition leg; the explicit repay happens after
        // it fills on the next pass.
        place_shortfall_buy(pool, kcc, loan, pending.repay_amount - pending.available).await?;
    }

    let price = kucoin_margin::last_price(kcc, &loan.currency).await?;
    let cost_usdt = pending.cost_base * price;

    let repay_order_no = if pending.repay_amount > Decimal::ZERO {
        // Persist the intent before the exchange call. If the call succeeds but
        // `mark_repaid` fails, the next pass reconciles it instead of paying
        // twice.
        MarginLoan::set_repay_attempt(pool, loan.id, pending.repay_amount, cost_usdt)
            .await
            .map_err(|e| e.to_string())?;
        match submit_repay(kcc, loan, pending.repay_amount).await {
            Ok(receipt) => {
                let buy = BuyState {
                    bought,
                    available_before: available_before_buy,
                    repaid: receipt.actual,
                };
                if let Err(e) = cleanup_surplus(kcc, loan, buy).await {
                    log::warn!("[REPAY] surplus cleanup for loan {} failed: {e}", loan.id);
                }
                Some(receipt.order_no)
            }
            // A sub-minimum repay can be rejected by the exchange even though the
            // base buy succeeded. Absorbing the dust is safer than retrying
            // forever. Transport errors still retry.
            Err(e) => {
                let min_size = base_min_size(kcc, &loan.currency)
                    .await
                    .unwrap_or(Decimal::ZERO);
                if pending.repay_amount < min_size && e.starts_with("kucoin margin error") {
                    log::warn!(
                        "[REPAY] repay of {} {} is below the exchange minimum; absorbing as platform cost",
                        pending.repay_amount,
                        loan.currency
                    );
                    let buy = BuyState {
                        bought,
                        available_before: available_before_buy,
                        repaid: Decimal::ZERO,
                    };
                    if let Err(e) = cleanup_surplus(kcc, loan, buy).await {
                        log::warn!("[REPAY] surplus cleanup for loan {} failed: {e}", loan.id);
                    }
                    None
                } else {
                    return Err(e);
                }
            }
        }
    } else {
        let buy = BuyState {
            bought,
            available_before: available_before_buy,
            repaid: Decimal::ZERO,
        };
        if let Err(e) = cleanup_surplus(kcc, loan, buy).await {
            log::warn!("[REPAY] surplus cleanup for loan {} failed: {e}", loan.id);
        }
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

/// Check the exchange's repay history for a prior intent.
async fn reconcile_prior_repay(
    kcc: &KuCoinClient,
    loan: &MarginLoan,
) -> Result<Option<String>, String> {
    let Some(attempted) = loan.repay_attempted_at else {
        return Ok(None);
    };
    let amount = loan
        .repay_attempted_amount
        .unwrap_or_else(|| residual(loan.outstanding, loan.principal));
    if amount <= DUST {
        return Ok(None);
    }
    let start = attempted.timestamp_millis();
    let end = chrono::Utc::now().timestamp_millis();
    let page = kucoin_margin::repay_history(kcc, &loan.currency, start, end, 1).await?;
    Ok(matching_repay(&page.items, amount, start))
}

/// Find a successful repay that matches `amount` at or after `since_ms`.
pub(crate) fn matching_repay(
    items: &[RepayHistoryItem],
    amount: Decimal,
    since_ms: i64,
) -> Option<String> {
    items.iter().find_map(|item| {
        if item.created_time < since_ms {
            return None;
        }
        if item
            .status
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("FAILURE"))
        {
            return None;
        }
        let actual: Decimal = item.actual_size.as_deref()?.parse().ok()?;
        if actual + DUST >= amount {
            Some(item.order_no.clone())
        } else {
            None
        }
    })
}

/// Best-effort surplus cleanup after a reconciled repay.
async fn cleanup_after_reconcile(kcc: &KuCoinClient, loan: &MarginLoan, bought: Decimal) {
    if bought <= DUST {
        return;
    }
    let account = match kucoin_margin::account(kcc).await {
        Ok(a) => a,
        Err(e) => {
            log::warn!("[REPAY] account read for surplus cleanup failed: {e}");
            return;
        }
    };
    let available = account
        .accounts
        .iter()
        .find(|a| a.currency == loan.currency)
        .and_then(|a| a.available.as_deref())
        .and_then(|v| v.parse::<Decimal>().ok())
        .unwrap_or(Decimal::ZERO);
    let available_before = (available - bought).max(Decimal::ZERO);
    let buy = BuyState {
        bought,
        available_before,
        repaid: loan.repay_attempted_amount.unwrap_or(Decimal::ZERO),
    };
    if let Err(e) = cleanup_surplus(kcc, loan, buy).await {
        log::warn!("[REPAY] surplus cleanup for loan {} failed: {e}", loan.id);
    }
}

/// Acknowledgement of a successful repay.
struct RepayReceipt {
    order_no: String,
    actual: Decimal,
}

/// Exchange minimum order size for `currency`'s base asset.
async fn base_min_size(kcc: &KuCoinClient, currency: &str) -> Result<Decimal, String> {
    let symbol = margin_symbol(&format!("{currency}/USDT"));
    let info = kcc
        .market()
        .get_symbol(&symbol)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("symbol {symbol} not found"))?;
    info.base_min_size
        .parse()
        .map_err(|_| format!("bad baseMinSize for {symbol}"))
}

/// Submit the repay and confirm the exchange actually cleared the debt.
///
/// A successful call can report less than requested when the estimate exceeded
/// the real debt; only a genuine remaining liability is retried.
async fn submit_repay(
    kcc: &KuCoinClient,
    loan: &MarginLoan,
    amount: Decimal,
) -> Result<RepayReceipt, String> {
    let data = kucoin_margin::repay(kcc, &loan.currency, amount.normalize().to_string()).await?;
    let actual: Decimal = data
        .actual_size
        .parse()
        .map_err(|_| format!("bad actualSize: {}", data.actual_size))?;
    if actual < amount {
        let remaining = remaining_liability(kcc, &loan.currency).await?;
        if remaining > DUST {
            return Err(format!(
                "partial repay {actual}/{amount} ({remaining} remains); retrying"
            ));
        }
    }
    Ok(RepayReceipt {
        order_no: data.order_no,
        actual,
    })
}

/// Remaining base liability for `currency`, or zero when the account omits the
/// row (KuCoin commonly omits zero-liability currencies).
async fn remaining_liability(kcc: &KuCoinClient, currency: &str) -> Result<Decimal, String> {
    let account = kucoin_margin::account(kcc).await?;
    match account
        .accounts
        .iter()
        .find(|a| a.currency == currency)
        .and_then(|a| a.liability.as_deref())
    {
        Some(v) => v.parse().map_err(|_| format!("bad liability: {v}")),
        None => Ok(Decimal::ZERO),
    }
}

/// Resolve a buy left by a previous pass. Returns its filled base quantity, or
/// `0` when there was no prior buy. An active buy is an error (wait for it).
async fn resolve_prior_buy(
    pool: &PgPool,
    kcc: &KuCoinClient,
    loan: &MarginLoan,
) -> Result<Decimal, String> {
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
                MarginLoan::clear_repay_buy(pool, loan.id)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(filled);
            }
            None => {}
        }
    }

    // Recover an ambiguous placement by its persisted clientOid.
    if let Some(client_oid) = &loan.repay_buy_client_oid {
        match kcc
            .margin()
            .get_order_by_client_oid(client_oid, &symbol)
            .await
        {
            Ok(res) => {
                if let Some(data) = res.data {
                    MarginLoan::set_repay_buy(pool, loan.id, &data.id, client_oid)
                        .await
                        .map_err(|e| e.to_string())?;
                    return Err("recovered shortfall buy; waiting".to_string());
                }
                MarginLoan::clear_repay_buy(pool, loan.id)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(e) if e.is_not_found() => {
                MarginLoan::clear_repay_buy(pool, loan.id)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(Decimal::ZERO)
}

/// Place a market buy for the base shortfall. The clientOid is persisted before
/// the exchange call so an ambiguous response can be recovered next pass.
///
/// Always returns an error: either a placed buy to wait for, or a rejection.
async fn place_shortfall_buy(
    pool: &PgPool,
    kcc: &KuCoinClient,
    loan: &MarginLoan,
    shortfall: Decimal,
) -> Result<(), String> {
    let symbol = margin_symbol(&format!("{}/USDT", loan.currency));
    let info = kcc
        .market()
        .get_symbol(&symbol)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("symbol {symbol} not found"))?;
    // Round up to at least the exchange minimum. The over-buy is sold back by
    // `cleanup_surplus` after the repay, so sub-minimum debt is still cleared.
    let qty = round_up(shortfall, &info.base_increment, &info.base_min_size)
        .ok_or_else(|| format!("shortfall {shortfall} below minimum for {symbol}"))?;

    let client_oid = Uuid::new_v4().to_string();
    MarginLoan::set_repay_buy_client(pool, loan.id, &client_oid)
        .await
        .map_err(|e| e.to_string())?;
    let request = margin_market_order(&symbol, Side::Buy, qty, &client_oid, OrderIntent::Repay);
    let res = kcc
        .margin()
        .place_order(request)
        .await
        .map_err(|e| format!("shortfall buy ambiguous: {e}"))?;
    let data = res
        .data
        .ok_or_else(|| format!("buy rejected: {}", res.code))?;
    MarginLoan::set_repay_buy(pool, loan.id, &data.order_id, &client_oid)
        .await
        .map_err(|e| e.to_string())?;
    Err(format!(
        "shortfall buy {} placed; retry next pass",
        data.order_id
    ))
}

/// What a shortfall buy did, used to compute any over-buy.
#[derive(Debug, Clone, Copy)]
struct BuyState {
    /// Base actually acquired by the shortfall buy.
    bought: Decimal,
    /// Free base before that buy.
    available_before: Decimal,
    /// Base actually consumed by the repay.
    repaid: Decimal,
}

impl BuyState {
    /// Base bought but not consumed by the repay.
    fn surplus(&self) -> Decimal {
        surplus_qty(self.bought, self.available_before, self.repaid)
    }
}

/// Base bought for a shortfall that the repay did not consume.
pub(crate) fn surplus_qty(
    bought: Decimal,
    available_before_buy: Decimal,
    repaid: Decimal,
) -> Decimal {
    let from_buy = (repaid - available_before_buy).max(Decimal::ZERO);
    (bought - from_buy).max(Decimal::ZERO)
}

/// Sell a rounding/min-size surplus back to USDT so no long base is left.
async fn cleanup_surplus(
    kcc: &KuCoinClient,
    loan: &MarginLoan,
    buy: BuyState,
) -> Result<(), String> {
    let surplus = buy.surplus();
    if surplus <= DUST {
        return Ok(());
    }
    let account = kucoin_margin::account(kcc).await?;
    let available = account
        .accounts
        .iter()
        .find(|a| a.currency == loan.currency)
        .and_then(|a| a.available.as_deref())
        .and_then(|v| v.parse::<Decimal>().ok())
        .unwrap_or(Decimal::ZERO);
    let amount = surplus.min(available);
    if amount <= DUST {
        return Ok(());
    }

    let symbol = margin_symbol(&format!("{}/USDT", loan.currency));
    let info = kcc
        .market()
        .get_symbol(&symbol)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("symbol {symbol} not found"))?;
    let Some(qty) = floor_qty(amount, &info.base_increment, &info.base_min_size) else {
        log::warn!(
            "[REPAY] surplus {amount} {} for loan {} is below the sell minimum; retained by the platform",
            loan.currency,
            loan.id
        );
        return Ok(());
    };
    let client_oid = Uuid::new_v4().to_string();
    let request = margin_market_order(
        &symbol,
        Side::Sell,
        qty.clone(),
        &client_oid,
        OrderIntent::Repay,
    );
    let res = kcc
        .margin()
        .place_order(request)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(data) = res.data {
        log::info!(
            "[REPAY] sold surplus {} {} for loan {} (oid {})",
            qty,
            loan.currency,
            loan.id,
            data.order_id
        );
    } else {
        log::warn!("[REPAY] surplus sell rejected: code={}", res.code);
    }
    Ok(())
}

/// Floor `amount` down to `increment` and require `min_size`.
pub(crate) fn floor_qty(amount: Decimal, increment: &str, min_size: &str) -> Option<String> {
    let inc: Decimal = increment.parse().ok()?;
    if inc <= Decimal::ZERO {
        return None;
    }
    let steps = (amount / inc).floor();
    if steps < Decimal::ONE {
        return None;
    }
    let qty = steps * inc;
    if qty < min_size.parse().unwrap_or(Decimal::ZERO) {
        return None;
    }
    Some(qty.normalize().to_string())
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
        let (cost, repay) = split(dec("2"), true, Some(dec("3")), dec("4"));
        assert_eq!(cost, dec("3"));
        assert_eq!(repay, dec("4"));
    }

    #[test]
    fn only_loan_without_interest_row_repays_only_liability() {
        let (cost, repay) = split(dec("2"), true, None, dec("0"));
        assert_eq!(cost, dec("2"));
        assert_eq!(repay, Decimal::ZERO);
    }

    #[test]
    fn pooled_loan_uses_tracked_interest() {
        let (cost, repay) = split(dec("2"), false, None, dec("999"));
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

    #[test]
    fn required_fields_must_be_present_and_numeric() {
        assert!(parse_required(None, "liability").is_err());
        assert!(parse_required(Some("not-a-number"), "liability").is_err());
        assert_eq!(parse_required(Some("1"), "liability").unwrap(), dec("1"));
    }

    #[test]
    fn optional_interest_defaults_to_zero() {
        assert_eq!(
            parse_optional_zero(None, "liabilityInterest").unwrap(),
            Decimal::ZERO
        );
        assert_eq!(
            parse_optional_zero(Some("0.25"), "liabilityInterest").unwrap(),
            dec("0.25")
        );
    }

    #[test]
    fn buy_state_reports_the_over_buy() {
        let buy = BuyState {
            bought: dec("1.1"),
            available_before: dec("0.5"),
            repaid: dec("1"),
        };
        assert_eq!(buy.surplus(), dec("0.6"));
    }

    #[test]
    fn surplus_is_bought_minus_used_from_buy() {
        assert_eq!(surplus_qty(dec("1.1"), dec("0.5"), dec("1")), dec("0.6"));
        assert_eq!(surplus_qty(dec("1"), dec("2"), dec("1")), dec("1"));
        assert_eq!(surplus_qty(dec("1"), dec("0"), dec("1")), Decimal::ZERO);
    }

    fn repay_item(
        order_no: &str,
        actual: &str,
        created_time: i64,
        status: Option<&str>,
    ) -> RepayHistoryItem {
        RepayHistoryItem {
            order_no: order_no.to_string(),
            currency: "SOL".to_string(),
            size: Some(actual.to_string()),
            actual_size: Some(actual.to_string()),
            status: status.map(str::to_string),
            created_time,
        }
    }

    #[test]
    fn matching_repay_ignores_old_and_failed_rows() {
        let items = vec![
            repay_item("old", "1", 10, Some("SUCCESS")),
            repay_item("failed", "1", 20, Some("FAILURE")),
            repay_item("good", "1", 20, Some("SUCCESS")),
        ];
        assert_eq!(
            matching_repay(&items, dec("1"), 20).as_deref(),
            Some("good")
        );
    }

    #[test]
    fn matching_repay_requires_the_full_amount() {
        let items = vec![repay_item("partial", "0.5", 20, Some("SUCCESS"))];
        assert_eq!(matching_repay(&items, dec("1"), 20), None);
    }

    #[test]
    fn floors_surplus_to_the_increment() {
        assert_eq!(
            floor_qty(dec("0.019"), "0.01", "0.01").as_deref(),
            Some("0.01")
        );
        assert_eq!(floor_qty(dec("0.001"), "0.01", "0.01"), None);
        assert_eq!(floor_qty(dec("0.5"), "0.1", "1"), None);
    }
}
