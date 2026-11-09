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
    /// Cumulative USDT value of borrow interest charged to this contract.
    pub borrow_interest: Decimal,
    /// Platform performance fee taken at settlement.
    pub platform_fee: Decimal,
    /// Net amount credited to the user at settlement.
    pub net_payout: Option<Decimal>,
    /// Final status the settle loop must apply (`cancelled`/`completed`).
    pub settlement_target: Option<String>,
    /// When the contract was settled.
    pub settled_at: Option<chrono::DateTime<chrono::Utc>>,
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
               WHERE id = $2 AND status = 'active' AND available_fund >= $1
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

    /// Fetch a contract only when it belongs to `user_uid`.
    pub async fn find_for_user(
        pool: &sqlx::PgPool,
        id: Uuid,
        user_uid: Uuid,
    ) -> Result<Option<Contract>, sqlx::Error> {
        sqlx::query_as::<_, Contract>("SELECT * FROM contracts WHERE id = $1 AND user_uid = $2")
            .bind(id)
            .bind(user_uid)
            .fetch_optional(pool)
            .await
    }

    /// Atomically move an owned active/inactive contract to `closing`.
    ///
    /// Returns `None` when the contract is missing, foreign, or already
    /// closing/terminal.
    pub async fn set_closing(
        pool: &sqlx::PgPool,
        id: Uuid,
        user_uid: Uuid,
    ) -> Result<Option<Contract>, sqlx::Error> {
        sqlx::query_as::<_, Contract>(
            r#"UPDATE contracts
               SET status = 'closing', settlement_target = 'cancelled', updated_at = NOW()
               WHERE id = $1 AND user_uid = $2 AND status IN ('active', 'inactive')
               RETURNING *"#,
        )
        .bind(id)
        .bind(user_uid)
        .fetch_optional(pool)
        .await
    }

    /// Lock a contract row for settlement.
    pub async fn find_for_update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<Option<Contract>, sqlx::Error> {
        sqlx::query_as::<_, Contract>("SELECT * FROM contracts WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
    }

    /// Apply the terminal status and payout fields. Requires `status = 'closing'`.
    pub async fn finalize(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        target: &str,
        fee: Decimal,
        payout: Decimal,
    ) -> Result<Contract, sqlx::Error> {
        sqlx::query_as::<_, Contract>(
            r#"UPDATE contracts
               SET status = $1,
                   available_fund = 0,
                   platform_fee = $2,
                   net_payout = $3,
                   settled_at = NOW(),
                   updated_at = NOW()
               WHERE id = $4 AND status = 'closing'
               RETURNING *"#,
        )
        .bind(target)
        .bind(fee)
        .bind(payout)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }
}
