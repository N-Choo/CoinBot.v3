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
use kucoin::types::KuCoinResponse;
use kucoin::types::spot::{Side, SpotOrderRequest, TradeType};
use log::{error, info, warn};
use rust_decimal::Decimal;
use serde::Deserialize;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

/// How often the sweeper polls, in seconds.
const POLL_SECS: u64 = 8;

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
pub async fn run_order_sweeper(pool: Pool<Postgres>, kcc: KuCoinClient) {
    loop {
        if let Err(e) = sweep_once(&pool, &kcc).await {
            error!("[ORDER-SWEEP] {e}");
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS)).await;
    }
}

/// Run a single sweep pass.
async fn sweep_once(pool: &Pool<Postgres>, kcc: &KuCoinClient) -> Result<(), String> {
    let positions = TradeOrders::held_entries(pool)
        .await
        .map_err(|e| format!("held-entries query failed: {e}"))?;
    if positions.is_empty() {
        return Ok(());
    }

    let prices = fetch_last_prices(kcc).await?;

    for entry in positions {
        let (Some(tp), Some(sl)) = (entry.tp_price, entry.sl_price) else {
            continue;
        };
        let symbol = spot_symbol(&entry.ticker);
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

/// KuCoin spot symbol: `"BTC/USDT"` -> `"BTC-USDT"`.
fn spot_symbol(ticker: &str) -> String {
    ticker.replace('/', "-")
}

/// Fetch `symbol -> last price` for every KuCoin market in one request.
async fn fetch_last_prices(kcc: &KuCoinClient) -> Result<HashMap<String, Decimal>, String> {
    let res = kcc
        .send::<KuCoinResponse<AllTickers>>("GET", "", "/api/v1/market/allTickers")
        .await
        .map_err(|e| format!("allTickers request failed: {e}"))?;

    let data = res
        .data
        .ok_or_else(|| "allTickers returned no data".to_string())?;

    Ok(data
        .ticker
        .into_iter()
        .filter_map(|t| {
            let last = t.last.and_then(|l| Decimal::from_str(&l).ok())?;
            Some((t.symbol, last))
        })
        .collect())
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

    let symbol = spot_symbol(&entry.ticker);
    let comment = Uuid::new_v4().to_string();

    let mut request = SpotOrderRequest::new(TradeType::Market, &symbol, exit_side);
    request.size = Some(entry.filled_qty.to_string());
    request.client_oid = Some(comment.clone());

    let res = kcc
        .spot()
        .place_order(request)
        .await
        .map_err(|e| format!("place_order failed: {e}"))?;

    let Some(data) = res.data else {
        return Err(format!(
            "exit rejected (code={}, msg={})",
            res.code,
            res.msg.as_deref().unwrap_or("<none>")
        ));
    };

    TradeOrders::record_exit(
        pool,
        entry,
        side_str,
        &comment,
        &data.order_id,
        entry.filled_qty,
        entry.filled_qty * last,
    )
    .await
    .map_err(|e| format!("record_exit failed: {e}"))?;

    info!(
        "[ORDER-SWEEP] {} {} exit for order {} -> {}",
        entry.ticker,
        reason.label(),
        entry.id,
        data.order_id
    );
    Ok(())
}

/// `/api/v1/market/allTickers` payload.
#[derive(Debug, Deserialize)]
struct AllTickers {
    ticker: Vec<Ticker>,
}

/// One entry of the `allTickers` list; only `symbol` and `last` are needed.
#[derive(Debug, Deserialize)]
struct Ticker {
    symbol: String,
    last: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn builds_spot_symbol() {
        assert_eq!(spot_symbol("BTC/USDT"), "BTC-USDT");
        assert_eq!(spot_symbol("BTC-USDT"), "BTC-USDT");
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

    #[test]
    fn parses_all_tickers_last_prices() {
        let json = r#"{
            "code": "200000",
            "data": {
                "time": 1700000000000,
                "ticker": [
                    { "symbol": "BTC-USDT", "last": "83600.5" },
                    { "symbol": "SOL-USDT", "last": "112.15" },
                    { "symbol": "BAD-USDT", "last": null }
                ]
            }
        }"#;
        let res: KuCoinResponse<AllTickers> = serde_json::from_str(json).unwrap();
        let data = res.data.unwrap();
        assert_eq!(data.ticker.len(), 3);
        let parsed: HashMap<String, Decimal> = data
            .ticker
            .into_iter()
            .filter_map(|t| {
                let last = t.last.and_then(|l| Decimal::from_str(&l).ok())?;
                Some((t.symbol, last))
            })
            .collect();
        assert_eq!(parsed.get("BTC-USDT"), Some(&dec("83600.5")));
        assert_eq!(parsed.get("SOL-USDT"), Some(&dec("112.15")));
        assert!(!parsed.contains_key("BAD-USDT"));
    }
}
