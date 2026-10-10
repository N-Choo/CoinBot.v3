//! Trade execution: turn a verified signal into a KuCoin spot order and record it.

use kucoin::{
    client::rest::KuCoinClient,
    types::spot::{Side, SpotOrderRequest, TradeType},
};
use log::{info, warn};
use share::{
    ProcessError,
    db::contracts::{Contract, Contracts},
    db::trade::TradeOrders,
    models::signals::TradeSignal,
};
use sqlx::PgPool;

/// Execute a verified signal for a contract in KuCoin **spot**.
pub async fn execute(
    kcc: &KuCoinClient,
    pool: &PgPool,
    signal: &TradeSignal,
    c: &Contract,
) -> Result<(), ProcessError> {
    let action = signal.action.to_uppercase();
    let (side, side_str) = match action.as_str() {
        "BUY" => (Side::Buy, "buy"),
        "SELL" => (Side::Sell, "sell"),
        other => {
            warn!("[TRADE] {} ignored signal action '{}'", c.ticker, other);
            return Ok(());
        }
    };

    let price = signal.entry_price;
    let Ok(available) = c.available_fund.parse::<f64>() else {
        warn!(
            "[TRADE] {} invalid available_fund '{}'",
            c.ticker, c.available_fund
        );
        return Ok(());
    };
    let quantity = ((available / price) * 10_000.0).floor() / 10_000.0;
    if price <= 0.0 || quantity <= 0.0 {
        warn!(
            "[TRADE] {} skipped | price={price} available={available} qty={quantity}",
            c.ticker
        );
        return Ok(());
    }
    let trade_fund = price * quantity;

    // KuCoin spot symbol ("BTC/USDT" -> "BTC-USDT"); the remark is capped at 20 chars.
    let ticker = c.ticker.replace('/', "-");
    let comment = c.id.to_string();
    let remark: String = comment.chars().take(20).collect();

    // Claim the order slot first: the unique `comment` index rejects a second order.
    let order = match TradeOrders::create(
        pool,
        c.id,
        c.user_uid,
        &c.ticker,
        side_str,
        "open",
        &comment,
        &quantity.to_string(),
    )
    .await
    {
        Ok(o) => o,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            warn!("[TRADE] {} already has an order; skipping", c.ticker);
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    // Deterministic client id so the exchange rejects a duplicate on retry.
    let mut request = SpotOrderRequest::new(TradeType::Market, &ticker, side)
        .set_size(quantity)
        .set_remark(&remark);
    request.client_oid = Some(comment.clone());

    let res = match kcc.spot().place_order(request).await {
        Ok(r) => r,
        Err(e) => {
            fail_order(pool, order.id, &format!("place_order failed: {e}")).await;
            return Err(ProcessError::Network(format!("place_order failed: {e}")));
        }
    };
    let Some(data) = res.data else {
        fail_order(pool, order.id, "empty order response").await;
        return Err(ProcessError::Internal("empty order response".into()));
    };

    // Move the traded funds (available -> total_traded).
    let mut tx = pool.begin().await?;
    Contracts::consume_fund(&mut tx, c.id, &trade_fund.to_string()).await?;
    tx.commit().await?;

    let mut tx = pool.begin().await?;
    TradeOrders::mark_placed(&mut tx, order.id, &data.order_id).await?;
    tx.commit().await?;

    info!(
        "[TRADE] {} {} qty={quantity} price={price} fund={trade_fund} oid={}",
        c.ticker, side_str, data.order_id
    );
    Ok(())
}

/// Best-effort: mark an order failed so the ledger reflects a placement error.
async fn fail_order(pool: &PgPool, id: sqlx::types::Uuid, reason: &str) {
    match pool.begin().await {
        Ok(mut tx) => {
            if let Err(e) = TradeOrders::mark_failed(&mut tx, id, reason).await {
                warn!("[TRADE] failed to mark order {id} failed: {e}");
            }
            let _ = tx.commit().await;
        }
        Err(e) => warn!("[TRADE] could not open tx to mark order {id} failed: {e}"),
    }
}
