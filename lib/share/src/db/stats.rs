use rust_decimal::Decimal;
use sqlx::PgPool;

/// Aggregate platform metrics for the public stats strip.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct SystemStats {
    pub accounts: i64,
    pub active_contracts: i64,
    pub funds_deployed: Decimal,
    pub trades_executed: i64,
}

/// Read the aggregate system metrics in a single query.
pub async fn system_stats(pool: &PgPool) -> Result<SystemStats, sqlx::Error> {
    sqlx::query_as::<_, SystemStats>(
        r#"SELECT
             (SELECT COUNT(*) FROM users)                             AS accounts,
             (SELECT COUNT(*) FROM contracts WHERE status = 'active') AS active_contracts,
             (SELECT COALESCE(SUM(init_fund), 0)
                FROM contracts WHERE status = 'active')               AS funds_deployed,
             (SELECT COUNT(*) FROM trade_orders)                      AS trades_executed"#,
    )
    .fetch_one(pool)
    .await
}

/// Per-user balance stats for the dashboard.
///
/// `realized_pnl` is a placeholder (`0`) until PnL tracking exists.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct UserStats {
    pub total: Decimal,
    pub free: Decimal,
    pub locked: Decimal,
    pub realized_pnl: Decimal,
}

/// Read a user's balance stats by wallet address.
pub async fn user_stats(pool: &PgPool, wallet: &str) -> Result<Option<UserStats>, sqlx::Error> {
    sqlx::query_as::<_, UserStats>(
        r#"SELECT
             (balance + locked_balance) AS total,
             balance                    AS free,
             locked_balance             AS locked,
             0::numeric                 AS realized_pnl
           FROM users WHERE wallet_address = $1"#,
    )
    .bind(wallet)
    .fetch_optional(pool)
    .await
}
