//! Reconcile in-flight orders against the exchange and settle their contracts.
//!
//! A background loop that keeps the local `trade_orders` ledger consistent with
//! KuCoin's real order state: it resolves placements whose outcome is unknown
//! (looked up by `clientOid`), then settles every order the exchange reports as
//! final, crediting or releasing contract funds against the fill.

mod exchange;
mod pending;
mod pnl;
mod settle;

use kucoin::client::rest::KuCoinClient;
use sqlx::{Pool, Postgres};

/// Reconcile in-flight orders and settle the contracts they belong to.
pub async fn run(db: Pool<Postgres>, kcc: KuCoinClient) {
    // First resolve placements whose outcome is unknown (crash, transport error),
    // then settle every order the exchange reports as final.
    pending::reconcile(&db, &kcc).await;
    settle::settle_open_orders(&db, &kcc).await;
}
