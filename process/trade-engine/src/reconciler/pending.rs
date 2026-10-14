//! Resolve `pending` orders whose placement outcome is unknown.
//!
//! Looks each up on the exchange by its `clientOid`: found => record the
//! exchange id; absent past the grace window => mark failed and release any
//! reserved entry fund.

use chrono::{Duration, Utc};
use kucoin::client::rest::KuCoinClient;
use share::db::contracts::Contracts;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use super::exchange::fetch_by_client_oid;
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
                if let Err(e) = place_pending(db, o.id, &data.id).await {
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

/// Record a reserved order's exchange id once the exchange confirms it exists.
async fn place_pending(db: &Pool<Postgres>, id: Uuid, order_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    TradeOrders::mark_placed(&mut tx, id, order_id).await?;
    tx.commit().await
}

/// Mark a never-placed order failed, then (separately) return a reserved entry
/// fund. Marking failed in its own transaction first means a release error can
/// never wedge the row in `pending` forever.
async fn fail_pending(db: &Pool<Postgres>, o: &TradeOrder) {
    match db.begin().await {
        Ok(mut tx) => {
            if let Err(e) = TradeOrders::mark_failed(&mut tx, o.id, "not placed (reconciled)").await
            {
                log::error!("[RECONCILE] mark_failed failed for pending {}: {e}", o.id);
                return;
            }
            if let Err(e) = tx.commit().await {
                log::error!("[RECONCILE] commit failed for pending {}: {e}", o.id);
                return;
            }
        }
        Err(e) => {
            log::error!("[RECONCILE] could not begin tx for pending {}: {e}", o.id);
            return;
        }
    }

    if o.action == "open" {
        match db.begin().await {
            Ok(mut tx) => {
                if let Err(e) = Contracts::release_fund(&mut tx, o.contract_id, o.funds).await {
                    log::error!(
                        "[RECONCILE] release_fund failed for contract {}: {e}",
                        o.contract_id
                    );
                    return;
                }
                if let Err(e) = tx.commit().await {
                    log::error!("[RECONCILE] commit failed releasing fund for {}: {e}", o.id);
                }
            }
            Err(e) => log::error!("[RECONCILE] could not begin release tx for {}: {e}", o.id),
        }
    }
}
