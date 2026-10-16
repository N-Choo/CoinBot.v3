//! Trade execution: turn a verified signal into a KuCoin margin order and record it.

mod order;
mod sizing;

pub use order::{OrderIntent, margin_market_order};
pub use sizing::order_budget;

use crate::ledger;
use kucoin::{client::rest::KuCoinClient, types::market::SymbolInfo};
use log::{info, warn};
use order::order_side;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use share::{ProcessError, db::contracts::Contract, models::signals::TradeSignal};
use sizing::compute_quantity;
use sqlx::PgPool;
use uuid::Uuid;

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

    let order = ledger::create_pending_open(
        pool,
        c,
        side_str,
        &comment,
        quantity_dec,
        trade_fund,
        tp_dec,
        sl_dec,
    )
    .await?;

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
        if let Err(e) = ledger::release_reserved(pool, c.id, trade_fund).await {
            warn!("[TRADE] failed to release fund for contract {}: {e}", c.id);
        }
        if let Err(e) = ledger::record_failed(pool, order.id, &reason).await {
            warn!("[TRADE] failed to mark order {} failed: {e}", order.id);
        }
        return Err(ProcessError::Internal(reason));
    };

    ledger::record_placed(pool, order.id, &data.order_id).await?;

    info!(
        "[TRADE] {} {} qty={quantity} price={price} fund={trade_fund} oid={}",
        c.ticker, side_str, data.order_id
    );
    Ok(())
}

/// Fetch `symbol`'s increment metadata from KuCoin's `/api/v1/symbols`.
async fn fetch_symbol_info(kcc: &KuCoinClient, symbol: &str) -> Result<SymbolInfo, ProcessError> {
    kcc.market()
        .get_symbol(symbol)
        .await
        .map_err(|e| ProcessError::Network(format!("symbols fetch failed: {e}")))?
        .ok_or_else(|| ProcessError::Internal(format!("symbol {symbol} not found")))
}
