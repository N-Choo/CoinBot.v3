//! Background sweeper: reconcile open orders against the exchange and settle
//! their contracts.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::KuCoinResponse;
use serde::Deserialize;
use share::ProcessError;
use share::db::contracts::Contracts;
use share::db::trade::{TradeOrderFilter, TradeOrders};
use sqlx::{Pool, Postgres};

/// Sync open trade orders against the exchange and settle their contracts.
///
/// Loads every `open` order in a single query (each order already carries its
/// `contract_id`, `ticker`, and reserved `funds`), then reconciles them one by
/// one. Fund settlements target the order's own contract.
pub async fn sync_contract_status(db: Pool<Postgres>, kcc: KuCoinClient) {
    let orders = match TradeOrderFilter::new()
        .with_status(share::db::trade::Status::Open)
        .execute(&db)
        .await
    {
        Ok(orders) => orders,
        Err(e) => {
            log::error!("[SWEEP] open-orders query failed: {e}");
            return;
        }
    };

    for o in orders {
        let Some(order_id) = o.exchange_order_id.as_deref() else {
            continue;
        };
        let symbol = spot_symbol(&o.ticker);

        match fetch_order(&kcc, order_id, &symbol).await {
            Ok(Some(data)) => {
                match classify_order(data.active, &data.deal_size, &data.deal_funds) {
                    // Still working — nothing to settle yet.
                    OrderOutcome::Working => continue,

                    // Done with no fill => cancelled.
                    OrderOutcome::Cancelled => {
                        match db.begin().await {
                            Ok(mut tx) => {
                                if let Err(e) = TradeOrders::mark_cancelled(
                                    &mut tx,
                                    o.id,
                                    "no fill (cancelled)",
                                )
                                .await
                                {
                                    log::error!(
                                        "[SWEEP] mark_cancelled failed for order {}: {e}",
                                        o.id
                                    );
                                    continue;
                                }

                                // Return the reserved funds to the contract.
                                if let Err(e) =
                                    Contracts::release_fund(&mut tx, o.contract_id, &o.funds).await
                                {
                                    log::error!(
                                        "[SWEEP] release_fund failed for contract {}: {e}",
                                        o.contract_id
                                    );
                                    continue;
                                }
                                if let Err(e) = tx.commit().await {
                                    log::error!("[SWEEP] commit failed for order {}: {e}", o.id);
                                }
                            }
                            Err(e) => {
                                log::error!("[SWEEP] could not begin tx for order {}: {e}", o.id)
                            }
                        }
                        continue;
                    }

                    OrderOutcome::Filled { avg } => {
                        match db.begin().await {
                            Ok(mut tx) => {
                                if let Err(e) = TradeOrders::mark_filled(
                                    &mut tx,
                                    o.id,
                                    &data.deal_size,
                                    &avg.to_string(),
                                    &data.fee,
                                )
                                .await
                                {
                                    log::error!(
                                        "[SWEEP] mark_filled failed for order {}: {e}",
                                        o.id
                                    );
                                    continue;
                                }

                                // Credit the filled proceeds back to the contract.
                                if let Err(e) =
                                    Contracts::add_fund(&mut tx, o.contract_id, &data.deal_funds)
                                        .await
                                {
                                    log::error!(
                                        "[SWEEP] add_fund failed for contract {}: {e}",
                                        o.contract_id
                                    );
                                    continue;
                                }

                                if let Err(e) = tx.commit().await {
                                    log::error!("[SWEEP] commit failed for order {}: {e}", o.id);
                                    continue;
                                }

                                log::info!(
                                    "[SWEEP] {} order {} filled {} @ {avg} (fee {})",
                                    symbol,
                                    o.id,
                                    data.deal_size,
                                    data.fee
                                );
                            }
                            Err(e) => {
                                log::error!("[SWEEP] could not begin tx for order {}: {e}", o.id)
                            }
                        }
                    }
                }
            }
            Ok(None) => continue,
            Err(e) => {
                log::warn!("[SWEEP] get_order failed for order {}: {e}", o.id);
                continue;
            }
        }
    }
}

/// Subset of a KuCoin order status needed for reconciliation.
///
/// Deliberately lenient: only `active` is required; the rest default, so a
/// `null` in any other response field cannot fail decoding.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderStatus {
    active: bool,
    #[serde(default)]
    deal_size: String,
    #[serde(default)]
    deal_funds: String,
    #[serde(default)]
    fee: String,
}

/// Fetch `order_id`'s reconciliation fields from KuCoin without the strict
/// `SpotDatum` schema; unrelated fields (which may be `null`) are ignored.
async fn fetch_order(
    kcc: &KuCoinClient,
    order_id: &str,
    symbol: &str,
) -> Result<Option<OrderStatus>, ProcessError> {
    let endpoint = format!("/api/v1/hf/orders/{order_id}?symbol={symbol}");
    let res = kcc
        .send::<KuCoinResponse<OrderStatus>>("GET", "", &endpoint)
        .await
        .map_err(|e| ProcessError::Network(format!("get_order failed: {e}")))?;
    Ok(res.data)
}

/// KuCoin spot symbol: `"BTC/USDT"` -> `"BTC-USDT"`.
fn spot_symbol(ticker: &str) -> String {
    ticker.replace('/', "-")
}

/// Outcome of reconciling one exchange order.
#[derive(Debug, PartialEq)]
enum OrderOutcome {
    /// Still working on the exchange.
    Working,
    /// Finished with no fill (cancelled).
    Cancelled,
    /// Filled at `avg` (deal funds / deal size).
    Filled { avg: f64 },
}

/// Classify an exchange order's reconciliation outcome.
fn classify_order(active: bool, deal_size: &str, deal_funds: &str) -> OrderOutcome {
    if active {
        return OrderOutcome::Working;
    }
    let filled: f64 = deal_size.parse().unwrap_or(0.0);
    if filled <= 0.0 {
        return OrderOutcome::Cancelled;
    }
    let funds: f64 = deal_funds.parse().unwrap_or(0.0);
    OrderOutcome::Filled {
        avg: funds / filled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_spot_symbol() {
        assert_eq!(spot_symbol("BTC/USDT"), "BTC-USDT");
        assert_eq!(spot_symbol("BTC-USDT"), "BTC-USDT");
    }

    #[test]
    fn tolerates_null_unrelated_fields() {
        let json = r#"{
            "active": false,
            "dealSize": "0.072",
            "dealFunds": "7.95",
            "fee": "0.008",
            "visibleSize": null,
            "tax": null,
            "channel": null,
            "clientOid": null
        }"#;
        let status: OrderStatus = serde_json::from_str(json).expect("lenient decode");
        assert!(!status.active);
        assert_eq!(status.deal_size, "0.072");
        assert_eq!(status.deal_funds, "7.95");
        assert_eq!(status.fee, "0.008");
    }

    #[test]
    fn defaults_missing_reconcile_fields() {
        let status: OrderStatus = serde_json::from_str(r#"{"active": true}"#).unwrap();
        assert!(status.active);
        assert_eq!(status.deal_size, "");
        assert_eq!(status.deal_funds, "");
        assert_eq!(status.fee, "");
    }

    #[test]
    fn classifies_working_order() {
        assert_eq!(classify_order(true, "0", "0"), OrderOutcome::Working);
        assert_eq!(classify_order(true, "5", "100"), OrderOutcome::Working);
    }

    #[test]
    fn classifies_unfilled_as_cancelled() {
        assert_eq!(classify_order(false, "0", "0"), OrderOutcome::Cancelled);
        assert_eq!(classify_order(false, "abc", "0"), OrderOutcome::Cancelled);
        assert_eq!(classify_order(false, "-1", "0"), OrderOutcome::Cancelled);
    }

    #[test]
    fn classifies_filled_and_computes_average() {
        assert_eq!(
            classify_order(false, "2", "100"),
            OrderOutcome::Filled { avg: 50.0 }
        );
        assert_eq!(
            classify_order(false, "2", "abc"),
            OrderOutcome::Filled { avg: 0.0 }
        );
    }
}
