//! Exit an open position when its price reaches its take-profit or stop-loss.
//!
//! Polls the exchange for last prices, compares them against each held
//! position's stored TP/SL, and places a market exit order on the opposite side
//! when one is reached. It also closes a position at market when a target-side
//! signal reverses direction (see [`close_at_market`]). Every exit is recorded
//! as a `close` row linked to its entry via `parent_order_id`, which the
//! reconciler then settles (mark filled + credit realized PnL).

mod decision;
mod exit;
mod price;

use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use log::{error, warn};
use rust_decimal::Decimal;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};

use crate::symbol::margin_symbol;
use decision::{ExitReason, evaluate_exit};
use exit::exit_position;
use price::fetch_last_prices;

/// Poll interval, in seconds.
const POLL_SECS: u64 = 12;

/// Backoff applied after KuCoin answers `429 Too Many Requests`, in seconds.
const BASE_BACKOFF_SECS: u64 = 30;

/// Upper bound for the 429 backoff, in seconds.
const MAX_BACKOFF_SECS: u64 = 300;

/// Error from a sweep pass; distinguishes throttling so the loop can back off.
enum SweepError {
    /// KuCoin answered 429: stop sweeping and wait before retrying so we don't
    /// keep hammering a limit we're already exceeding.
    RateLimited,
    /// Any other failure.
    Other(String),
}

impl From<String> for SweepError {
    fn from(e: String) -> Self {
        SweepError::Other(e)
    }
}

/// Poll open positions forever and exit any whose TP or SL has been reached.
///
/// On a 429 the interval is stretched with exponential backoff (reset as soon as
/// a sweep succeeds) instead of retrying on the normal cadence.
pub async fn run(pool: Pool<Postgres>, kcc: KuCoinClient) {
    let mut backoff = 0u64;
    loop {
        match sweep_once(&pool, &kcc).await {
            Ok(()) => backoff = 0,
            Err(SweepError::RateLimited) => {
                backoff = if backoff == 0 {
                    BASE_BACKOFF_SECS
                } else {
                    (backoff * 2).min(MAX_BACKOFF_SECS)
                };
                warn!("[EXIT] rate limited by KuCoin; backing off {backoff}s");
            }
            Err(SweepError::Other(e)) => error!("[EXIT] {e}"),
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS.max(backoff))).await;
    }
}

/// Run a single sweep pass.
async fn sweep_once(pool: &Pool<Postgres>, kcc: &KuCoinClient) -> Result<(), SweepError> {
    let positions = TradeOrders::held_entries(pool)
        .await
        .map_err(|e| format!("held-entries query failed: {e}"))?;
    if positions.is_empty() {
        return Ok(());
    }

    let mut symbols: Vec<String> = positions.iter().map(|e| margin_symbol(&e.ticker)).collect();
    symbols.sort();
    symbols.dedup();

    let prices = fetch_last_prices(kcc, &symbols).await?;

    for entry in positions {
        let (Some(tp), Some(sl)) = (entry.tp_price, entry.sl_price) else {
            continue;
        };
        let symbol = margin_symbol(&entry.ticker);
        let Some(&last) = prices.get(&symbol) else {
            warn!("[EXIT] no price for {}", entry.ticker);
            continue;
        };
        let Some(reason) = evaluate_exit(&entry.side, last, tp, sl) else {
            continue;
        };

        if let Err(e) = exit_position(pool, kcc, &entry, reason, last).await {
            error!("[EXIT] exit failed for order {}: {e}", entry.id);
        }
    }

    Ok(())
}

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
    exit::exit_position(pool, kcc, entry, ExitReason::Reversal, reference_price).await
}
