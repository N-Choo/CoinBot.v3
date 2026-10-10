use sqlx::PgPool;
use uuid::Uuid;

use crate::db::trade::status::{Action, Status};

use super::model::TradeOrder;

/// Builder for querying [`TradeOrder`] rows.
///
/// Filters are combined with `AND` and results are ordered by `created_at DESC`.
#[derive(Default)]
pub struct TradeOrderFilter {
    user_uid: Option<Uuid>,
    contract_id: Option<Uuid>,
    ticker: Option<String>,
    status: Option<String>,
    action: Option<String>,
    side: Option<String>,
}

impl TradeOrderFilter {
    /// Create an empty filter matching all trade orders.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to orders owned by `uid`.
    pub fn with_user(mut self, uid: Uuid) -> Self {
        self.user_uid = Some(uid);
        self
    }

    /// Restrict to orders for a given `contract_id`.
    pub fn with_contract(mut self, contract_id: Uuid) -> Self {
        self.contract_id = Some(contract_id);
        self
    }

    /// Restrict to orders for `ticker` (upper-cased).
    pub fn with_ticker(mut self, ticker: &str) -> Self {
        self.ticker = Some(ticker.to_uppercase());
        self
    }

    /// Restrict to orders with the given lifecycle `status`.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = Some(status.to_string());
        self
    }

    /// Restrict to orders with the given `action` (open/close).
    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action.to_string());
        self
    }

    /// Restrict to orders with the given `side` (buy/sell, case-insensitive).
    pub fn with_side(mut self, side: &str) -> Self {
        self.side = Some(side.to_lowercase());
        self
    }

    /// Run the query, returning matching orders newest-first.
    pub async fn execute(&self, pool: &PgPool) -> Result<Vec<TradeOrder>, sqlx::Error> {
        use sqlx::QueryBuilder;
        let mut builder = QueryBuilder::new("SELECT * FROM trade_orders WHERE 1=1");

        if let Some(ref uid) = self.user_uid {
            builder.push(" AND user_uid = ").push_bind(uid);
        }
        if let Some(ref cid) = self.contract_id {
            builder.push(" AND contract_id = ").push_bind(cid);
        }
        if let Some(ref ticker) = self.ticker {
            builder.push(" AND ticker = ").push_bind(ticker);
        }
        if let Some(ref status) = self.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(ref action) = self.action {
            builder.push(" AND action = ").push_bind(action);
        }
        if let Some(ref side) = self.side {
            builder.push(" AND side = ").push_bind(side);
        }

        builder.push(" ORDER BY created_at DESC");
        builder.build_query_as().fetch_all(pool).await
    }
}
