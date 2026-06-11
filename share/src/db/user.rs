use sqlx::{PgPool, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub wallet_address: String,
    pub uid: Uuid,
    pub balance: String,
    pub locked_balance: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub async fn find_by_wallet(pool: &PgPool, wallet: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE wallet_address = $1")
            .bind(wallet)
            .fetch_optional(pool)
            .await
    }

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
