//! Integration test: `user_stats.realized_pnl` sums persisted order PnL.
//!
//! Requires `DATABASE_URL`; skipped when it is unset or unreachable.

use rust_decimal::Decimal;
use share::db::contracts::{ContractFilter, Contracts};
use share::db::stats::user_stats;
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

/// `realized_pnl` is the sum of close-order `pnl`, ignoring entries (which keep
/// a `NULL` pnl).
#[tokio::test]
async fn realized_pnl_sums_close_orders_and_ignores_entries() {
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
        "XRP/USDT",
        Decimal::from(100),
        Decimal::from(100),
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

    // A filled entry (pnl NULL) and two closes with realized PnL (10.5 + 2.0).
    insert_order(&pool, contract.id, user.uid, "open", None).await;
    insert_order(
        &pool,
        contract.id,
        user.uid,
        "close",
        Some(Decimal::new(105, 1)),
    )
    .await;
    insert_order(
        &pool,
        contract.id,
        user.uid,
        "close",
        Some(Decimal::new(20, 1)),
    )
    .await;

    let stats = user_stats(&pool, &wallet).await.unwrap().unwrap();
    assert_eq!(stats.realized_pnl, Decimal::new(125, 1)); // 12.5, NULL ignored

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

/// Insert a minimal trade order row with an optional realized `pnl`.
async fn insert_order(
    pool: &PgPool,
    contract_id: Uuid,
    user_uid: Uuid,
    action: &str,
    pnl: Option<Decimal>,
) {
    let side = if action == "close" { "buy" } else { "sell" };
    sqlx::query(
        r#"INSERT INTO trade_orders
             (contract_id, user_uid, ticker, side, action, comment,
              requested_qty, funds, fee, filled_qty, pnl)
           VALUES ($1, $2, 'XRP/USDT', $3, $4, $5, 1, 1, 0, 1, $6)"#,
    )
    .bind(contract_id)
    .bind(user_uid)
    .bind(side)
    .bind(action)
    .bind(format!("c-{}", Uuid::new_v4()))
    .bind(pnl)
    .execute(pool)
    .await
    .unwrap();
}
