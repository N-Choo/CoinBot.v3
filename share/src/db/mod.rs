use sqlx::PgPool;
use uuid::Uuid;

pub mod deposit;

pub async fn get_uid(pool: &PgPool, wallet: &str) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("SELECT uid FROM users WHERE LOWER(wallet_address) = LOWER($1)")
        .bind(wallet)
        .fetch_one(pool)
        .await
}
