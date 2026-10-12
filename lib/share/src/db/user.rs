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

    /// Look up a user's UUID by wallet address.
    ///
    /// Returns [`sqlx::Error::RowNotFound`] when no user has that wallet.
    pub async fn uid_for_wallet(pool: &PgPool, wallet: &str) -> Result<Uuid, sqlx::Error> {
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

    /// Insert the user if absent and return the row.
    pub async fn upsert(
        tx: &mut Transaction<'_, sqlx::Postgres>,
        wallet: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO users (wallet_address)
               VALUES ($1)
               ON CONFLICT (wallet_address) DO UPDATE SET wallet_address = EXCLUDED.wallet_address
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
}
