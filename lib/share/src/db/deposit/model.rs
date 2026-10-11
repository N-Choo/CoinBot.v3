use ethers::types::Transaction as EthTx;
use sqlx::{PgPool, Transaction};
use uuid::Uuid;

use crate::erc20::Erc20;

/// A verified on-chain USDT deposit.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Deposit {
    /// Deposit identifier (also the ticket id).
    pub id: Uuid,
    /// Owning user.
    pub user_uid: Uuid,
    /// Ethereum transaction hash.
    pub tx_hash: String,
    /// Deposited ticker (e.g. `USDT`).
    pub ticker: String,
    /// Deposited amount as a decimal string.
    pub amount: String,
    /// Sender address.
    pub from_address: String,
    /// Recipient address.
    pub to_address: String,
    /// Block the transaction was mined in, once confirmed.
    pub block_number: Option<i64>,
    /// Lifecycle status (see [`Status`](super::Status)).
    pub status: String,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the deposit was confirmed or failed.
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Failure reason, when the deposit failed.
    pub reason: Option<String>,
}

impl Deposit {
    /// Insert a new `pending` deposit from an on-chain transaction.
    ///
    /// The amount is decoded from the ERC-20 transfer calldata.
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

    /// Mark a `pending` deposit as `confirmed`.
    ///
    /// Records the swept `amount` and block number; returns the updated row.
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

    /// Mark a `pending` deposit as `failed` with a reason.
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
