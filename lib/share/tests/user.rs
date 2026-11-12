//! Integration test: a wallet gets an account on first contact.
//!
//! This is the mechanism the deposit sweeper relies on to create an account for
//! a wallet when finalizing its deposit. Requires `DATABASE_URL`; skipped when
//! it is unset, and fails if set but unreachable.

use share::db::user::User;
use sqlx::PgPool;

/// Connect to the test database and apply migrations, or `None` when
/// `DATABASE_URL` is unset (fails if set but unreachable).
async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url)
        .await
        .expect("DATABASE_URL is set but Postgres is unreachable");
    sqlx::migrate!("../../process/migrations")
        .run(&pool)
        .await
        .expect("database migrations failed");
    Some(pool)
}

#[tokio::test]
async fn new_wallet_gets_an_account() {
    let Some(pool) = pool().await else { return };

    let wallet = format!("0xtest{}", uuid::Uuid::new_v4().simple());

    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &wallet).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(user.wallet_address, wallet);
    assert_eq!(user.balance, rust_decimal::Decimal::ZERO);
    assert!(User::find_by_wallet(&pool, &wallet)
        .await
        .unwrap()
        .is_some());

    // Idempotent: upserting an existing wallet returns the same account.
    let mut tx = pool.begin().await.unwrap();
    let again = User::upsert(&mut tx, &wallet).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(again.uid, user.uid);

    sqlx::query("DELETE FROM users WHERE wallet_address = $1")
        .bind(&wallet)
        .execute(&pool)
        .await
        .unwrap();
}
