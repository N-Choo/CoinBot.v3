use sqlx::PgPool;
use uuid::Uuid;

use crate::db::contracts::status::Status;

use super::model::Contract;

/// Builder for querying [`Contract`] rows.
///
/// Filters are combined with `AND` and results are ordered by `created_at DESC`.
#[derive(Default)]
pub struct ContractFilter {
    user_uid: Option<Uuid>,
    status: Option<String>,
    ticker: Option<String>,
}

impl ContractFilter {
    /// Create an empty filter matching all contracts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to contracts owned by `uid`.
    pub fn with_user(mut self, uid: Uuid) -> Self {
        self.user_uid = Some(uid);
        self
    }

    /// Restrict to contracts with the given lifecycle `status`.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = Some(status.to_string());
        self
    }

    /// Restrict to contracts for `ticker` (upper-cased).
    pub fn with_ticker(mut self, ticker: &str) -> Self {
        self.ticker = Some(ticker.to_uppercase());
        self
    }

    /// Run the query, returning matching contracts newest-first.
    pub async fn execute(&self, pool: &PgPool) -> Result<Vec<Contract>, sqlx::Error> {
        use sqlx::QueryBuilder;
        let mut builder = QueryBuilder::new("SELECT * FROM contracts WHERE 1=1");

        if let Some(ref uid) = self.user_uid {
            builder.push(" AND user_uid = ").push_bind(uid);
        }
        if let Some(ref status) = self.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(ref ticker) = self.ticker {
            builder.push(" AND ticker = ").push_bind(ticker);
        }

        builder.push(" ORDER BY created_at DESC");
        builder.build_query_as().fetch_all(pool).await
    }

    /// Run the query, returning the distinct tickers matching the filter.
    pub async fn execute_tickers(&self, pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
        use sqlx::QueryBuilder;
        let mut builder = QueryBuilder::new("SELECT DISTINCT ticker FROM contracts WHERE 1=1");

        if let Some(ref uid) = self.user_uid {
            builder.push(" AND user_uid = ").push_bind(uid);
        }
        if let Some(ref status) = self.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(ref ticker) = self.ticker {
            builder.push(" AND ticker = ").push_bind(ticker);
        }

        builder.build_query_scalar().fetch_all(pool).await
    }
}
