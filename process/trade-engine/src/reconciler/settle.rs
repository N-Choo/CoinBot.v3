//! Settle orders the exchange reports as final.
//!
//! Reads each open order's exchange state and applies the consequence to the
//! local ledger: mark the order `filled`/`cancelled`, and credit the contract
//! with the realized PnL (close) or release its reserved fund (cancelled entry).

use kucoin::client::rest::KuCoinClient;
use kucoin::types::margin::MarginOrderStatus;
use rust_decimal::Decimal;
use share::db::contracts::Contracts;
use share::db::trade::{Status, TradeOrder, TradeOrderFilter, TradeOrders};
use sqlx::{Pool, Postgres};

use super::exchange::{OrderOutcome, classify_order, fetch_order};
use super::pnl::{cap_loss, close_credit, fee_in_quote, realized_pnl, split_symbol};
use crate::symbol::margin_symbol;

/// Settle every order the exchange reports as open.
pub(super) async fn settle_open_orders(db: &Pool<Postgres>, kcc: &KuCoinClient) {
    let orders = match TradeOrderFilter::new()
        .with_status(Status::Open)
        .execute(db)
        .await
    {
        Ok(orders) => orders,
        Err(e) => {
            log::error!("[RECONCILE] open-orders query failed: {e}");
            return;
        }
    };

    for o in orders {
        let Some(order_id) = o.exchange_order_id.as_deref() else {
            continue;
        };
        let symbol = margin_symbol(&o.ticker);

        match fetch_order(kcc, order_id, &symbol).await {
            Ok(Some(data)) => match classify_order(data.active, &data.deal_size) {
                OrderOutcome::Working => continue,
                OrderOutcome::Cancelled => settle_cancelled(db, &o).await,
                OrderOutcome::Filled => settle_filled(db, &o, &symbol, &data).await,
            },
            Ok(None) => continue,
            Err(e) => {
                log::warn!("[RECONCILE] get_order failed for order {}: {e}", o.id);
                continue;
            }
        }
    }
}

/// Settle an order that finished with no fill.
async fn settle_cancelled(db: &Pool<Postgres>, o: &TradeOrder) {
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("[RECONCILE] could not begin tx for order {}: {e}", o.id);
            return;
        }
    };
    if let Err(e) = TradeOrders::mark_cancelled(&mut tx, o.id, "no fill (cancelled)").await {
        log::error!("[RECONCILE] mark_cancelled failed for order {}: {e}", o.id);
        return;
    }
    // Only an entry reserved funds at placement; a cancelled exit returns nothing.
    if o.action == "open"
        && let Err(e) = Contracts::release_fund(&mut tx, o.contract_id, o.funds).await
    {
        log::error!(
            "[RECONCILE] release_fund failed for contract {}: {e}",
            o.contract_id
        );
        return;
    }
    if let Err(e) = tx.commit().await {
        log::error!("[RECONCILE] commit failed for order {}: {e}", o.id);
    }
}

/// Settle a filled order. Amounts are parsed strictly; a malformed field aborts
/// the settle (retried next cycle) rather than committing a bogus 0 credit.
async fn settle_filled(
    db: &Pool<Postgres>,
    o: &TradeOrder,
    symbol: &str,
    data: &MarginOrderStatus,
) {
    let (Ok(deal_size), Ok(deal_funds), Ok(fee)) = (
        data.deal_size.parse::<Decimal>(),
        data.deal_funds.parse::<Decimal>(),
        data.fee.parse::<Decimal>(),
    ) else {
        log::warn!(
            "[RECONCILE] order {} has unparseable amounts; retrying",
            o.id
        );
        return;
    };
    let fill_price = if deal_size.is_zero() {
        Decimal::ZERO
    } else {
        deal_funds / deal_size
    };
    let (base, quote) = split_symbol(symbol);
    // Fee for THIS (exit) leg, expressed in the contract's quote currency.
    let exit_fee_q = fee_in_quote(fee, data.fee_currency.as_deref(), &base, &quote, fill_price);

    // For a close, derive realized PnL from the parent entry: its actual
    // transacted basis (filled_qty * fill_price), its fee, and this leg's fee.
    // Computed before opening the tx so the parent lookup does not borrow a
    // second pool connection while the tx holds one.
    let (credit, pnl) = if o.action == "close" {
        match o.parent_order_id {
            None => (deal_funds, None),
            Some(pid) => {
                let entry = match TradeOrders::find(db, pid).await {
                    Ok(entry) => entry,
                    Err(e) => {
                        log::error!("[RECONCILE] parent lookup failed for close {}: {e}", o.id);
                        return; // retry next cycle
                    }
                };
                let entry_price = entry.fill_price.unwrap_or(Decimal::ZERO);
                // Prefer the entry's real fill for the cost basis; fall back to
                // the committed fund (never a silent zero) when it is missing.
                let entry_basis = if !entry.filled_qty.is_zero() && entry.fill_price.is_some() {
                    entry.filled_qty * entry_price
                } else {
                    log::warn!(
                        "[RECONCILE] parent entry {} lacks a real fill; using committed fund as basis",
                        entry.id
                    );
                    entry.funds
                };
                let entry_fee_q = fee_in_quote(
                    entry.fee,
                    entry.fee_currency.as_deref(),
                    &base,
                    &quote,
                    entry_price,
                );
                let raw_pnl = realized_pnl(
                    &entry.side,
                    entry_basis,
                    deal_funds,
                    entry_fee_q + exit_fee_q,
                );
                let pnl = cap_loss(raw_pnl, entry.funds);
                if pnl != raw_pnl {
                    log::warn!(
                        "[RECONCILE] close {} raw pnl {raw_pnl} exceeds committed capital {}; capping",
                        o.id,
                        entry.funds
                    );
                }
                (close_credit(entry.funds, pnl), Some(pnl))
            }
        }
    } else {
        (Decimal::ZERO, None)
    };

    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("[RECONCILE] could not begin tx for order {}: {e}", o.id);
            return;
        }
    };
    if let Err(e) = TradeOrders::mark_filled(
        &mut tx,
        o.id,
        deal_size,
        fill_price,
        fee,
        data.fee_currency.as_deref(),
        pnl,
    )
    .await
    {
        log::error!("[RECONCILE] mark_filled failed for order {}: {e}", o.id);
        return;
    }
    // Only an exit returns cash.
    if o.action == "close"
        && let Err(e) = Contracts::add_fund(&mut tx, o.contract_id, credit.max(Decimal::ZERO)).await
    {
        log::error!(
            "[RECONCILE] add_fund failed for contract {}: {e}",
            o.contract_id
        );
        return;
    }
    if let Err(e) = tx.commit().await {
        log::error!("[RECONCILE] commit failed for order {}: {e}", o.id);
        return;
    }
    log::info!(
        "[RECONCILE] {symbol} order {} filled {} @ {fill_price} (fee {fee}, pnl {})",
        o.id,
        data.deal_size,
        pnl.map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
}
