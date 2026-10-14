//! Place the market exit for a triggered position and record it.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::spot::Side;
use log::info;
use rust_decimal::Decimal;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use super::decision::ExitReason;
use crate::symbol::margin_symbol;

/// Place a market exit for `entry` and record it as a linked `close` order.
pub(super) async fn exit_position(
    pool: &Pool<Postgres>,
    kcc: &KuCoinClient,
    entry: &TradeOrder,
    reason: ExitReason,
    last: Decimal,
) -> Result<(), String> {
    let (exit_side, side_str) = match entry.side.as_str() {
        "buy" => (Side::Sell, "sell"),
        "sell" => (Side::Buy, "buy"),
        other => return Err(format!("unknown entry side '{other}'")),
    };

    let symbol = margin_symbol(&entry.ticker);
    let comment = Uuid::new_v4().to_string();

    // Reserve the exit first so a crash or retry cannot place a second one.
    let Some(exit) = TradeOrders::reserve_exit(
        pool,
        entry,
        side_str,
        &comment,
        entry.filled_qty,
        entry.filled_qty * last,
    )
    .await
    .map_err(|e| format!("reserve_exit failed: {e}"))?
    else {
        // A live exit already exists for this position; nothing to do.
        return Ok(());
    };

    let request = crate::trader::margin_market_order(
        &symbol,
        exit_side,
        entry.filled_qty.to_string(),
        &comment,
        crate::trader::OrderIntent::Close,
    );

    let res = match kcc.margin().place_order(request).await {
        Ok(r) => r,
        Err(e) => {
            // Ambiguous: the exchange may have accepted the order. Leave the
            // exit reserved (pending) so the reconciler resolves it by clientOid;
            // marking it failed would let a later sweep place a duplicate exit.
            return Err(format!("place_order failed: {e}"));
        }
    };

    let Some(data) = res.data else {
        let reason = format!(
            "exit rejected (code={}, msg={})",
            res.code,
            res.msg.as_deref().unwrap_or("<none>")
        );
        // Definite rejection: no order exists, so release the reservation.
        crate::trader::fail_order(pool, exit.id, &reason).await;
        return Err(reason);
    };

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    TradeOrders::mark_placed(&mut tx, exit.id, &data.order_id)
        .await
        .map_err(|e| format!("mark_placed failed: {e}"))?;
    tx.commit().await.map_err(|e| e.to_string())?;

    info!(
        "[EXIT] {} {} exit for order {} -> {}",
        entry.ticker,
        reason.label(),
        entry.id,
        data.order_id
    );
    Ok(())
}
