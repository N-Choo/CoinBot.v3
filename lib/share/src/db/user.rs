use sqlx::{PgPool, Transaction};
use uuid::Uuid;

/// A platform user, identified by wallet address.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    /// Wallet address (primary key).
    pub wallet_address: String,
    /// Stable internal identifier.
    pub uid: Uuid,
    /// Available balance, stored as a decimal string.
    pub balance: String,
    /// Balance reserved by open contracts, stored as a decimal string.
    pub locked_balance: String,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Fetch a user by wallet address, or `None` if not registered.
    pub async fn find_by_wallet(pool: &PgPool, wallet: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE wallet_address = $1")
            .bind(wallet)
            .fetch_optional(pool)
            .await
    }

    /// Insert the user if absent and return the row.
    pub async fn upsert(
        tx: &mut Transaction<'_, sqlx::Postgres>,
        wallet: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO users (wallet_address)
               VALUES ($1)
               ON CONFLICT (wallet_address) DO NOTHING
               RETURNING *"#,
        )
        .bind(wallet)
        .fetch_one(&mut **tx)
        .await
    }

    /// Credit `amount` to the user's available balance.
    pub async fn add_balance(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        amount: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE users
               SET balance = balance::numeric + $1::numeric
               WHERE wallet_address = $2
               RETURNING *"#,
        )
        .bind(amount)
        .bind(&self.wallet_address)
        .fetch_one(&mut **tx)
        .await
    }

    /// Move `amount` from available to locked balance.
    ///
    /// Fails (no row) when the available balance is insufficient.
    pub async fn lock_balance(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        amount: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE users
               SET balance = balance::numeric - $1::numeric,
                   locked_balance = locked_balance::numeric + $1::numeric
               WHERE wallet_address = $2 AND balance::numeric >= $1::numeric
               RETURNING *"#,
        )
        .bind(amount)
        .bind(&self.wallet_address)
        .fetch_one(&mut **tx)
        .await
    }

    /// Check that a user's balance covers `amount`.
    ///
    /// Returns an error message when the user is missing, the stored balance is
    /// corrupt, the amount is invalid, or funds are insufficient.
    pub async fn check_funds(pool: &PgPool, uid: Uuid, amount: &str) -> Result<(), String> {
        let has: String = sqlx::query_scalar("SELECT balance FROM users WHERE uid = $1")
            .bind(uid)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        let balance: f64 = has
            .parse()
            .map_err(|_| format!("Corrupt balance value: {}", has))?;
        let required: f64 = amount
            .parse()
            .map_err(|_| format!("Invalid amount: {}", amount))?;

        if balance >= required {
            Ok(())
        } else {
            Err("Insufficient funds".into())
        }
    }

    /// Move `amount` from locked back to available balance.
    ///
    /// Fails (no row) when the locked balance is insufficient.
    pub async fn unlock_balance(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        amount: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE users
               SET balance = balance::numeric + $1::numeric,
                   locked_balance = locked_balance::numeric - $1::numeric
               WHERE wallet_address = $2 AND locked_balance::numeric >= $1::numeric
               RETURNING *"#,
        )
        .bind(amount)
        .bind(&self.wallet_address)
        .fetch_one(&mut **tx)
        .await
    }
}
