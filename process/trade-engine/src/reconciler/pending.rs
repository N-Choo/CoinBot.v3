//! Resolve `pending` orders whose placement outcome is unknown.
//!
//! Looks each up on the exchange by its `clientOid`: found => record the
//! exchange id; absent past the grace window => mark failed and release any
//! reserved entry fund. DB writes live in `ledger`.

use chrono::{Duration, Utc};
use kucoin::client::rest::KuCoinClient;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};

use super::exchange::fetch_by_client_oid;
use crate::ledger;
use crate::symbol::margin_symbol;

/// Grace before a `pending` order with no exchange id is treated as unplaced.
const PENDING_GRACE_MINUTES: i64 = 5;
/// Hard cap: even if lookups keep erroring (e.g. a 404 surfacing as an error),
/// give up on a `pending` order past this age so a position is never blocked.
const PENDING_MAX_MINUTES: i64 = 30;

/// Resolve every `pending` order (no exchange id) against the exchange.
pub(super) async fn reconcile(db: &Pool<Postgres>, kcc: &KuCoinClient) {
    let pending = match TradeOrders::pending_orders(db).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("[RECONCILE] pending-orders query failed: {e}");
            return;
        }
    };

    for o in pending {
        let symbol = margin_symbol(&o.ticker);
        match fetch_by_client_oid(kcc, &o.comment, &symbol).await {
            Ok(Some(data)) => {
                if let Err(e) = ledger::record_placed(db, o.id, &data.id).await {
                    log::error!("[RECONCILE] mark_placed failed for pending {}: {e}", o.id);
                }
            }
            Ok(None) => {
                if o.created_at < Utc::now() - Duration::minutes(PENDING_GRACE_MINUTES) {
                    fail_pending(db, &o).await;
                }
            }
            Err(e) => {
                log::warn!("[RECONCILE] client-oid lookup failed for {}: {e}", o.id);
                if o.created_at < Utc::now() - Duration::minutes(PENDING_MAX_MINUTES) {
                    fail_pending(db, &o).await;
                }
            }
        }
    }
}

/// Mark a never-placed order failed, then (separately) return a reserved entry
/// fund. The failure is recorded first, so a release error can never wedge the
/// row in `pending` forever.
async fn fail_pending(db: &Pool<Postgres>, o: &TradeOrder) {
    if let Err(e) = ledger::record_failed(db, o.id, "not placed (reconciled)").await {
        log::error!("[RECONCILE] mark_failed failed for pending {}: {e}", o.id);
        return;
    }
    if o.action == "open"
        && let Err(e) = ledger::release_reserved(db, o.contract_id, o.funds).await
    {
        log::error!(
            "[RECONCILE] release_fund failed for contract {}: {e}",
            o.contract_id
        );
    }
}
