//! Integration tests for the withdrawal ledger transitions.
//!
//! Require `DATABASE_URL`; skipped when it is unset or unreachable.

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use share::db::user::User;
use share::db::withdrawals::Withdrawals;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

/// Serialise tests in this file: `expire_safe` scans all rows globally.
static DB_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("../../process/migrations")
        .run(&pool)
        .await
        .ok()?;
    Some(pool)
}

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

async fn user_with(pool: &PgPool, balance: Decimal) -> User {
    let wallet = format!("0xwd{}", Uuid::new_v4().simple());
    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &wallet).await.unwrap();
    let user = user.add_balance(&mut tx, balance).await.unwrap();
    tx.commit().await.unwrap();
    user
}

async fn cleanup(pool: &PgPool, uid: Uuid) {
    sqlx::query("DELETE FROM ledger_entries WHERE user_uid = $1")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM withdrawals WHERE user_uid = $1")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE uid = $1")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
}

async fn create(
    pool: &PgPool,
    user: &User,
    expiry: chrono::DateTime<Utc>,
) -> share::db::withdrawals::Withdrawal {
    Withdrawals::create_pending(
        pool,
        user.uid,
        &user.wallet_address,
        dec("10"),
        dec("1"),
        dec("11"),
        &format!("n-{}", Uuid::new_v4()),
        "msg",
        "sig",
        expiry,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn create_locks_total_and_expiry_releases_once() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;

    // An already-expired intent exercises the release path.
    let withdrawal = create(&pool, &user, Utc::now() - Duration::minutes(1)).await;
    assert_eq!(withdrawal.status, "pending");

    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, dec("89"));
    assert_eq!(after.locked_balance, dec("11"));

    assert!(Withdrawals::expire_safe(&pool, 120)
        .await
        .unwrap()
        .iter()
        .any(|row| row.id == withdrawal.id));
    assert!(!Withdrawals::expire_safe(&pool, 120)
        .await
        .unwrap()
        .iter()
        .any(|row| row.id == withdrawal.id));

    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, dec("100"));
    assert_eq!(after.locked_balance, Decimal::ZERO);

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn completion_consumes_the_lock_and_applies_the_fee_delta() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() + Duration::minutes(5)).await;

    // Claim this row directly so parallel tests cannot pick it up first.
    sqlx::query(
        "UPDATE withdrawals SET status = 'processing', claimed_at = NOW()
           WHERE id = $1 AND status = 'pending'",
    )
    .bind(withdrawal.id)
    .execute(&pool)
    .await
    .unwrap();
    let claimed = Withdrawals::find(&pool, withdrawal.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.id, withdrawal.id);
    Withdrawals::mark_attempt(&pool, claimed.id, dec("10"), dec("1"))
        .await
        .unwrap();
    Withdrawals::mark_submitted(&pool, claimed.id, "kc-1")
        .await
        .unwrap();

    // Actual fee 0.5: the 0.5 difference is refunded to available balance.
    Withdrawals::mark_completed(&pool, claimed.id, dec("0.5"), Some("0xhash"))
        .await
        .unwrap();

    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, dec("89.5"));
    assert_eq!(after.locked_balance, Decimal::ZERO);

    // A second call must not release or settle again.
    Withdrawals::mark_completed(&pool, claimed.id, dec("0.5"), Some("0xhash"))
        .await
        .unwrap();
    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, dec("89.5"));

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn expiry_respects_the_claim_lease() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() - Duration::minutes(1)).await;

    // Simulate a worker that has just claimed the row.
    sqlx::query("UPDATE withdrawals SET status = 'processing', claimed_at = NOW() WHERE id = $1")
        .bind(withdrawal.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        !Withdrawals::expire_safe(&pool, 120)
            .await
            .unwrap()
            .iter()
            .any(|row| row.id == withdrawal.id),
        "an active claim must not be expired"
    );
    assert!(
        Withdrawals::expire_safe(&pool, 0)
            .await
            .unwrap()
            .iter()
            .any(|row| row.id == withdrawal.id),
        "an expired claim lease must be releasable"
    );

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn mark_attempt_reports_a_lost_transition() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() + Duration::minutes(5)).await;

    sqlx::query("UPDATE withdrawals SET status = 'processing', claimed_at = NOW() WHERE id = $1")
        .bind(withdrawal.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        Withdrawals::mark_attempt(&pool, withdrawal.id, dec("10"), dec("1"))
            .await
            .unwrap()
    );

    sqlx::query("UPDATE withdrawals SET status = 'failed' WHERE id = $1")
        .bind(withdrawal.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !Withdrawals::mark_attempt(&pool, withdrawal.id, dec("10"), dec("1"))
            .await
            .unwrap()
    );

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn counts_active_and_recent_rows() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    create(&pool, &user, Utc::now() + Duration::minutes(5)).await;
    create(&pool, &user, Utc::now() + Duration::minutes(5)).await;

    assert_eq!(
        Withdrawals::count_active_for_user(&pool, user.uid)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        Withdrawals::count_since_for_user(&pool, user.uid, 24)
            .await
            .unwrap(),
        2
    );

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn failure_releases_the_lock_once() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() + Duration::minutes(5)).await;

    Withdrawals::mark_failed(&pool, withdrawal.id, "test failure")
        .await
        .unwrap();
    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, dec("100"));
    assert_eq!(after.locked_balance, Decimal::ZERO);

    Withdrawals::mark_failed(&pool, withdrawal.id, "again")
        .await
        .unwrap();
    let again = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.balance, dec("100"));

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn expiry_never_touches_attempted_rows() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() - Duration::minutes(1)).await;

    sqlx::query(
        "UPDATE withdrawals
            SET status = 'processing', claimed_at = NOW(), attempted_at = NOW()
          WHERE id = $1",
    )
    .bind(withdrawal.id)
    .execute(&pool)
    .await
    .unwrap();

    assert!(!Withdrawals::expire_safe(&pool, 0)
        .await
        .unwrap()
        .iter()
        .any(|row| row.id == withdrawal.id));

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn negative_fee_delta_is_clamped_to_the_balance() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let withdrawal = create(&pool, &user, Utc::now() + Duration::minutes(5)).await;

    sqlx::query("UPDATE withdrawals SET status = 'processing', claimed_at = NOW() WHERE id = $1")
        .bind(withdrawal.id)
        .execute(&pool)
        .await
        .unwrap();
    Withdrawals::mark_attempt(&pool, withdrawal.id, dec("10"), dec("1"))
        .await
        .unwrap();
    Withdrawals::mark_submitted(&pool, withdrawal.id, "kc-clamp")
        .await
        .unwrap();

    // Actual fee 200 vs quoted 1: delta -199, but only 89 is available after
    // the lock. The platform absorbs the remainder instead of failing the row.
    Withdrawals::mark_completed(&pool, withdrawal.id, dec("200"), Some("0xhash"))
        .await
        .unwrap();

    let after = User::find_by_wallet(&pool, &user.wallet_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.balance, Decimal::ZERO);
    assert_eq!(after.locked_balance, Decimal::ZERO);

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn create_requires_enough_balance() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("10")).await;

    let result = Withdrawals::create_pending(
        &pool,
        user.uid,
        &user.wallet_address,
        dec("10"),
        dec("1"),
        dec("11"),
        &format!("n-{}", Uuid::new_v4()),
        "msg",
        "sig",
        Utc::now() + Duration::minutes(5),
    )
    .await;
    assert!(matches!(result, Err(sqlx::Error::RowNotFound)));

    cleanup(&pool, user.uid).await;
}

#[tokio::test]
async fn a_nonce_cannot_be_reused() {
    let _guard = DB_LOCK.lock().await;
    let Some(pool) = pool().await else { return };
    let user = user_with(&pool, dec("100")).await;
    let nonce = format!("n-{}", Uuid::new_v4());

    let first = Withdrawals::create_pending(
        &pool,
        user.uid,
        &user.wallet_address,
        dec("10"),
        dec("1"),
        dec("11"),
        &nonce,
        "msg",
        "sig",
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();
    assert_eq!(first.nonce, nonce);

    let second = Withdrawals::create_pending(
        &pool,
        user.uid,
        &user.wallet_address,
        dec("10"),
        dec("1"),
        dec("11"),
        &nonce,
        "msg",
        "sig",
        Utc::now() + Duration::minutes(5),
    )
    .await;
    assert!(matches!(second, Err(sqlx::Error::Database(_))));

    cleanup(&pool, user.uid).await;
}
