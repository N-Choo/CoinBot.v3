//! Trade execution: turn a verified signal into a KuCoin margin order and record it.

mod order;
mod sizing;

pub use order::{OrderIntent, margin_market_order};
pub use sizing::order_budget;

use crate::ledger;
use kucoin::{client::rest::KuCoinClient, types::market::SymbolInfo};
use log::{info, warn};
use order::order_side;
use rust_decimal::prelude::ToPrimitive;
use share::db::trade::{TradeOrder, TradeOrders};
use share::{ProcessError, db::contracts::Contract, models::signals::TradeSignal};
use sizing::{compute_quantity, parse_quantity, to_decimal};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionAction {
    Open,
    AlreadyTarget,
    Reverse,
    CloseOpposite,
}

/// Classify what a target-side signal should do against the sides currently held.
///
/// A pre-existing two-sided (hedged) state is recovered by closing the opposite
/// leg without opening another target position, which keeps the first signal
/// from adding to the hedge. The in-flight guard should prevent new hedges.
fn position_action(held_sides: &[&str], target_side: &str) -> PositionAction {
    if held_sides.is_empty() {
        PositionAction::Open
    } else if !held_sides.contains(&target_side) {
        PositionAction::Reverse
    } else if held_sides.iter().all(|side| *side == target_side) {
        PositionAction::AlreadyTarget
    } else {
        PositionAction::CloseOpposite
    }
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

    // Validate the price before it reaches `compute_quantity`: a non-finite
    // price (NaN/±inf) slips through its `<= 0.0` checks (NaN comparisons are
    // false) and would yield an unusable size.
    let Some(price_dec) = to_decimal(price) else {
        warn!("[TRADE] {} skipped | invalid entry price {price}", c.ticker);
        return Ok(());
    };

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
    let Some(quantity_dec) = parse_quantity(&quantity) else {
        warn!(
            "[TRADE] {} skipped | unparseable quantity '{quantity}'",
            c.ticker
        );
        return Ok(());
    };
    let trade_fund = price_dec * quantity_dec;

    if TradeOrders::in_flight_for_contract(pool, c.id)
        .await
        .map_err(|e| ProcessError::Internal(format!("in-flight query failed: {e}")))?
    {
        info!(
            "[TRADE] {} has an entry or exit in flight; waiting",
            c.ticker
        );
        return Ok(());
    }

    let held = TradeOrders::held_entries(pool)
        .await
        .map_err(|e| ProcessError::Internal(format!("held-entries query failed: {e}")))?;
    let contract_positions: Vec<&TradeOrder> = held
        .iter()
        .filter(|entry| entry.contract_id == c.id)
        .collect();
    let held_sides: Vec<&str> = contract_positions
        .iter()
        .map(|entry| entry.side.as_str())
        .collect();

    let reconciliation = position_action(&held_sides, side_str);
    match reconciliation {
        PositionAction::AlreadyTarget => {
            info!(
                "[TRADE] {} already positioned {}; ignoring target-side signal",
                c.ticker, side_str
            );
            return Ok(());
        }
        PositionAction::Open => {}
        PositionAction::Reverse | PositionAction::CloseOpposite => {
            for entry in contract_positions
                .iter()
                .copied()
                .filter(|entry| entry.side != side_str)
            {
                crate::position_exit::close_at_market(pool, kcc, entry, price_dec)
                    .await
                    .map_err(|e| ProcessError::Internal(format!("reversal close failed: {e}")))?;
            }
            if reconciliation == PositionAction::CloseOpposite {
                info!(
                    "[TRADE] {} closed opposite hedge leg(s); not opening another {}",
                    c.ticker, side_str
                );
                return Ok(());
            }
        }
    }

    let order =
        ledger::create_pending_open(pool, c, side_str, &comment, quantity_dec, trade_fund).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_account_opens_the_target_side() {
        assert_eq!(position_action(&[], "buy"), PositionAction::Open);
        assert_eq!(position_action(&[], "sell"), PositionAction::Open);
    }

    #[test]
    fn already_target_side_is_a_no_op() {
        assert_eq!(
            position_action(&["buy"], "buy"),
            PositionAction::AlreadyTarget
        );
        assert_eq!(
            position_action(&["sell"], "sell"),
            PositionAction::AlreadyTarget
        );
    }

    #[test]
    fn opposite_side_reverses() {
        assert_eq!(position_action(&["sell"], "buy"), PositionAction::Reverse);
        assert_eq!(position_action(&["buy"], "sell"), PositionAction::Reverse);
    }

    #[test]
    fn mixed_state_closes_the_opposite_leg_without_reopening() {
        assert_eq!(
            position_action(&["sell", "buy"], "buy"),
            PositionAction::CloseOpposite
        );
        assert_eq!(
            position_action(&["buy", "sell"], "sell"),
            PositionAction::CloseOpposite
        );
    }
}
