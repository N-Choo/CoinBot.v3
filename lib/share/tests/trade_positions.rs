//! Integration test for the order sweeper's position bookkeeping.
//!
//! Requires `DATABASE_URL`; skipped when it is unset or unreachable.

use rust_decimal::Decimal;
use share::db::contracts::{ContractFilter, Contracts};
use share::db::trade::TradeOrders;
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

/// A filled open entry shows up in `held_entries`; once a close order is
/// recorded against it, it no longer does (so it cannot be exited twice).
#[tokio::test]
async fn held_entries_tracks_open_positions() {
    let Some(pool) = pool().await else { return };

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

    // A filled open entry with a TP/SL.
    let entry = {
        let mut tx = pool.begin().await.unwrap();
        let entry = TradeOrders::create(
            &mut tx,
            contract.id,
            user.uid,
            "BTC/USDT",
            "buy",
            "open",
            &format!("c-{}", Uuid::new_v4()),
            Decimal::from(1),
            Decimal::from(100),
            Decimal::from(110),
            Decimal::from(90),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        entry
    };
    let mut tx = pool.begin().await.unwrap();
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

    let held = TradeOrders::held_entries(&pool).await.unwrap();
    assert!(
        held.iter().any(|o| o.id == entry.id),
        "filled entry should be a held position"
    );

    // Reserve an exit linked to the entry; it now blocks a duplicate exit.
    let exit = TradeOrders::reserve_exit(
        &pool,
        &entry,
        "sell",
        &format!("x-{}", Uuid::new_v4()),
        Decimal::from(1),
        Decimal::from(110),
    )
    .await
    .unwrap()
    .expect("fresh exit reserved");

    // A second live exit for the same entry must be rejected.
    let duplicate = TradeOrders::reserve_exit(
        &pool,
        &entry,
        "sell",
        &format!("x2-{}", Uuid::new_v4()),
        Decimal::from(1),
        Decimal::from(110),
    )
    .await
    .unwrap();
    assert!(duplicate.is_none(), "a live exit already exists");

    let held = TradeOrders::held_entries(&pool).await.unwrap();
    assert!(
        !held.iter().any(|o| o.id == entry.id),
        "position with a live exit must not be held again"
    );

    let mut tx = pool.begin().await.unwrap();
    TradeOrders::mark_placed(&mut tx, exit.id, "exch-exit")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // A failed exit must not permanently de-protect the position.
    let mut tx = pool.begin().await.unwrap();
    TradeOrders::mark_failed(&mut tx, exit.id, "boom")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let held = TradeOrders::held_entries(&pool).await.unwrap();
    assert!(
        held.iter().any(|o| o.id == entry.id),
        "a failed exit should allow a retry"
    );

    sqlx::query("DELETE FROM trade_orders WHERE contract_id = $1")
        .bind(contract.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM contracts WHERE id = $1")
        .bind(contract.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE wallet_address = $1")
        .bind(&wallet)
        .execute(&pool)
        .await
        .unwrap();
}
