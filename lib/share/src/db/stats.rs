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
/// `realized_pnl` is the sum of fee-net PnL over the user's closed orders.
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
             (u.balance + u.locked_balance) AS total,
             u.balance                      AS free,
             u.locked_balance               AS locked,
             COALESCE(
                 (SELECT SUM(t.pnl) FROM trade_orders t WHERE t.user_uid = u.uid),
                 0
             )                              AS realized_pnl
           FROM users u WHERE u.wallet_address = $1"#,
    )
    .bind(wallet)
    .fetch_optional(pool)
    .await
}
