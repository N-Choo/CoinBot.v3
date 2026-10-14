//! Trade execution: turn a verified signal into a KuCoin margin order and record it.

use kucoin::{
    client::rest::KuCoinClient,
    types::margin::MarginOrderRequest,
    types::market::SymbolInfo,
    types::spot::{Side, TradeType},
};
use log::{info, warn};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use share::{
    ProcessError,
    db::contracts::{Contract, Contracts},
    db::trade::TradeOrders,
    models::signals::TradeSignal,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Whether a margin order opens a new position or closes an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderIntent {
    Open,
    Close,
}

/// Build a cross-margin market order.
///
/// `size` is the base-asset quantity. Only short positions borrow/repay: a
/// short entry (`sell`) borrows the base asset (and is set to repay); a short
/// close (`buy`) repays it. Long orders never borrow or repay.
pub fn margin_market_order(
    ticker: &str,
    side: Side,
    size: String,
    client_oid: &str,
    intent: OrderIntent,
) -> MarginOrderRequest {
    let short_open = intent == OrderIntent::Open && side == Side::Sell;
    let short_close = intent == OrderIntent::Close && side == Side::Buy;

    let mut request = MarginOrderRequest::new(TradeType::Market, ticker, side);
    request.size = Some(size);
    request.client_oid = client_oid.to_string();
    request.auto_borrow = Some(short_open);
    request.auto_repay = Some(short_open || short_close);
    request
}

/// Execute a verified signal for a contract in KuCoin **margin**.
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
    // Budget each order at half of the contract's initial fund; the caller
    // already verified the available fund covers it.
    let budget = order_budget(c.init_fund);
    let budget_f = budget.to_f64().unwrap_or(0.0);

    // KuCoin symbol ("BTC/USDT" -> "BTC-USDT").
    let ticker = c.ticker.replace('/', "-");
    // One unique client order id per attempt; the contract link is `contract_id`.
    let comment = Uuid::new_v4().to_string();

    // Size must respect the pair's increment/min-size rules, or KuCoin rejects it.
    let info = fetch_symbol_info(kcc, &ticker).await?;
    let Some(quantity) =
        compute_quantity(budget_f, price, &info.base_increment, &info.base_min_size)
    else {
        warn!(
            "[TRADE] {} skipped | price={price} budget={budget} inc={} min={}",
            c.ticker, info.base_increment, info.base_min_size
        );
        return Ok(());
    };
    let quantity_dec: Decimal = quantity.parse().unwrap_or_default();
    let trade_fund = Decimal::from_f64(price).unwrap_or_default() * quantity_dec;
    let tp_dec = Decimal::from_f64(signal.tp_price).unwrap_or_default();
    let sl_dec = Decimal::from_f64(signal.sl_price).unwrap_or_default();

    // Record the order and reserve its fund in ONE transaction, so a committed
    // `pending` row always has its fund reserved (the reconciler relies on this
    // to release only what was actually reserved).
    let order = {
        let mut tx = pool.begin().await?;
        let order = match TradeOrders::create(
            &mut tx,
            c.id,
            c.user_uid,
            &c.ticker,
            side_str,
            "open",
            &comment,
            quantity_dec,
            trade_fund,
            tp_dec,
            sl_dec,
        )
        .await
        {
            Ok(o) => o,
            Err(e) => return Err(e.into()),
        };
        if let Err(e) = Contracts::consume_fund(&mut tx, c.id, trade_fund).await {
            // Roll back both the row and the reservation.
            return Err(e.into());
        }
        tx.commit().await?;
        order
    };

    // Signal entries always open a position.
    let request = margin_market_order(&ticker, side, quantity.clone(), &comment, OrderIntent::Open);

    let res = match kcc.margin().place_order(request).await {
        Ok(r) => r,
        Err(e) => {
            // Ambiguous transport error: the exchange may have accepted the
            // order. Leave the row pending (funds stay reserved); the reconciler
            // resolves it by clientOid and releases the fund only if the order
            // was never placed.
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
        release_fund(pool, c.id, trade_fund).await;
        fail_order(pool, order.id, &reason).await;
        return Err(ProcessError::Internal(reason));
    };

    let mut tx = pool.begin().await?;
    TradeOrders::mark_placed(&mut tx, order.id, &data.order_id).await?;
    tx.commit().await?;

    info!(
        "[TRADE] {} {} qty={quantity} price={price} fund={trade_fund} oid={}",
        c.ticker, side_str, data.order_id
    );
    Ok(())
}

/// Best-effort: return a reserved fund to the contract after a failed placement.
async fn release_fund(pool: &PgPool, contract_id: Uuid, amount: Decimal) {
    match pool.begin().await {
        Ok(mut tx) => {
            if let Err(e) = Contracts::release_fund(&mut tx, contract_id, amount).await {
                warn!("[TRADE] failed to release fund for contract {contract_id}: {e}");
            }
            let _ = tx.commit().await;
        }
        Err(e) => warn!("[TRADE] could not open tx to release fund: {e}"),
    }
}

/// Divisor applied to a contract's initial fund to get the per-order budget.
const ORDER_FUND_DIVISOR: u32 = 2;

/// Budget for a single order: `init_fund / ORDER_FUND_DIVISOR` (half).
pub fn order_budget(init_fund: Decimal) -> Decimal {
    init_fund / Decimal::from(ORDER_FUND_DIVISOR)
}

/// Map an already-uppercased signal action to a KuCoin side and its wire string.
fn order_side(action: &str) -> Option<(Side, &'static str)> {
    match action {
        "BUY" => Some((Side::Buy, "buy")),
        "SELL" => Some((Side::Sell, "sell")),
        _ => None,
    }
}

/// Fetch `symbol`'s increment metadata from KuCoin's `/api/v1/symbols`.
async fn fetch_symbol_info(kcc: &KuCoinClient, symbol: &str) -> Result<SymbolInfo, ProcessError> {
    kcc.market()
        .get_symbol(symbol)
        .await
        .map_err(|e| ProcessError::Network(format!("symbols fetch failed: {e}")))?
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
    fn short_open_borrows_and_repays() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Sell,
            "15.9".to_string(),
            "cid-1",
            OrderIntent::Open,
        );
        assert_eq!(req.symbol, "XRP-USDT");
        assert_eq!(req.client_oid, "cid-1");
        assert_eq!(req.size.as_deref(), Some("15.9"));
        assert_eq!(req.side, Side::Sell);
        assert!(matches!(req.order_type, TradeType::Market));
        assert_eq!(req.auto_borrow, Some(true));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn long_open_does_not_borrow_or_repay() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            "10".to_string(),
            "cid-2",
            OrderIntent::Open,
        );
        assert_eq!(req.side, Side::Buy);
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn short_close_repays_but_does_not_borrow() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            "10".to_string(),
            "cid-3",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn long_close_does_not_borrow_or_repay() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Sell,
            "10".to_string(),
            "cid-4",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn counts_decimal_scale() {
        assert_eq!(decimal_scale("0.00000001"), 8);
        assert_eq!(decimal_scale("0.001"), 3);
        assert_eq!(decimal_scale("0.1"), 1);
        assert_eq!(decimal_scale("1"), 0);
    }
}
