use ethers::types::Transaction as EthTx;
use sqlx::{PgPool, Transaction};
use uuid::Uuid;

use crate::erc20::Erc20;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Deposit {
    pub id: Uuid,
    pub user_uid: Uuid,
    pub tx_hash: String,
    pub ticker: String,
    pub amount: String,
    pub from_address: String,
    pub to_address: String,
    pub block_number: Option<i64>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: Option<String>,
}

impl Deposit {
    pub async fn create_pending(
        pool: &PgPool,
        user_uid: Uuid,
        tx_hash: &str,
        tx: &EthTx,
        ticker: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO deposits (user_uid, tx_hash, from_address, to_address, ticker, amount)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(user_uid)
        .bind(tx_hash)
        .bind(format!("0x{:x}", tx.from))
        .bind(format!("0x{:x}", tx.to.unwrap_or_default()))
        .bind(ticker)
        .bind(Erc20::decode_amount(&tx.input))
        .fetch_one(pool)
        .await
    }

    pub async fn confirm(
        tx: &mut Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        amount: &str,
        block_nr: Option<i64>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE deposits
            SET amount = $1, block_number = $2, status = 'confirmed', processed_at = NOW()
            WHERE id = $3 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(amount)
        .bind(block_nr)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn fail(
        tx: &mut Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        reason: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE deposits
            SET status = 'failed', reason = $1, processed_at = NOW()
            WHERE id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(reason)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }
}
