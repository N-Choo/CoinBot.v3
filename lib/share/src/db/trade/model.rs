use sqlx::FromRow;
use uuid::Uuid;

/// Trade-order persistence operations.
pub struct TradeOrders;

/// An order submitted for a contract, recorded at order level.
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TradeOrder {
    /// Trade order identifier.
    pub id: Uuid,
    /// Contract this order belongs to.
    pub contract_id: Uuid,
    /// Owning user.
    pub user_uid: Uuid,
    /// Trading pair.
    pub ticker: String,
    /// Order side, e.g. `buy` or `sell`.
    pub side: String,
    /// What the order does to the position: `open` or `close`.
    pub action: String,
    /// Comment sent to the exchange, linking back to `contract_id`.
    pub comment: String,
    /// Exchange-assigned order id, once accepted.
    pub exchange_order_id: Option<String>,
    /// Lifecycle status (see [`Status`](super::Status)).
    pub status: String,
    /// Requested quantity as a decimal string.
    pub requested_qty: String,
    /// Reserved fund for this order (quote amount), as a decimal string.
    pub funds: String,
    /// Filled quantity as a decimal string.
    pub filled_qty: String,
    /// Execution price as a decimal string, once filled.
    pub fill_price: Option<String>,
    /// Fee paid as a decimal string.
    pub fee: String,
    /// Failure reason, when the order failed.
    pub reason: Option<String>,
    /// Take-profit price from the originating signal, as a decimal string.
    pub tp_price: Option<String>,
    /// Stop-loss price from the originating signal, as a decimal string.
    pub sl_price: Option<String>,
    /// Exchange order id of the protective OCO exit, once placed.
    pub exit_order_id: Option<String>,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TradeOrders {
    /// Record a new order in `pending` status.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        contract_id: Uuid,
        user_uid: Uuid,
        ticker: &str,
        side: &str,
        action: &str,
        comment: &str,
        requested_qty: &str,
        funds: &str,
        tp_price: &str,
        sl_price: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"INSERT INTO trade_orders (contract_id, user_uid, ticker, side, action, comment, requested_qty, funds, tp_price, sl_price)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING *"#,
        )
        .bind(contract_id)
        .bind(user_uid)
        .bind(ticker)
        .bind(side)
        .bind(action)
        .bind(comment)
        .bind(requested_qty)
        .bind(funds)
        .bind(tp_price)
        .bind(sl_price)
        .fetch_one(pool)
        .await
    }

    /// Mark an order as `open`, recording its exchange order id.
    pub async fn mark_placed(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        exchange_order_id: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET exchange_order_id = $1, status = 'open', updated_at = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(exchange_order_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Record the exchange order id of the protective OCO exit.
    pub async fn mark_exit_order(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        exit_order_id: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET exit_order_id = $1, updated_at = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(exit_order_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Mark an order as `filled`, recording its execution details.
    pub async fn mark_filled(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        filled_qty: &str,
        fill_price: &str,
        fee: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET filled_qty = $1, fill_price = $2, fee = $3, status = 'filled', updated_at = NOW()
               WHERE id = $4
               RETURNING *"#,
        )
        .bind(filled_qty)
        .bind(fill_price)
        .bind(fee)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Mark an order as `cancelled` with a reason.
    pub async fn mark_cancelled(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        reason: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET status = 'cancelled', reason = $1, updated_at = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(reason)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Mark an order as `failed` with a reason.
    pub async fn mark_failed(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        reason: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET status = 'failed', reason = $1, updated_at = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(reason)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Mark an order as `closed` with a reason.
    pub async fn mark_closed(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        reason: &str,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET status = 'closed', reason = $1, updated_at = NOW()
               WHERE id = $2
               RETURNING *"#,
        )
        .bind(reason)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }
}
