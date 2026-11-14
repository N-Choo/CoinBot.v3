//! Trade execution: turn a verified signal into a KuCoin margin order and record it.

mod order;
mod sizing;

pub use order::{OrderIntent, margin_market_order};
pub use sizing::{order_margin, order_notional};

use crate::kucoin_margin;
use crate::ledger;
use kucoin::client::rest::KuCoinClient;
use log::{info, warn};
use order::order_side;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use share::db::trade::{TradeOrder, TradeOrders};
use share::{ProcessError, db::contracts::Contract, models::signals::TradeSignal};
use sizing::{compute_funds, compute_quantity, parse_quantity, to_decimal};
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

/// Map a wire level (`0.0` means "no level") to an optional price.
///
/// Returns `None` for values that are not a usable price (negative, NaN, or
/// infinite); callers reject the signal in that case.
fn optional_level(value: f64) -> Option<Option<Decimal>> {
    if value == 0.0 {
        Some(None)
    } else if !value.is_finite() || value < 0.0 {
        None
    } else {
        to_decimal(value).map(Some)
    }
}

/// Execute a verified signal for a contract in KuCoin **margin**.
pub async fn execute(
    kcc: &KuCoinClient,
    pool: &PgPool,
    signal: &TradeSignal,
    c: &Contract,
    leverage: u32,
) -> Result<(), ProcessError> {
    let action = signal.action.to_uppercase();
    let Some((side, side_str)) = order_side(&action) else {
        warn!("[TRADE] {} ignored signal action '{}'", c.ticker, action);
        return Ok(());
    };

    let price = signal.entry_price;
    // Reserve half of the contract's fund as margin and borrow up to `leverage`.
    let margin = order_margin(c.init_fund);
    let notional = order_notional(margin, leverage);

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
    let rules = kucoin_margin::symbol_rules(kcc, &ticker)
        .await
        .map_err(|e| ProcessError::Network(format!("symbol rules fetch failed: {e}")))?;
    let Some(quantity) = compute_quantity(
        notional.to_f64().unwrap_or(0.0),
        price,
        &rules.base_increment,
        &rules.base_min_size,
    ) else {
        warn!(
            "[TRADE] {} skipped | price={price} notional={notional} inc={} min={}",
            c.ticker, rules.base_increment, rules.base_min_size
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

    // A leveraged long must send `funds` (quote) so KuCoin can auto-borrow; all
    // other entries send `size` (base), including leveraged shorts.
    let (order_size, order_funds) = if side_str == "buy" && leverage > 1 {
        let min_funds = [
            rules.quote_min_size.as_str(),
            rules.min_funds.as_deref().unwrap_or(""),
        ]
        .into_iter()
        .filter_map(|s| s.parse::<Decimal>().ok())
        .max()
        .unwrap_or(Decimal::ZERO);
        let Some(funds) = compute_funds(notional, &rules.quote_increment, min_funds) else {
            warn!(
                "[TRADE] {} skipped | price={price} notional={notional} quote inc={} min={min_funds}",
                c.ticker, rules.quote_increment
            );
            return Ok(());
        };
        (None, Some(funds))
    } else {
        (Some(quantity.clone()), None)
    };
    let Some(tp_dec) = optional_level(signal.tp_price) else {
        warn!(
            "[TRADE] {} skipped | invalid tp_price {}",
            c.ticker, signal.tp_price
        );
        return Ok(());
    };
    let Some(sl_dec) = optional_level(signal.sl_price) else {
        warn!(
            "[TRADE] {} skipped | invalid sl_price {}",
            c.ticker, signal.sl_price
        );
        return Ok(());
    };
    // Reserve only the margin; the exchange borrows the rest of the notional.
    let trade_fund = margin;

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

    // A cancel may have landed after this signal was pulled; never open a new
    // position for a contract that is no longer active.
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM contracts WHERE id = $1")
        .bind(c.id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ProcessError::Internal(format!("contract re-read failed: {e}")))?;
    if status.as_deref() != Some("active") {
        info!("[TRADE] contract {} is no longer active; skipping", c.id);
        return Ok(());
    }

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
    let request = margin_market_order(
        &ticker,
        side,
        order_size,
        order_funds,
        &comment,
        OrderIntent::Open,
    );

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

    let borrow_size = data
        .borrow_size
        .as_deref()
        .and_then(|v| v.parse::<Decimal>().ok());
    ledger::record_placed(
        pool,
        order.id,
        &data.order_id,
        data.loan_apply_id.as_deref(),
        borrow_size,
    )
    .await?;

    info!(
        "[TRADE] {} {} qty={quantity} price={price} margin={trade_fund} notional={notional} leverage={leverage} oid={}",
        c.ticker, side_str, data.order_id
    );
    Ok(())
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

    #[test]
    fn wire_zero_means_no_level() {
        assert_eq!(optional_level(0.0), Some(None));
    }

    #[test]
    fn valid_level_is_converted() {
        assert_eq!(optional_level(110.0), Some(Some(Decimal::from(110))));
    }

    #[test]
    fn invalid_levels_are_rejected() {
        assert_eq!(optional_level(-1.0), None);
        assert_eq!(optional_level(f64::NAN), None);
        assert_eq!(optional_level(f64::INFINITY), None);
    }
}
