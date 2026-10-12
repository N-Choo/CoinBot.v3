//! Trade execution: turn a verified signal into a KuCoin spot order and record it.

use kucoin::{
    client::rest::KuCoinClient,
    types::KuCoinResponse,
    types::spot::{Side, SpotOrderRequest, TradeType},
};
use log::{info, warn};
use serde::Deserialize;
use share::{
    ProcessError,
    db::contracts::{Contract, Contracts},
    db::trade::TradeOrders,
    models::signals::TradeSignal,
};
use sqlx::PgPool;
use uuid::Uuid;

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

    // KuCoin spot symbol ("BTC/USDT" -> "BTC-USDT"); the remark is capped at 20 chars.
    let ticker = c.ticker.replace('/', "-");
    // One unique client order id per attempt; the contract link is `contract_id`.
    let comment = Uuid::new_v4().to_string();
    let remark: String = c.id.to_string().chars().take(20).collect();

    // Size must respect the pair's increment/min-size rules, or KuCoin rejects it.
    let info = fetch_symbol_info(kcc, &ticker).await?;
    let Some(quantity) =
        compute_quantity(available, price, &info.base_increment, &info.base_min_size)
    else {
        warn!(
            "[TRADE] {} skipped | price={price} available={available} inc={} min={}",
            c.ticker, info.base_increment, info.base_min_size
        );
        return Ok(());
    };
    let Ok(quantity_value) = quantity.parse::<f64>() else {
        warn!("[TRADE] {} invalid computed qty '{}'", c.ticker, quantity);
        return Ok(());
    };
    let trade_fund = price * quantity_value;

    // Record the order (pending) before touching the exchange.
    let order = match TradeOrders::create(
        pool,
        c.id,
        c.user_uid,
        &c.ticker,
        side_str,
        "open",
        &comment,
        &quantity,
        &trade_fund.to_string(),
        &signal.tp_price.to_string(),
        &signal.sl_price.to_string(),
    )
    .await
    {
        Ok(o) => o,
        Err(e) => return Err(e.into()),
    };

    let mut request = SpotOrderRequest::new(TradeType::Market, &ticker, side).set_remark(&remark);
    request.size = Some(quantity.clone());
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

/// Minimal KuCoin symbol metadata needed to size a market order.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolInfo {
    /// Trading pair, e.g. `BTC-USDT`.
    symbol: String,
    /// Minimum size step for the base asset, as a decimal string.
    base_increment: String,
    /// Minimum order size for the base asset, as a decimal string.
    base_min_size: String,
}

/// Fetch `symbol`'s increment metadata from KuCoin's `/api/v1/symbols`.
async fn fetch_symbol_info(kcc: &KuCoinClient, symbol: &str) -> Result<SymbolInfo, ProcessError> {
    let res = kcc
        .send::<KuCoinResponse<Vec<SymbolInfo>>>("GET", "", "/api/v1/symbols")
        .await
        .map_err(|e| ProcessError::Network(format!("symbols fetch failed: {e}")))?;

    res.data
        .and_then(|symbols| symbols.into_iter().find(|s| s.symbol == symbol))
        .ok_or_else(|| ProcessError::Internal(format!("symbol {symbol} not found")))
}

/// Order size: `available / price`, floored to a multiple of `base_increment`.
///
/// Returns `None` when the inputs are non-positive, the pair has no valid
/// increment, the computed step count is zero, or the size is below
/// `base_min_size`.
fn compute_quantity(
    available: f64,
    price: f64,
    base_increment: &str,
    base_min_size: &str,
) -> Option<String> {
    if price <= 0.0 || available <= 0.0 {
        return None;
    }
    let increment: f64 = base_increment.parse().ok()?;
    if !increment.is_finite() || increment <= 0.0 {
        return None;
    }

    let steps = ((available / price) / increment).floor();
    if steps < 1.0 {
        return None;
    }

    let size = steps * increment;
    let min_size: f64 = base_min_size.parse().unwrap_or(0.0);
    if size < min_size {
        return None;
    }

    Some(format!("{:.*}", decimal_scale(base_increment), size))
}

/// Number of fractional digits in a decimal string, e.g. `"0.001"` -> `3`.
fn decimal_scale(decimal: &str) -> usize {
    decimal.split_once('.').map_or(0, |(_, frac)| frac.len())
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
    fn floors_size_to_the_pair_increment() {
        // SOL-style: 0.001 increment.
        assert_eq!(
            compute_quantity(7.993424, 80.0, "0.001", "0.001").as_deref(),
            Some("0.099")
        );
        // BTC-style: 1e-8 increment (float division rounds down one step, still valid).
        assert_eq!(
            compute_quantity(8.36, 83_600.0, "0.00000001", "0.000001").as_deref(),
            Some("0.00009999")
        );
    }

    #[test]
    fn rejects_sizes_below_one_increment() {
        // 7.993424 / 111.6 = 0.0716 -> 0 whole 0.1 steps.
        assert_eq!(compute_quantity(7.993424, 111.6, "0.1", "0.1"), None);
    }

    #[test]
    fn rejects_sizes_below_min_size() {
        assert_eq!(compute_quantity(15.0, 100.0, "0.1", "1.0"), None);
    }

    #[test]
    fn rejects_invalid_or_non_positive_inputs() {
        assert_eq!(compute_quantity(0.0, 100.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 0.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, -1.0, "0.1", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 100.0, "not-a-number", "0.1"), None);
        assert_eq!(compute_quantity(10.0, 100.0, "0", "0.1"), None);
    }

    #[test]
    fn counts_decimal_scale() {
        assert_eq!(decimal_scale("0.00000001"), 8);
        assert_eq!(decimal_scale("0.001"), 3);
        assert_eq!(decimal_scale("0.1"), 1);
        assert_eq!(decimal_scale("1"), 0);
    }
}
