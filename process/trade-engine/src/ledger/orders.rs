//! Atomic order operations. Each function is one transaction.

use rust_decimal::Decimal;
use share::db::contracts::Contract;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Result, funds};

/// Exchange-reported fill for an order.
pub struct Fill<'a> {
    pub deal_size: Decimal,
    pub fill_price: Decimal,
    pub fee: Decimal,
    pub fee_currency: Option<&'a str>,
    pub pnl: Option<Decimal>,
}

/// Record a new `pending` open order and reserve its fund in one transaction, so a
/// committed `pending` row always has its fund reserved (the reconciler relies on
/// this to release only what was actually reserved).
#[allow(clippy::too_many_arguments)]
pub async fn create_pending_open(
    pool: &PgPool,
    c: &Contract,
    side: &str,
    comment: &str,
    quantity: Decimal,
    fund: Decimal,
    tp_price: Decimal,
    sl_price: Decimal,
) -> Result<TradeOrder> {
    let mut tx = pool.begin().await?;
    let order = TradeOrders::create(
        &mut tx, c.id, c.user_uid, &c.ticker, side, "open", comment, quantity, fund, tp_price,
        sl_price,
    )
    .await?;
    // Dropping `tx` on error rolls back both the row and the reservation.
    funds::reserve(&mut tx, c.id, fund).await?;
    tx.commit().await?;
    Ok(order)
}

/// Reserve a `close` row linked to `entry`. `Ok(None)` when a live exit already
/// exists for this position. No fund moves: a close returns cash on fill.
pub async fn reserve_exit(
    pool: &PgPool,
    entry: &TradeOrder,
    side: &str,
    comment: &str,
    quantity: Decimal,
    notional: Decimal,
) -> Result<Option<TradeOrder>> {
    TradeOrders::reserve_exit(pool, entry, side, comment, quantity, notional).await
}

/// Record the exchange id of an order once the exchange accepts it.
pub async fn record_placed(pool: &PgPool, id: Uuid, exchange_order_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_placed(&mut tx, id, exchange_order_id).await?;
    tx.commit().await
}

/// Mark an order failed. Does not touch funds; pair with [`release_reserved`]
/// when an entry that reserved a fund never reached the exchange.
pub async fn record_failed(pool: &PgPool, id: Uuid, reason: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_failed(&mut tx, id, reason).await?;
    tx.commit().await
}

/// Return a fund reserved by an order that will never be placed.
pub async fn release_reserved(pool: &PgPool, contract_id: Uuid, amount: Decimal) -> Result<()> {
    let mut tx = pool.begin().await?;
    funds::release(&mut tx, contract_id, amount).await?;
    tx.commit().await
}

/// Mark an order cancelled with no fill. An entry also releases its reserved
/// fund; a cancelled exit returns nothing.
pub async fn record_cancelled(pool: &PgPool, order: &TradeOrder) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_cancelled(&mut tx, order.id, "no fill (cancelled)").await?;
    if order.action == "open" {
        funds::release(&mut tx, order.contract_id, order.funds).await?;
    }
    tx.commit().await
}

/// Mark an order filled. `finalize` (close only) is credited to the contract in
/// the same transaction.
pub async fn record_filled(
    pool: &PgPool,
    order: &TradeOrder,
    fill: &Fill<'_>,
    finalize: Option<Decimal>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_filled(
        &mut tx,
        order.id,
        fill.deal_size,
        fill.fill_price,
        fill.fee,
        fill.fee_currency,
        fill.pnl,
    )
    .await?;
    if let Some(amount) = finalize {
        funds::finalize(&mut tx, order.contract_id, amount).await?;
    }
    tx.commit().await
}
