use rust_decimal::Decimal;
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
    /// Initial fund allocated to this contract.
    pub init_fund: Decimal,
    /// Funds still available to use.
    pub available_fund: Decimal,
    /// Cumulative funds traded on this contract.
    pub total_traded_fund: Decimal,
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
    /// database. `available_fund` starts at `init_fund` and `total_traded_fund`
    /// at `0`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_uid: Uuid,
        signature: &str,
        message: &str,
        nonce: &str,
        ticker: &str,
        init_fund: Decimal,
        available_fund: Decimal,
        total_traded_fund: Decimal,
        sl_pct: f32,
        tp_pct: f32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO contracts (user_uid, signature, message, nonce, ticker, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'active')"#,
        )
        .bind(user_uid)
        .bind(signature)
        .bind(message)
        .bind(nonce)
        .bind(ticker.to_uppercase())
        .bind(init_fund)
        .bind(available_fund)
        .bind(total_traded_fund)
        .bind(sl_pct)
        .bind(tp_pct)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Move `amount` from `available_fund` to `total_traded_fund` on a contract.
    ///
    /// Returns [`sqlx::Error::RowNotFound`] when `available_fund` is less than
    /// `amount`, so callers can reject over-spending.
    pub async fn consume_fund(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        amount: Decimal,
    ) -> Result<Contract, sqlx::Error> {
        sqlx::query_as::<_, Contract>(
            r#"UPDATE contracts
               SET available_fund    = available_fund - $1,
                   total_traded_fund = total_traded_fund + $1,
                   updated_at        = NOW()
               WHERE id = $2 AND available_fund >= $1
               RETURNING *"#,
        )
        .bind(amount)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Credit `amount` to a contract's `available_fund`.
    pub async fn add_fund(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        amount: Decimal,
    ) -> Result<Contract, sqlx::Error> {
        sqlx::query_as::<_, Contract>(
            r#"UPDATE contracts
               SET available_fund = available_fund + $1,
                   updated_at     = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(amount)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Move `amount` from `total_traded_fund` back to `available_fund`.
    ///
    /// Returns [`sqlx::Error::RowNotFound`] when `total_traded_fund` is less
    /// than `amount`.
    pub async fn release_fund(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        amount: Decimal,
    ) -> Result<Contract, sqlx::Error> {
        sqlx::query_as::<_, Contract>(
            r#"UPDATE contracts
               SET available_fund    = available_fund + $1,
                   total_traded_fund = total_traded_fund - $1,
                   updated_at        = NOW()
               WHERE id = $2 AND total_traded_fund >= $1
               RETURNING *"#,
        )
        .bind(amount)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }
}
