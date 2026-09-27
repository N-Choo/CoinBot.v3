use sqlx::FromRow;
use uuid::Uuid;

/// Contract persistence operations.
pub struct Contracts;

/// A signed trading contract.
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct Contract {
    /// Contract identifier.
    pub id: Uuid,
    /// Owning user.
    pub user_uid: Uuid,
    /// Signature authorising the contract.
    pub signature: String,
    /// Signed message payload.
    pub message: String,
    /// One-time nonce used for signing.
    pub nonce: String,
    /// Trading pair, upper-cased.
    pub ticker: String,
    /// Balance snapshot at signing time, as a decimal string.
    pub snap_balance: String,
    /// Initial fund allocated to this contract, as a decimal string.
    pub init_fund: String,
    /// Funds still available to use, as a decimal string.
    pub available_fund: String,
    /// Funds used so far, as a decimal string.
    pub used_fund: String,
    /// Stop-loss distance, in percent.
    pub sl_pct: f32,
    /// Take-profit distance, in percent.
    pub tp_pct: f32,
    /// Lifecycle status (see [`Status`](super::Status)).
    pub status: String,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Contracts {
    /// Insert a new contract in `active` status within `tx`.
    ///
    /// The ticker is upper-cased; the `nonce` is uniquely constrained by the
    /// database. `available_fund` and `used_fund` start at `init_fund` and `0`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_uid: Uuid,
        signature: &str,
        message: &str,
        nonce: &str,
        ticker: &str,
        snap_balance: &str,
        init_fund: &str,
        available_fund: &str,
        used_fund: &str,
        sl_pct: f32,
        tp_pct: f32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, used_fund, sl_pct, tp_pct, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'active')"#,
        )
        .bind(user_uid)
        .bind(signature)
        .bind(message)
        .bind(nonce)
        .bind(ticker.to_uppercase())
        .bind(snap_balance)
        .bind(init_fund)
        .bind(available_fund)
        .bind(used_fund)
        .bind(sl_pct)
        .bind(tp_pct)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Move `amount` from `available_fund` to `used_fund` on a contract.
    ///
    /// Returns [`sqlx::Error::RowNotFound`] when `available_fund` is less than
    /// `amount`, so callers can reject over-spending.
    pub async fn consume_fund(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        amount: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE contracts
               SET available_fund = available_fund::numeric - $1::numeric,
                   used_fund      = used_fund::numeric + $1::numeric,
                   updated_at     = NOW()
               WHERE id = $2 AND available_fund::numeric >= $1::numeric
               RETURNING *"#,
        )
        .bind(amount)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }
}
