use sqlx::PgPool;
use uuid::Uuid;

use super::model::Deposit;

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
