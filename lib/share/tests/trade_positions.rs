//! Integration tests for the order sweeper's position bookkeeping.
//!
//! Requires `DATABASE_URL`; skipped when it is unset or unreachable.

use rust_decimal::Decimal;
use share::db::contracts::{Contract, ContractFilter, Contracts};
use share::db::trade::{TradeOrder, TradeOrders};
use share::db::user::User;
use sqlx::PgPool;
use uuid::Uuid;

/// Connect to the test database and apply migrations, or `None` when
/// `DATABASE_URL` is unset/unreachable.
async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("../../process/migrations")
        .run(&pool)
        .await
        .ok()?;
    Some(pool)
}

/// A fresh user + contract, cleaned up at the end of each test.
struct Fixture {
    pool: PgPool,
    wallet: String,
    contract: Contract,
}

impl Fixture {
    async fn new(pool: PgPool) -> Self {
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
            Decimal::from(1000),
            Decimal::from(1000),
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

        Self {
            pool,
            wallet,
            contract,
        }
    }

    fn comment(prefix: &str) -> String {
        format!("{prefix}-{}", Uuid::new_v4())
    }

    /// Create a `pending` open entry.
    async fn pending_entry(&self, side: &str) -> TradeOrder {
        let mut tx = self.pool.begin().await.unwrap();
        let entry = TradeOrders::create(
            &mut tx,
            self.contract.id,
            self.contract.user_uid,
            &self.contract.ticker,
            side,
            "open",
            &Self::comment("c"),
            Decimal::from(1),
            Decimal::from(100),
            Some(Decimal::from(110)),
            Some(Decimal::from(90)),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        entry
    }

    /// Create a `filled` open entry with no close order.
    async fn filled_entry(&self, side: &str) -> TradeOrder {
        let entry = self.pending_entry(side).await;

        let mut tx = self.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
            .await
            .unwrap();
        TradeOrders::mark_filled(
            &mut tx,
            entry.id,
            Decimal::from(1),
            Decimal::from(100),
            Decimal::ZERO,
            None,
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        TradeOrders::find(&self.pool, entry.id).await.unwrap()
    }

    async fn reserve_exit(&self, entry: &TradeOrder) -> Option<TradeOrder> {
        TradeOrders::reserve_exit(
            &self.pool,
            entry,
            "sell",
            &Self::comment("x"),
            Decimal::from(1),
            Decimal::from(110),
        )
        .await
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

/// A filled open entry shows up in `held_entries`; once a close order is
/// recorded against it, it no longer does (so it cannot be exited twice).
#[tokio::test]
async fn held_entries_tracks_open_positions() {
    let Some(pool) = pool().await else { return };
    let fx = Fixture::new(pool).await;
    let entry = fx.filled_entry("buy").await;

    let held = TradeOrders::held_entries(&fx.pool).await.unwrap();
    assert!(
        held.iter().any(|o| o.id == entry.id),
        "filled entry should be a held position"
    );

    // Reserve an exit linked to the entry; it now blocks a duplicate exit.
    let exit = fx.reserve_exit(&entry).await.expect("fresh exit reserved");

    // A second live exit for the same entry must be rejected.
    let duplicate = fx.reserve_exit(&entry).await;
    assert!(duplicate.is_none(), "a live exit already exists");

    let held = TradeOrders::held_entries(&fx.pool).await.unwrap();
    assert!(
        !held.iter().any(|o| o.id == entry.id),
        "position with a live exit must not be held again"
    );

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_placed(&mut tx, exit.id, "exch-exit")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // A failed exit must not permanently de-protect the position.
    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_failed(&mut tx, exit.id, "boom")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let held = TradeOrders::held_entries(&fx.pool).await.unwrap();
    assert!(
        held.iter().any(|o| o.id == entry.id),
        "a failed exit should allow a retry"
    );

    fx.cleanup().await;
}

/// Once a close settles `filled`, the entry is not closable again even if the
/// caller still holds a stale `held_entries` snapshot.
#[tokio::test]
async fn reserve_exit_refuses_after_a_filled_close() {
    let Some(pool) = pool().await else { return };
    let fx = Fixture::new(pool).await;
    let entry = fx.filled_entry("buy").await;

    let exit = fx.reserve_exit(&entry).await.expect("fresh exit reserved");
    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_placed(&mut tx, exit.id, "exch-exit")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_filled(
        &mut tx,
        exit.id,
        Decimal::from(1),
        Decimal::from(110),
        Decimal::ZERO,
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let duplicate = fx.reserve_exit(&entry).await;
    assert!(
        duplicate.is_none(),
        "a settled close must block a second exit"
    );

    fx.cleanup().await;
}

/// The in-flight guard reports pending/open entries and filled entries with a
/// working close, and clears once the close settles.
#[tokio::test]
async fn in_flight_tracks_entries_and_working_closes() {
    let Some(pool) = pool().await else { return };
    let fx = Fixture::new(pool).await;

    let entry = fx.pending_entry("buy").await;
    assert!(
        TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "pending entry is in flight"
    );

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "open entry is in flight"
    );

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_filled(
        &mut tx,
        entry.id,
        Decimal::from(1),
        Decimal::from(100),
        Decimal::ZERO,
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(
        !TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "filled entry with no close is not in flight"
    );

    let entry = TradeOrders::find(&fx.pool, entry.id).await.unwrap();
    let close = fx.reserve_exit(&entry).await.expect("fresh exit reserved");
    assert!(
        TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "pending close is in flight"
    );

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_placed(&mut tx, close.id, "exch-exit")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "open close is in flight"
    );

    let mut tx = fx.pool.begin().await.unwrap();
    TradeOrders::mark_filled(
        &mut tx,
        close.id,
        Decimal::from(1),
        Decimal::from(110),
        Decimal::ZERO,
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(
        !TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "settled close is no longer in flight"
    );

    // The close-then-open boundary: once the reversal close settles, the old
    // entry is no longer held and nothing is in flight, so a fresh target-side
    // open may be reserved for the contract.
    let held = TradeOrders::held_entries(&fx.pool).await.unwrap();
    assert!(
        !held.iter().any(|o| o.id == entry.id),
        "settled close leaves no held position"
    );
    let replacement = fx.pending_entry("sell").await;
    assert!(
        TradeOrders::in_flight_for_contract(&fx.pool, fx.contract.id)
            .await
            .unwrap(),
        "the replacement open is now in flight"
    );
    assert_eq!(replacement.side, "sell");

    fx.cleanup().await;
}
