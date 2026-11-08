//! Integration test: repaying a loan charges its interest to the contract.
//!
//! Requires `DATABASE_URL`; skipped when it is unset or unreachable.

use chrono::Utc;
use rust_decimal::Decimal;
use share::db::contracts::ContractFilter;
use share::db::loan::MarginLoan;
use share::db::trade::TradeOrders;
use share::db::user::User;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("../../process/migrations")
        .run(&pool)
        .await
        .ok()?;
    Some(pool)
}

#[tokio::test]
async fn mark_repaid_charges_the_contract_once() {
    let Some(pool) = pool().await else { return };
    let wallet = format!("0xtest{}", Uuid::new_v4().simple());

    let mut tx = pool.begin().await.unwrap();
    let user = User::upsert(&mut tx, &wallet).await.unwrap();
    let contract_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO contracts
               (user_uid, signature, message, nonce, ticker,
                init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status)
           VALUES ($1, 'sig', 'msg', $2, 'SOL/USDT', '1000', '900', '100', 2.0, 5.0, 'active')
           RETURNING id"#,
    )
    .bind(user.uid)
    .bind(format!("loan-{}", Uuid::new_v4()))
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let order = TradeOrders::create(
        &mut tx,
        contract_id,
        user.uid,
        "SOL/USDT",
        "sell",
        "open",
        &format!("entry-{}", Uuid::new_v4()),
        Decimal::from(2),
        Decimal::from(100),
        None,
        None,
    )
    .await
    .unwrap();
    let loan = MarginLoan::create(
        &mut tx,
        contract_id,
        order.id,
        "SOL",
        Decimal::from(2),
        Some("loan-1"),
        Utc::now(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let repaid = MarginLoan::mark_repaid(
        &pool,
        loan.id,
        Decimal::from_str("0.25").unwrap(),
        None,
        Some("repay-1"),
    )
    .await
    .unwrap();
    assert!(repaid.is_some());

    let contract = ContractFilter::new()
        .with_user(user.uid)
        .execute(&pool)
        .await
        .unwrap()[0]
        .clone();
    assert_eq!(contract.borrow_interest, Decimal::from_str("0.25").unwrap());

    // A second call is a no-op and cannot charge twice.
    let again = MarginLoan::mark_repaid(
        &pool,
        loan.id,
        Decimal::from_str("0.25").unwrap(),
        None,
        Some("repay-1"),
    )
    .await
    .unwrap();
    assert!(again.is_none());
    let contract = ContractFilter::new()
        .with_user(user.uid)
        .execute(&pool)
        .await
        .unwrap()[0]
        .clone();
    assert_eq!(contract.borrow_interest, Decimal::from_str("0.25").unwrap());

    sqlx::query("DELETE FROM margin_loans WHERE contract_id = $1")
        .bind(contract_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM trade_orders WHERE contract_id = $1")
        .bind(contract_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM contracts WHERE id = $1")
        .bind(contract_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE wallet_address = $1")
        .bind(&wallet)
        .execute(&pool)
        .await
        .unwrap();
}
