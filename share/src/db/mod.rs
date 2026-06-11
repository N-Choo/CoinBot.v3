use sqlx::PgPool;
use uuid::Uuid;

pub mod contracts;
pub mod deposit;
pub mod user;

pub async fn get_uid(pool: &PgPool, wallet: &str) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("SELECT uid FROM users WHERE wallet_address = $1")
        .bind(wallet)
        .fetch_one(pool)
        .await
}
