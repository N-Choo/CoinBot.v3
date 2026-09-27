//! PostgreSQL models and query filters for users, contracts, deposits, and trade orders.
use sqlx::PgPool;
use uuid::Uuid;

pub mod contracts;
pub mod deposit;
pub mod trade;
pub mod user;

/// Look up a user's UUID by wallet address.
///
/// Returns [`sqlx::Error::RowNotFound`] when no user has that wallet.
pub async fn get_uid(pool: &PgPool, wallet: &str) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("SELECT uid FROM users WHERE wallet_address = $1")
        .bind(wallet)
        .fetch_one(pool)
        .await
}

/// Look up a user's wallet address by UUID.
///
/// Returns [`sqlx::Error::RowNotFound`] when no user has that uid.
pub async fn wallet_for_uid(pool: &PgPool, uid: Uuid) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT wallet_address FROM users WHERE uid = $1")
        .bind(uid)
        .fetch_one(pool)
        .await
}
