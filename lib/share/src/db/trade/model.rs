use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
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
    /// Client order id sent to the exchange (unique per order attempt).
    pub comment: String,
    /// Exchange-assigned order id, once accepted.
    pub exchange_order_id: Option<String>,
    /// Exchange loan application id, when this order auto-borrowed.
    pub loan_apply_id: Option<String>,
    /// Actual base quantity borrowed by this order, when auto-borrow returned it.
    pub borrow_size: Option<Decimal>,
    /// Lifecycle status (see [`Status`](super::Status)).
    pub status: String,
    /// Requested quantity.
    pub requested_qty: Decimal,
    /// Reserved fund for this order (quote amount).
    pub funds: Decimal,
    /// Filled quantity.
    pub filled_qty: Decimal,
    /// Execution price, once filled.
    pub fill_price: Option<Decimal>,
    /// Fee paid.
    pub fee: Decimal,
    /// Currency the fee was charged in (e.g. `USDT`), when the exchange reports it.
    pub fee_currency: Option<String>,
    /// Realized PnL of the closed round trip (quote currency), net of fees.
    ///
    /// Set on close orders only; entries leave it `NULL`.
    pub pnl: Option<Decimal>,
    /// Failure reason, when the order failed.
    pub reason: Option<String>,
    /// Take-profit price from the originating signal.
    pub tp_price: Option<Decimal>,
    /// Stop-loss price from the originating signal.
    pub sl_price: Option<Decimal>,
    /// Entry order this close order exits, when `action = 'close'`.
    pub parent_order_id: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TradeOrders {
    /// Record a new order in `pending` status within `tx`.
    ///
    /// Callers reserve the order's fund in the *same* transaction, so a committed
    /// `pending` row always has its fund reserved.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        contract_id: Uuid,
        user_uid: Uuid,
        ticker: &str,
        side: &str,
        action: &str,
        comment: &str,
        requested_qty: Decimal,
        funds: Decimal,
        tp_price: Option<Decimal>,
        sl_price: Option<Decimal>,
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
        .fetch_one(&mut **tx)
        .await
    }

    /// Fetch a single order by id.
    pub async fn find(pool: &PgPool, id: Uuid) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>("SELECT * FROM trade_orders WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    /// Load entry orders that are still open positions: filled entries with no
    /// live close (exit) order against them.
    ///
    /// A close order blocks a re-exit only while it is `open`/`filled`, or
    /// `pending` and recent (a crash between reserving and placing self-heals
    /// after the grace window). Failed/cancelled exits therefore retry.
    pub async fn held_entries(pool: &PgPool) -> Result<Vec<TradeOrder>, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"SELECT e.* FROM trade_orders e
               WHERE e.action = 'open' AND e.status = 'filled'
                 AND COALESCE((
                     SELECT SUM(c.filled_qty) FROM trade_orders c
                     WHERE c.parent_order_id = e.id AND c.status = 'filled'
                 ), 0) < e.filled_qty
                 AND NOT EXISTS (
                     SELECT 1 FROM trade_orders c
                     WHERE c.parent_order_id = e.id
                       AND (
                           c.status IN ('pending', 'open')
                           OR (c.status = 'pending' AND c.created_at > NOW() - INTERVAL '5 minutes')
                       )
                 )
               ORDER BY e.created_at"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Reserve an exit for `entry` *before* it is sent to the exchange.
    ///
    /// Inserts a `pending` close row linked via `parent_order_id`, but only when
    /// `entry` is still a filled open position and no live or settled close
    /// exists for it. The `INSERT ... SELECT` check makes this atomic against a
    /// periodic exit loop settling the position between `held_entries` and this
    /// call. Returns `None` when the entry is no longer closable. The caller
    /// marks the row placed or failed.
    pub async fn reserve_exit(
        pool: &PgPool,
        entry: &TradeOrder,
        side: &str,
        comment: &str,
        requested_qty: Decimal,
        funds: Decimal,
    ) -> Result<Option<TradeOrder>, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"INSERT INTO trade_orders
                   (contract_id, user_uid, ticker, side, action, comment, status,
                    requested_qty, funds, parent_order_id)
               SELECT $1, $2, $3, $4, 'close', $5, 'pending', $6, $7, $8
               WHERE EXISTS (
                   SELECT 1 FROM trade_orders p
                   WHERE p.id = $8 AND p.action = 'open' AND p.status = 'filled'
               )
                 AND COALESCE((
                   SELECT SUM(c.filled_qty) FROM trade_orders c
                   WHERE c.parent_order_id = $8 AND c.status = 'filled'
                 ), 0) < (SELECT filled_qty FROM trade_orders WHERE id = $8)
                 AND NOT EXISTS (
                   SELECT 1 FROM trade_orders c
                   WHERE c.parent_order_id = $8
                     AND c.status IN ('pending', 'open')
               )
               ON CONFLICT DO NOTHING
               RETURNING *"#,
        )
        .bind(entry.contract_id)
        .bind(entry.user_uid)
        .bind(&entry.ticker)
        .bind(side)
        .bind(comment)
        .bind(requested_qty)
        .bind(funds)
        .bind(entry.id)
        .fetch_optional(pool)
        .await
    }

    /// Whether `contract_id` has an entry or close still working at the exchange.
    ///
    /// True when an `open` order is `pending`/`open`, or a filled entry has a
    /// live (`pending`/`open`) close. Signals must wait while this is true so a
    /// target-side signal cannot open a second position or hedge a working exit.
    pub async fn in_flight_for_contract(
        pool: &PgPool,
        contract_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS (
                   SELECT 1 FROM trade_orders o
                   WHERE o.contract_id = $1
                     AND (
                         (o.action = 'open' AND o.status IN ('pending', 'open'))
                         OR (
                             o.action = 'open' AND o.status = 'filled'
                             AND EXISTS (
                                 SELECT 1 FROM trade_orders c
                                 WHERE c.parent_order_id = o.id
                                   AND c.status IN ('pending', 'open')
                             )
                         )
                     )
               )"#,
        )
        .bind(contract_id)
        .fetch_one(pool)
        .await
    }

    /// Held entries belonging to one contract.
    ///
    /// Same rules as [`Self::held_entries`], scoped to `contract_id`.
    pub async fn held_entries_for_contract(
        pool: &PgPool,
        contract_id: Uuid,
    ) -> Result<Vec<TradeOrder>, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"SELECT e.* FROM trade_orders e
               WHERE e.contract_id = $1 AND e.action = 'open' AND e.status = 'filled'
                 AND COALESCE((
                     SELECT SUM(c.filled_qty) FROM trade_orders c
                     WHERE c.parent_order_id = e.id AND c.status = 'filled'
                 ), 0) < e.filled_qty
                 AND NOT EXISTS (
                     SELECT 1 FROM trade_orders c
                     WHERE c.parent_order_id = e.id AND c.status IN ('pending', 'open')
                 )
               ORDER BY e.created_at"#,
        )
        .bind(contract_id)
        .fetch_all(pool)
        .await
    }

    /// Total quantity already closed for `entry_id`.
    pub async fn filled_close_qty(pool: &PgPool, entry_id: Uuid) -> Result<Decimal, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(filled_qty), 0) FROM trade_orders WHERE parent_order_id = $1 AND status = 'filled'",
        )
        .bind(entry_id)
        .fetch_one(pool)
        .await
    }

    /// Whether the contract still has an entry with unclosed quantity.
    pub async fn has_open_position_for_contract_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        contract_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM trade_orders e
                   WHERE e.contract_id = $1 AND e.action = 'open' AND e.status = 'filled'
                     AND COALESCE((
                         SELECT SUM(c.filled_qty) FROM trade_orders c
                         WHERE c.parent_order_id = e.id AND c.status = 'filled'
                     ), 0) < e.filled_qty
               )"#,
        )
        .bind(contract_id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Whether `contract_id` has any `pending` or `open` order.
    pub async fn has_unsettled_for_contract(
        pool: &PgPool,
        contract_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM trade_orders WHERE contract_id = $1 AND status IN ('pending','open'))",
        )
        .bind(contract_id)
        .fetch_one(pool)
        .await
    }

    /// Transaction-scoped variant of [`Self::has_unsettled_for_contract`].
    pub async fn has_unsettled_for_contract_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        contract_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM trade_orders WHERE contract_id = $1 AND status IN ('pending','open'))",
        )
        .bind(contract_id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Orders reserved but not yet confirmed placed (no exchange order id).
    ///
    /// Used to reconcile placements whose outcome is unknown (crash, transport
    /// error): look them up by `comment` (clientOid) on the exchange.
    pub async fn pending_orders(pool: &PgPool) -> Result<Vec<TradeOrder>, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"SELECT * FROM trade_orders
               WHERE status = 'pending' AND exchange_order_id IS NULL
               ORDER BY created_at"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Mark an order as `open`, recording its exchange order id and, when
    /// present, the loan application id returned by an auto-borrow.
    pub async fn mark_placed(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        exchange_order_id: &str,
        loan_apply_id: Option<&str>,
        borrow_size: Option<Decimal>,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET exchange_order_id = $1,
                   loan_apply_id = COALESCE($2, loan_apply_id),
                   borrow_size = COALESCE($3, borrow_size),
                   status = 'open', updated_at = NOW()
               WHERE id = $4 AND status = 'pending'
               RETURNING *"#,
        )
        .bind(exchange_order_id)
        .bind(loan_apply_id)
        .bind(borrow_size)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    /// Mark an order as `filled`, recording its execution details.
    ///
    /// `fee_currency` records the currency the fee was charged in (for
    /// quote conversion); `pnl` is set for close orders only.
    pub async fn mark_filled(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        filled_qty: Decimal,
        fill_price: Decimal,
        fee: Decimal,
        fee_currency: Option<&str>,
        pnl: Option<Decimal>,
    ) -> Result<TradeOrder, sqlx::Error> {
        sqlx::query_as::<_, TradeOrder>(
            r#"UPDATE trade_orders
               SET filled_qty = $1, fill_price = $2, fee = $3,
                   fee_currency = $4, pnl = $5, status = 'filled', updated_at = NOW()
               WHERE id = $6 AND status = 'open'
               RETURNING *"#,
        )
        .bind(filled_qty)
        .bind(fill_price)
        .bind(fee)
        .bind(fee_currency)
        .bind(pnl)
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
               WHERE id = $2 AND status = 'open'
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
