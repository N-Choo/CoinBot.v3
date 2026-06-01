use ethers::types::Transaction as EthTx;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Default)]
pub struct DepositFilter {
    user_uid: Option<Uuid>,
    status: Option<String>,
    tx_hash: Option<String>,
}

impl DepositFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user(mut self, uid: Uuid) -> Self {
        self.user_uid = Some(uid);
        self
    }

    pub fn with_status(mut self, status: &str) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_tx_hash(mut self, hash: &str) -> Self {
        self.tx_hash = Some(hash.into());
        self
    }

    pub async fn execute(&self, pool: &PgPool) -> Result<Vec<Deposit>, sqlx::Error> {
        use sqlx::QueryBuilder;
        let mut builder = QueryBuilder::new("SELECT * FROM deposits WHERE 1=1");

        if let Some(ref uid) = self.user_uid {
            builder.push(" AND user_uid = ").push_bind(uid);
        }
        if let Some(ref status) = self.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(ref hash) = self.tx_hash {
            builder.push(" AND tx_hash = ").push_bind(hash);
        }

        builder.push(" ORDER BY created_at DESC");
        builder.build_query_as().fetch_all(pool).await
    }
}

impl Deposit {
    pub async fn create_pending(
        pool: &PgPool,
        user_uid: Uuid,
        tx_hash: &str,
        tx: &EthTx,
        ticker: &str,
    ) -> Result<Self, sqlx::Error> {
        let amount = if ticker == "ETH" {
            tx.value.to_string()
        } else {
            "0".into()
        };

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
        .bind(amount)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_status(pool: &PgPool, status: &str) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM deposits WHERE status = $1 ORDER BY created_at")
            .bind(status)
            .fetch_all(pool)
            .await
    }

    pub async fn confirm(
        pool: &PgPool,
        id: Uuid,
        amount: &str,
        block_nr: i64,
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
        .fetch_one(pool)
        .await
    }

    pub async fn fail(pool: &PgPool, id: Uuid, reason: &str) -> Result<Self, sqlx::Error> {
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
        .fetch_one(pool)
        .await
    }
}
