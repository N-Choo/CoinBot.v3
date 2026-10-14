//! Background sweeper: exit an open position when its price reaches TP or SL.
//!
//! Polls the exchange for last prices, compares them against each held
//! position's stored take-profit / stop-loss, and places a market exit order on
//! the opposite side when one is reached. The exit is recorded as a `close`
//! row linked to its entry via `parent_order_id`, which the contract sweeper
//! then reconciles (mark filled + credit proceeds).

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use kucoin::types::spot::Side;
use log::{error, info, warn};
use rust_decimal::Decimal;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

///  Sweep the orders pause time, in seconds.
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

/// Which protective level a position hit.
#[derive(Debug, PartialEq, Eq)]
enum ExitReason {
    TakeProfit,
    StopLoss,
}

impl ExitReason {
    /// Short label used in logs.
    fn label(&self) -> &'static str {
        match self {
            ExitReason::TakeProfit => "TP",
            ExitReason::StopLoss => "SL",
        }
    }
}

/// Poll open positions forever and exit any whose TP or SL has been reached.
///
/// On a 429 the interval is stretched with exponential backoff (reset as soon as
/// a sweep succeeds) instead of retrying on the normal cadence.
pub async fn run_order_sweeper(pool: Pool<Postgres>, kcc: KuCoinClient) {
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
                warn!("[ORDER-SWEEP] rate limited by KuCoin; backing off {backoff}s");
            }
            Err(SweepError::Other(e)) => error!("[ORDER-SWEEP] {e}"),
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
            warn!("[ORDER-SWEEP] no price for {}", entry.ticker);
            continue;
        };
        let Some(reason) = evaluate_exit(&entry.side, last, tp, sl) else {
            continue;
        };

        if let Err(e) = exit_position(pool, kcc, &entry, reason, last).await {
            error!("[ORDER-SWEEP] exit failed for order {}: {e}", entry.id);
        }
    }

    Ok(())
}

/// Decide whether `side`'s position has reached `tp` or `sl` at price `last`.
///
/// Long (`buy`) exits high on TP / low on SL; short (`sell`) is inverted.
fn evaluate_exit(side: &str, last: Decimal, tp: Decimal, sl: Decimal) -> Option<ExitReason> {
    match side {
        "buy" => {
            if last >= tp {
                Some(ExitReason::TakeProfit)
            } else if last <= sl {
                Some(ExitReason::StopLoss)
            } else {
                None
            }
        }
        "sell" => {
            if last <= tp {
                Some(ExitReason::TakeProfit)
            } else if last >= sl {
                Some(ExitReason::StopLoss)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// KuCoin margin symbol: `"BTC/USDT"` -> `"BTC-USDT"`.
fn margin_symbol(ticker: &str) -> String {
    ticker.replace('/', "-")
}

/// Fetch `symbol -> last price` for the given margin symbols.
///
/// KuCoin margin has no last-price endpoint of its own (its mark-price feed is
/// BTC-denominated and BTC-pairs only) — margin trades on the same order book as
/// spot, so this uses the shared market-data endpoint. One light
/// `GET /api/v1/market/orderbook/level1` request per held symbol (weight 1)
/// instead of the heavy `allTickers` endpoint, so we only pay for the symbols
/// actually held and avoid tripping KuCoin's per-IP rate limit.
async fn fetch_last_prices(
    kcc: &KuCoinClient,
    symbols: &[String],
) -> Result<HashMap<String, Decimal>, SweepError> {
    let mut prices = HashMap::with_capacity(symbols.len());
    for symbol in symbols {
        let res = kcc.market().get_last_price(symbol).await.map_err(|e| {
            if e.is_rate_limited() {
                SweepError::RateLimited
            } else {
                SweepError::Other(format!("level1 request failed for {symbol}: {e}"))
            }
        })?;

        let Some(data) = res.data else {
            continue;
        };
        if let Ok(last) = Decimal::from_str(&data.price) {
            prices.insert(symbol.clone(), last);
        }
    }
    Ok(prices)
}

/// Place a market exit for `entry` and record it as a linked `close` order.
async fn exit_position(
    pool: &Pool<Postgres>,
    kcc: &KuCoinClient,
    entry: &TradeOrder,
    reason: ExitReason,
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
    let Some(exit) = TradeOrders::reserve_exit(
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
        crate::trader::fail_order(pool, exit.id, &reason).await;
        return Err(reason);
    };

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    TradeOrders::mark_placed(&mut tx, exit.id, &data.order_id)
        .await
        .map_err(|e| format!("mark_placed failed: {e}"))?;
    tx.commit().await.map_err(|e| e.to_string())?;

    info!(
        "[ORDER-SWEEP] {} {} exit for order {} -> {}",
        entry.ticker,
        reason.label(),
        entry.id,
        data.order_id
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn builds_margin_symbol() {
        assert_eq!(margin_symbol("BTC/USDT"), "BTC-USDT");
        assert_eq!(margin_symbol("BTC-USDT"), "BTC-USDT");
    }

    #[test]
    fn long_exits_on_tp_and_sl() {
        let (tp, sl) = (dec("110"), dec("90"));
        assert_eq!(
            evaluate_exit("buy", dec("110"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("buy", dec("120"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("buy", dec("90"), tp, sl),
            Some(ExitReason::StopLoss)
        );
        assert_eq!(evaluate_exit("buy", dec("100"), tp, sl), None);
    }

    #[test]
    fn short_exits_on_tp_and_sl() {
        let (tp, sl) = (dec("90"), dec("110"));
        assert_eq!(
            evaluate_exit("sell", dec("90"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("sell", dec("110"), tp, sl),
            Some(ExitReason::StopLoss)
        );
        assert_eq!(evaluate_exit("sell", dec("100"), tp, sl), None);
    }

    #[test]
    fn unknown_side_never_exits() {
        assert_eq!(evaluate_exit("hold", dec("999"), dec("1"), dec("2")), None);
    }
}
