//! Place the market exit for a triggered position and record it.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::spot::Side;
use log::{info, warn};
use rust_decimal::Decimal;
use share::db::trade::TradeOrder;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::ledger;
use crate::symbol::margin_symbol;

/// Place a market exit for `entry` and record it as a linked `close` order.
pub(crate) async fn exit_position(
    pool: &Pool<Postgres>,
    kcc: &KuCoinClient,
    entry: &TradeOrder,
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
    let Some(exit) = ledger::reserve_exit(
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
        if let Err(e) = ledger::record_failed(pool, exit.id, &reason).await {
            warn!("[EXIT] failed to mark exit {} failed: {e}", exit.id);
        }
        return Err(reason);
    };

    ledger::record_placed(pool, exit.id, &data.order_id)
        .await
        .map_err(|e| format!("mark_placed failed: {e}"))?;

    info!(
        "[EXIT] {} REVERSAL exit for order {} -> {}",
        entry.ticker, entry.id, data.order_id
    );
    Ok(())
}
