//! Background sweeper: reconcile pending/open orders against the exchange and
//! settle their contracts.

use chrono::{Duration, Utc};
use kucoin::client::rest::KuCoinClient;
use kucoin::types::margin::{MarginOrderId, MarginOrderStatus};
use rust_decimal::Decimal;
use share::ProcessError;
use share::db::contracts::Contracts;
use share::db::trade::{TradeOrder, TradeOrderFilter, TradeOrders};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

/// Grace before a `pending` order with no exchange id is treated as unplaced.
const PENDING_GRACE_MINUTES: i64 = 5;
/// Hard cap: even if lookups keep erroring (e.g. a 404 surfacing as an error),
/// give up on a `pending` order past this age so a position is never blocked.
const PENDING_MAX_MINUTES: i64 = 30;

/// Reconcile in-flight orders against the exchange and settle their contracts.
pub async fn sync_contract_status(db: Pool<Postgres>, kcc: KuCoinClient) {
    // First resolve placements whose outcome is unknown (crash, transport error),
    // then settle every order the exchange reports as open.
    reconcile_pending(&db, &kcc).await;
    settle_open_orders(&db, &kcc).await;
}

/// Resolve `pending` orders (no exchange id) by looking them up via their
/// clientOid: found => record the exchange id; absent past the grace window =>
/// mark failed and release any reserved entry fund.
async fn reconcile_pending(db: &Pool<Postgres>, kcc: &KuCoinClient) {
    let pending = match TradeOrders::pending_orders(db).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("[SWEEP] pending-orders query failed: {e}");
            return;
        }
    };

    for o in pending {
        let symbol = margin_symbol(&o.ticker);
        match fetch_by_client_oid(kcc, &o.comment, &symbol).await {
            Ok(Some(data)) => {
                if let Err(e) = place_pending(db, o.id, &data.id).await {
                    log::error!("[SWEEP] mark_placed failed for pending {}: {e}", o.id);
                }
            }
            Ok(None) => {
                if o.created_at < Utc::now() - Duration::minutes(PENDING_GRACE_MINUTES) {
                    fail_pending(db, &o).await;
                }
            }
            Err(e) => {
                log::warn!("[SWEEP] client-oid lookup failed for {}: {e}", o.id);
                if o.created_at < Utc::now() - Duration::minutes(PENDING_MAX_MINUTES) {
                    fail_pending(db, &o).await;
                }
            }
        }
    }
}

/// Record a reserved order's exchange id once the exchange confirms it exists.
async fn place_pending(db: &Pool<Postgres>, id: Uuid, order_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    TradeOrders::mark_placed(&mut tx, id, order_id).await?;
    tx.commit().await
}

/// Mark a never-placed order failed, then (separately) return a reserved entry
/// fund. Marking failed in its own transaction first means a release error can
/// never wedge the row in `pending` forever.
async fn fail_pending(db: &Pool<Postgres>, o: &TradeOrder) {
    match db.begin().await {
        Ok(mut tx) => {
            if let Err(e) = TradeOrders::mark_failed(&mut tx, o.id, "not placed (reconciled)").await
            {
                log::error!("[SWEEP] mark_failed failed for pending {}: {e}", o.id);
                return;
            }
            if let Err(e) = tx.commit().await {
                log::error!("[SWEEP] commit failed for pending {}: {e}", o.id);
                return;
            }
        }
        Err(e) => {
            log::error!("[SWEEP] could not begin tx for pending {}: {e}", o.id);
            return;
        }
    }

    if o.action == "open" {
        match db.begin().await {
            Ok(mut tx) => {
                if let Err(e) = Contracts::release_fund(&mut tx, o.contract_id, o.funds).await {
                    log::error!(
                        "[SWEEP] release_fund failed for contract {}: {e}",
                        o.contract_id
                    );
                    return;
                }
                if let Err(e) = tx.commit().await {
                    log::error!("[SWEEP] commit failed releasing fund for {}: {e}", o.id);
                }
            }
            Err(e) => log::error!("[SWEEP] could not begin release tx for {}: {e}", o.id),
        }
    }
}

/// Settle every order the exchange reports as open.
async fn settle_open_orders(db: &Pool<Postgres>, kcc: &KuCoinClient) {
    let orders = match TradeOrderFilter::new()
        .with_status(share::db::trade::Status::Open)
        .execute(db)
        .await
    {
        Ok(orders) => orders,
        Err(e) => {
            log::error!("[SWEEP] open-orders query failed: {e}");
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
                log::warn!("[SWEEP] get_order failed for order {}: {e}", o.id);
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
            log::error!("[SWEEP] could not begin tx for order {}: {e}", o.id);
            return;
        }
    };
    if let Err(e) = TradeOrders::mark_cancelled(&mut tx, o.id, "no fill (cancelled)").await {
        log::error!("[SWEEP] mark_cancelled failed for order {}: {e}", o.id);
        return;
    }
    // Only an entry reserved funds at placement; a cancelled exit returns nothing.
    if o.action == "open"
        && let Err(e) = Contracts::release_fund(&mut tx, o.contract_id, o.funds).await
    {
        log::error!(
            "[SWEEP] release_fund failed for contract {}: {e}",
            o.contract_id
        );
        return;
    }
    if let Err(e) = tx.commit().await {
        log::error!("[SWEEP] commit failed for order {}: {e}", o.id);
    }
}

/// Credit returned to a contract when a close order fills.
///
/// A long committed `entry_funds` (`E`) to buy the asset, so closing returns the
/// sale proceeds (`deal_funds`, `C`). A short committed `E` as collateral and
/// profited by `E - C`, so closing returns `E + (E - C) = 2*E - C`.
fn close_credit(entry_side: &str, entry_funds: Decimal, deal_funds: Decimal) -> Decimal {
    match entry_side {
        "sell" => Decimal::TWO * entry_funds - deal_funds,
        _ => deal_funds,
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
        log::warn!("[SWEEP] order {} has unparseable amounts; retrying", o.id);
        return;
    };
    let fill_price = if deal_size.is_zero() {
        Decimal::ZERO
    } else {
        deal_funds / deal_size
    };

    // Compute the exit credit before opening the transaction, so the parent
    // lookup does not borrow a second pool connection while the tx holds one.
    let credit = if o.action == "close" {
        match o.parent_order_id {
            Some(pid) => match TradeOrders::find(db, pid).await {
                Ok(entry) => close_credit(&entry.side, entry.funds, deal_funds),
                Err(e) => {
                    log::error!("[SWEEP] parent lookup failed for close {}: {e}", o.id);
                    return; // retry next cycle
                }
            },
            None => deal_funds,
        }
    } else {
        Decimal::ZERO
    };
    if credit < Decimal::ZERO {
        log::warn!(
            "[SWEEP] close {} credit {credit} < 0 (loss exceeds committed capital); crediting 0",
            o.id
        );
    }

    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("[SWEEP] could not begin tx for order {}: {e}", o.id);
            return;
        }
    };
    if let Err(e) = TradeOrders::mark_filled(&mut tx, o.id, deal_size, fill_price, fee).await {
        log::error!("[SWEEP] mark_filled failed for order {}: {e}", o.id);
        return;
    }
    // Only an exit returns cash.
    if o.action == "close"
        && let Err(e) = Contracts::add_fund(&mut tx, o.contract_id, credit.max(Decimal::ZERO)).await
    {
        log::error!(
            "[SWEEP] add_fund failed for contract {}: {e}",
            o.contract_id
        );
        return;
    }
    if let Err(e) = tx.commit().await {
        log::error!("[SWEEP] commit failed for order {}: {e}", o.id);
        return;
    }
    log::info!(
        "[SWEEP] {symbol} order {} filled {} @ {fill_price} (fee {fee})",
        o.id,
        data.deal_size
    );
}

/// Fetch an order by its exchange id via the margin HF endpoint.
async fn fetch_order(
    kcc: &KuCoinClient,
    order_id: &str,
    symbol: &str,
) -> Result<Option<MarginOrderStatus>, ProcessError> {
    let res = kcc
        .margin()
        .get_order(order_id, symbol)
        .await
        .map_err(|e| ProcessError::Network(format!("get_order failed: {e}")))?;
    Ok(res.data)
}

/// Fetch an order by the clientOid we assigned (`comment`). A 404 means the
/// order was never placed, which we surface as `Ok(None)` (not an error) so the
/// short grace window applies. KuCoin returns this for a missing margin order
/// (`code:126043`, HTTP 404).
async fn fetch_by_client_oid(
    kcc: &KuCoinClient,
    client_oid: &str,
    symbol: &str,
) -> Result<Option<MarginOrderId>, ProcessError> {
    match kcc
        .margin()
        .get_order_by_client_oid(client_oid, symbol)
        .await
    {
        Ok(res) => Ok(res.data),
        Err(e) if e.is_not_found() => Ok(None),
        Err(e) => Err(ProcessError::Network(format!(
            "client-order lookup failed: {e}"
        ))),
    }
}

/// KuCoin margin symbol: `"BTC/USDT"` -> `"BTC-USDT"`.
fn margin_symbol(ticker: &str) -> String {
    ticker.replace('/', "-")
}

/// Outcome of reconciling one exchange order.
#[derive(Debug, PartialEq)]
enum OrderOutcome {
    /// Still working (or unparseable, retried) on the exchange.
    Working,
    /// Finished with no fill (cancelled).
    Cancelled,
    /// Filled.
    Filled,
}

/// Classify an exchange order from its `active` flag and dealt size.
///
/// An unparseable dealt size is treated as `Working` so a decode hiccup retries
/// rather than mis-cancelling a real fill.
fn classify_order(active: bool, deal_size: &str) -> OrderOutcome {
    if active {
        return OrderOutcome::Working;
    }
    match deal_size.parse::<Decimal>() {
        Ok(v) if v > Decimal::ZERO => OrderOutcome::Filled,
        Ok(_) => OrderOutcome::Cancelled,
        Err(_) => OrderOutcome::Working,
    }
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
    fn close_credit_is_proceeds_for_a_long() {
        assert_eq!(close_credit("buy", dec("100"), dec("120")), dec("120"));
        assert_eq!(close_credit("buy", dec("100"), dec("80")), dec("80"));
    }

    #[test]
    fn close_credit_is_collateral_plus_short_pnl() {
        assert_eq!(close_credit("sell", dec("100"), dec("80")), dec("120"));
        assert_eq!(close_credit("sell", dec("100"), dec("120")), dec("80"));
    }

    #[test]
    fn close_credit_can_go_negative_when_a_short_loses_more_than_capital() {
        // Short: commit 100, cover for 250 -> 2*100 - 250 = -50 (floor to 0 at call site).
        assert_eq!(close_credit("sell", dec("100"), dec("250")), dec("-50"));
    }

    #[test]
    fn builds_margin_symbol() {
        assert_eq!(margin_symbol("BTC/USDT"), "BTC-USDT");
        assert_eq!(margin_symbol("BTC-USDT"), "BTC-USDT");
    }

    #[test]
    fn classifies_working_order() {
        assert_eq!(classify_order(true, "0"), OrderOutcome::Working);
        assert_eq!(classify_order(true, "5"), OrderOutcome::Working);
    }

    #[test]
    fn classifies_unfilled_as_cancelled() {
        assert_eq!(classify_order(false, "0"), OrderOutcome::Cancelled);
        assert_eq!(classify_order(false, "-1"), OrderOutcome::Cancelled);
    }

    #[test]
    fn classifies_filled() {
        assert_eq!(classify_order(false, "2"), OrderOutcome::Filled);
    }

    #[test]
    fn unparseable_size_retries_instead_of_cancelling() {
        assert_eq!(classify_order(false, "abc"), OrderOutcome::Working);
    }
}
