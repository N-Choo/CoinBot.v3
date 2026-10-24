//! Close an open position at market when a target-side signal reverses direction.
//!
//! Positions close only on an opposite target-side signal. Every exit is
//! recorded as a `close` row linked to its entry via `parent_order_id`, which
//! the reconciler then settles (mark filled + credit realized PnL).

mod exit;

use kucoin::client::rest::KuCoinClient;
use rust_decimal::Decimal;
use share::db::trade::TradeOrder;
use sqlx::{Pool, Postgres};

use exit::exit_position;

/// Close `entry` at market when a signal targets the opposite side.
///
/// `reference_price` only estimates the close row's notional; the order sent to
/// the exchange is a market order.
pub(crate) async fn close_at_market(
    pool: &Pool<Postgres>,
    kcc: &KuCoinClient,
    entry: &TradeOrder,
    reference_price: Decimal,
) -> Result<(), String> {
    exit_position(pool, kcc, entry, reference_price).await
}
