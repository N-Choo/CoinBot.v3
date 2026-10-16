//! Integration tests for the deposit finalize/sweep money guarantees.
//!
//! The KuCoin calls in the deposit sweeper cannot run offline, but the money
//! safety is enforced at the DB level by `Deposit`: confirming is guarded by
//! `status = 'pending'` (credit-once) and the Main -> Trade sweep is claimed
//! exactly once via `transferred_at`. This test mirrors the sweeper's
//! finalize/sweep sequence against a real Postgres.
//!
//! Requires `DATABASE_URL`; skipped when it is unset or unreachable.

use rust_decimal::Decimal;
use share::db::deposit::Deposit;
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

/// A fresh wallet/user with one deposit in `status`, returning both.
async fn make_deposit(pool: &PgPool, status: &str) -> (String, Deposit) {
    let wallet = format!("0xtest{}", Uuid::new_v4().simple());

    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &wallet).await.unwrap();
    tx.commit().await.unwrap();

    let deposit = sqlx::query_as::<_, Deposit>(
        r#"INSERT INTO deposits (user_uid, tx_hash, ticker, amount, from_address, to_address, status)
           VALUES ($1, $2, 'USDT', $3, $4, '0xto', $5)
           RETURNING *"#,
    )
    .bind(user.uid)
    .bind(format!("0xtx{}", Uuid::new_v4().simple()))
    .bind(Decimal::from(10))
    .bind(&wallet)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap();

    (wallet, deposit)
}

async fn cleanup(pool: &PgPool, wallet: &str, deposit_id: Uuid) {
    sqlx::query("DELETE FROM deposits WHERE id = $1")
        .bind(deposit_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE wallet_address = $1")
        .bind(wallet)
        .execute(pool)
        .await
        .unwrap();
}

/// Finalizing a pending deposit credits the user exactly once. The second pass
/// (which mirrors the sweeper's credit-then-confirm transaction) must fail on
/// `confirm` and be rolled back, so the balance does not move again.
#[tokio::test]
async fn confirm_credits_the_user_exactly_once() {
    let Some(pool) = pool().await else { return };
    let (wallet, deposit) = make_deposit(&pool, "pending").await;
    let amount = Decimal::from(25);

    // First finalize: credit + confirm commit together.
    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &deposit.from_address).await.unwrap();
    user.add_balance(&mut tx, amount).await.unwrap();
    Deposit::confirm(&mut tx, deposit.id, amount, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Second finalize: `add_balance` runs, but `confirm` is guarded by
    // `status = 'pending'`, so it errors and the caller rolls back.
    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &deposit.from_address).await.unwrap();
    user.add_balance(&mut tx, amount).await.unwrap();
    let second = Deposit::confirm(&mut tx, deposit.id, amount, None).await;
    assert!(
        second.is_err(),
        "confirming a non-pending deposit must error"
    );
    tx.rollback().await.unwrap();

    let stored = User::find_by_wallet(&pool, &wallet).await.unwrap().unwrap();
    assert_eq!(stored.balance, amount, "balance credited exactly once");

    cleanup(&pool, &wallet, deposit.id).await;
}

/// Claiming a sweep is exclusive: only the first caller wins.
#[tokio::test]
async fn claim_transfer_is_exclusive() {
    let Some(pool) = pool().await else { return };
    let (wallet, deposit) = make_deposit(&pool, "confirmed").await;

    let first = Deposit::claim_transfer(&pool, deposit.id).await.unwrap();
    assert!(first.is_some(), "the first claim wins");
    let second = Deposit::claim_transfer(&pool, deposit.id).await.unwrap();
    assert!(second.is_none(), "a second claim must lose");

    cleanup(&pool, &wallet, deposit.id).await;
}

/// `unswept` lists a confirmed-but-unclaimed deposit and drops it once claimed.
#[tokio::test]
async fn unswept_lists_confirmed_unclaimed_and_drops_after_claim() {
    let Some(pool) = pool().await else { return };
    let (wallet, deposit) = make_deposit(&pool, "confirmed").await;

    let listed = Deposit::unswept(&pool).await.unwrap();
    assert!(
        listed.iter().any(|d| d.id == deposit.id),
        "confirmed, unswept deposit is listed"
    );

    Deposit::claim_transfer(&pool, deposit.id)
        .await
        .unwrap()
        .unwrap();

    let listed = Deposit::unswept(&pool).await.unwrap();
    assert!(
        !listed.iter().any(|d| d.id == deposit.id),
        "a claimed deposit is no longer unswept"
    );

    cleanup(&pool, &wallet, deposit.id).await;
}

/// Releasing a claim returns the deposit to the retryable set, and it can be
/// claimed again.
#[tokio::test]
async fn release_transfer_returns_deposit_to_retryable_set() {
    let Some(pool) = pool().await else { return };
    let (wallet, deposit) = make_deposit(&pool, "confirmed").await;

    Deposit::claim_transfer(&pool, deposit.id)
        .await
        .unwrap()
        .unwrap();
    let listed = Deposit::unswept(&pool).await.unwrap();
    assert!(!listed.iter().any(|d| d.id == deposit.id));

    Deposit::release_transfer(&pool, deposit.id).await.unwrap();

    let listed = Deposit::unswept(&pool).await.unwrap();
    assert!(
        listed.iter().any(|d| d.id == deposit.id),
        "a released deposit is retryable again"
    );
    let reclaimed = Deposit::claim_transfer(&pool, deposit.id).await.unwrap();
    assert!(
        reclaimed.is_some(),
        "a released deposit can be claimed again"
    );

    cleanup(&pool, &wallet, deposit.id).await;
}
