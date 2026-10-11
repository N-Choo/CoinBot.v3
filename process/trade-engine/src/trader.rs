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
    let Some((side, side_str)) = order_side(&action) else {
        warn!("[TRADE] {} ignored signal action '{}'", c.ticker, action);
        return Ok(());
    };

    let price = signal.entry_price;
    let Ok(available) = c.available_fund.parse::<f64>() else {
        warn!(
            "[TRADE] {} invalid available_fund '{}'",
            c.ticker, c.available_fund
        );
        return Ok(());
    };
    let quantity = compute_quantity(available, price);
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
        &trade_fund.to_string(),
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
        let reason = format!(
            "order rejected (code={}, msg={})",
            res.code,
            res.msg.as_deref().unwrap_or("<none>")
        );
        warn!("[TRADE] {} {}", c.ticker, reason);
        fail_order(pool, order.id, &reason).await;
        return Err(ProcessError::Internal(reason));
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

/// Map an already-uppercased signal action to a KuCoin side and its wire string.
fn order_side(action: &str) -> Option<(Side, &'static str)> {
    match action {
        "BUY" => Some((Side::Buy, "buy")),
        "SELL" => Some((Side::Sell, "sell")),
        _ => None,
    }
}

/// Order size: available funds / price, floored to 4 decimal places.
fn compute_quantity(available: f64, price: f64) -> f64 {
    ((available / price) * 10_000.0).floor() / 10_000.0
}

/// Best-effort: mark an order failed so the ledger reflects a placement error.
pub async fn fail_order(pool: &PgPool, id: sqlx::types::Uuid, reason: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_buy_and_sell() {
        assert_eq!(order_side("BUY"), Some((Side::Buy, "buy")));
        assert_eq!(order_side("SELL"), Some((Side::Sell, "sell")));
    }

    #[test]
    fn rejects_unknown_or_lowercase_actions() {
        assert_eq!(order_side("buy"), None);
        assert_eq!(order_side("HOLD"), None);
        assert_eq!(order_side(""), None);
    }

    #[test]
    fn quantity_is_floored_to_four_decimals() {
        assert_eq!(compute_quantity(1.0, 3.0), 0.3333);
        assert_eq!(compute_quantity(2.0, 3.0), 0.6666);
        assert_eq!(compute_quantity(100.0, 2.0), 50.0);
        assert_eq!(compute_quantity(0.0, 2.0), 0.0);
    }
}
