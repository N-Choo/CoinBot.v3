//! Atomic order operations. Each function is one transaction.

use rust_decimal::Decimal;
use share::db::contracts::Contract;
use share::db::trade::{TradeOrder, TradeOrders};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Result, funds};

/// Exchange-reported fill for an order.
pub struct Fill<'a> {
    pub deal_size: Decimal,
    pub fill_price: Decimal,
    pub fee: Decimal,
    pub fee_currency: Option<&'a str>,
    pub pnl: Option<Decimal>,
}

/// Record a new `pending` open order and reserve its fund in one transaction, so a
/// committed `pending` row always has its fund reserved (the reconciler relies on
/// this to release only what was actually reserved).
#[allow(clippy::too_many_arguments)]
pub async fn create_pending_open(
    pool: &PgPool,
    c: &Contract,
    side: &str,
    comment: &str,
    quantity: Decimal,
    fund: Decimal,
    tp_price: Option<Decimal>,
    sl_price: Option<Decimal>,
) -> Result<TradeOrder> {
    let mut tx = pool.begin().await?;
    let order = TradeOrders::create(
        &mut tx, c.id, c.user_uid, &c.ticker, side, "open", comment, quantity, fund, tp_price,
        sl_price,
    )
    .await?;
    // Dropping `tx` on error rolls back both the row and the reservation.
    funds::reserve(&mut tx, c.id, fund).await?;
    tx.commit().await?;
    Ok(order)
}

/// Reserve a `close` row linked to `entry`. `Ok(None)` when a live exit already
/// exists for this position. No fund moves: a close returns cash on fill.
pub async fn reserve_exit(
    pool: &PgPool,
    entry: &TradeOrder,
    side: &str,
    comment: &str,
    quantity: Decimal,
    notional: Decimal,
) -> Result<Option<TradeOrder>> {
    TradeOrders::reserve_exit(pool, entry, side, comment, quantity, notional).await
}

/// Record the exchange id of an order once the exchange accepts it.
pub async fn record_placed(pool: &PgPool, id: Uuid, exchange_order_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_placed(&mut tx, id, exchange_order_id).await?;
    tx.commit().await
}

/// Mark an order failed. Does not touch funds; pair with [`release_reserved`]
/// when an entry that reserved a fund never reached the exchange.
pub async fn record_failed(pool: &PgPool, id: Uuid, reason: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_failed(&mut tx, id, reason).await?;
    tx.commit().await
}

/// Return a fund reserved by an order that will never be placed.
pub async fn release_reserved(pool: &PgPool, contract_id: Uuid, amount: Decimal) -> Result<()> {
    let mut tx = pool.begin().await?;
    funds::release(&mut tx, contract_id, amount).await?;
    tx.commit().await
}

/// Mark an order cancelled with no fill. An entry also releases its reserved
/// fund; a cancelled exit returns nothing.
pub async fn record_cancelled(pool: &PgPool, order: &TradeOrder) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_cancelled(&mut tx, order.id, "no fill (cancelled)").await?;
    if order.action == "open" {
        funds::release(&mut tx, order.contract_id, order.funds).await?;
    }
    tx.commit().await
}

/// Mark an order filled. `finalize` (close only) is credited to the contract in
/// the same transaction.
pub async fn record_filled(
    pool: &PgPool,
    order: &TradeOrder,
    fill: &Fill<'_>,
    finalize: Option<Decimal>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    TradeOrders::mark_filled(
        &mut tx,
        order.id,
        fill.deal_size,
        fill.fill_price,
        fill.fee,
        fill.fee_currency,
        fill.pnl,
    )
    .await?;
    if let Some(amount) = finalize {
        funds::finalize(&mut tx, order.contract_id, amount).await?;
    }
    tx.commit().await
}

#[cfg(test)]
mod tests {
    //! DB-backed tests for the money-moving ledger operations.
    //!
    //! Require `DATABASE_URL`; each test returns early (skips) when it is unset
    //! or unreachable. Every test uses a unique wallet/nonce and deletes the
    //! rows it created, so tests are order-independent.

    use super::*;
    use share::db::contracts::{ContractFilter, Contracts};
    use share::db::user::User;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        sqlx::migrate!("../migrations").run(&pool).await.ok()?;
        Some(pool)
    }

    struct Fixture {
        pool: PgPool,
        wallet: String,
        contract: Contract,
    }

    impl Fixture {
        async fn reload_contract(&self) -> Contract {
            ContractFilter::new()
                .with_user(self.contract.user_uid)
                .execute(&self.pool)
                .await
                .unwrap()
                .into_iter()
                .next()
                .unwrap()
        }

        async fn cleanup(&self) {
            sqlx::query("DELETE FROM trade_orders WHERE contract_id = $1")
                .bind(self.contract.id)
                .execute(&self.pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM contracts WHERE id = $1")
                .bind(self.contract.id)
                .execute(&self.pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM users WHERE wallet_address = $1")
                .bind(&self.wallet)
                .execute(&self.pool)
                .await
                .unwrap();
        }
    }

    /// A fresh user + contract funded with `init_fund`, or `None` without a DB.
    async fn setup(init_fund: Decimal) -> Option<Fixture> {
        let pool = test_pool().await?;
        let wallet = format!("0xtest{}", Uuid::new_v4().simple());
        let mut tx = pool.begin().await.unwrap();
        let user = User::upsert(&mut tx, &wallet).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        Contracts::create(
            &mut tx,
            user.uid,
            "0xsig",
            "msg",
            &format!("nonce-{}", Uuid::new_v4()),
            "BTC/USDT",
            init_fund,
            init_fund,
            Decimal::ZERO,
            2.0,
            5.0,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let contract = ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        Some(Fixture {
            pool,
            wallet,
            contract,
        })
    }

    fn comment(prefix: &str) -> String {
        format!("{prefix}-{}", Uuid::new_v4())
    }

    /// Insert a close order for the fixture contract and mark it `open`.
    async fn place_close(fx: &Fixture) -> TradeOrder {
        let mut tx = fx.pool.begin().await.unwrap();
        let close = TradeOrders::create(
            &mut tx,
            fx.contract.id,
            fx.contract.user_uid,
            &fx.contract.ticker,
            "sell",
            "close",
            &comment("close"),
            Decimal::from(1),
            Decimal::ZERO,
            Some(Decimal::from(120)),
            Some(Decimal::from(90)),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, close.id, "exch-close")
            .await
            .unwrap();
        tx.commit().await.unwrap();
        close
    }

    fn fill(pnl: Decimal) -> Fill<'static> {
        Fill {
            deal_size: Decimal::from(1),
            fill_price: Decimal::from(120),
            fee: Decimal::from(2),
            fee_currency: Some("USDT"),
            pnl: Some(pnl),
        }
    }

    /// A close fill marks the order filled *and* credits the contract by the
    /// committed fund plus realized PnL, atomically.
    #[tokio::test]
    async fn record_filled_close_marks_filled_and_credits_contract() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let close = place_close(&fx).await;

        record_filled(
            &fx.pool,
            &close,
            &fill(Decimal::from(18)),
            Some(Decimal::from(118)),
        )
        .await
        .unwrap();

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "filled");
        assert_eq!(stored.filled_qty, Decimal::from(1));
        assert_eq!(stored.fill_price, Some(Decimal::from(120)));
        assert_eq!(stored.fee, Decimal::from(2));
        assert_eq!(stored.fee_currency.as_deref(), Some("USDT"));
        assert_eq!(stored.pnl, Some(Decimal::from(18)));

        let contract = fx.reload_contract().await;
        assert_eq!(
            contract.available_fund,
            Decimal::from(1118),
            "close fill credits the contract"
        );
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        fx.cleanup().await;
    }

    /// If the fund finalize fails, the order write it shares a transaction with
    /// is rolled back too — no half-applied fill.
    #[tokio::test]
    async fn record_filled_rolls_back_order_write_when_finalize_fails() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let close = place_close(&fx).await;

        // Point the order struct at a contract that does not exist: `mark_filled`
        // still targets the real row (`close.id`), but `add_fund` on the bogus
        // contract returns `RowNotFound`, aborting the whole transaction.
        let mut bogus = close.clone();
        bogus.contract_id = Uuid::new_v4();

        let err = record_filled(
            &fx.pool,
            &bogus,
            &fill(Decimal::from(18)),
            Some(Decimal::from(118)),
        )
        .await;
        assert!(
            err.is_err(),
            "finalize against a missing contract must error"
        );

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "open", "order write must be rolled back");
        let contract = fx.reload_contract().await;
        assert_eq!(
            contract.available_fund,
            Decimal::from(1000),
            "fund must be unchanged on rollback"
        );

        fx.cleanup().await;
    }

    /// A cancelled entry releases the fund it reserved.
    #[tokio::test]
    async fn record_cancelled_entry_releases_reserved_fund() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &comment("entry"),
            Decimal::from(1),
            Decimal::from(100),
            Some(Decimal::from(110)),
            Some(Decimal::from(90)),
        )
        .await
        .unwrap();

        let reserved = fx.reload_contract().await;
        assert_eq!(reserved.available_fund, Decimal::from(900));
        assert_eq!(reserved.total_traded_fund, Decimal::from(100));

        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        record_cancelled(&fx.pool, &entry).await.unwrap();

        let stored = TradeOrders::find(&fx.pool, entry.id).await.unwrap();
        assert_eq!(stored.status, "cancelled");
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1000));
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        fx.cleanup().await;
    }

    /// A cancelled exit moves no funds.
    #[tokio::test]
    async fn record_cancelled_exit_changes_no_funds() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let close = place_close(&fx).await;

        record_cancelled(&fx.pool, &close).await.unwrap();

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "cancelled");
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1000));
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        fx.cleanup().await;
    }

    /// Creating a pending open order reserves its fund in the same transaction.
    #[tokio::test]
    async fn create_pending_open_reserves_fund() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let order = create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &comment("entry"),
            Decimal::from(2),
            Decimal::from(100),
            Some(Decimal::from(110)),
            Some(Decimal::from(90)),
        )
        .await
        .unwrap();

        assert_eq!(order.status, "pending");
        assert_eq!(order.funds, Decimal::from(100));
        assert_eq!(order.requested_qty, Decimal::from(2));

        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(900));
        assert_eq!(contract.total_traded_fund, Decimal::from(100));

        fx.cleanup().await;
    }

    /// Reserving more than the contract holds fails and leaves no pending row.
    #[tokio::test]
    async fn create_pending_open_rolls_back_when_reservation_fails() {
        let Some(fx) = setup(Decimal::from(50)).await else {
            return;
        };
        let marker = comment("entry");
        let err = create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &marker,
            Decimal::from(1),
            Decimal::from(100),
            Some(Decimal::from(110)),
            Some(Decimal::from(90)),
        )
        .await;
        assert!(err.is_err(), "over-reserving must error");

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trade_orders WHERE comment = $1")
            .bind(&marker)
            .fetch_one(&fx.pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "no pending row may survive a failed reservation");

        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(50));
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        fx.cleanup().await;
    }

    /// A failed reservation is released exactly once.
    #[tokio::test]
    async fn failed_reserved_entry_is_released_exactly_once() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &comment("entry"),
            Decimal::from(1),
            Decimal::from(100),
            Some(Decimal::from(110)),
            Some(Decimal::from(90)),
        )
        .await
        .unwrap();

        record_failed(&fx.pool, entry.id, "never reached exchange")
            .await
            .unwrap();
        let stored = TradeOrders::find(&fx.pool, entry.id).await.unwrap();
        assert_eq!(stored.status, "failed");

        release_reserved(&fx.pool, fx.contract.id, Decimal::from(100))
            .await
            .unwrap();
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1000));
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        // The reservation is gone, so a second release cannot move funds again.
        let second = release_reserved(&fx.pool, fx.contract.id, Decimal::from(100)).await;
        assert!(second.is_err(), "second release must fail");
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1000));

        fx.cleanup().await;
    }
}
