//! `/api/contracts/me` tests. Require a reachable Redis; database-backed
//! cases are skipped when `DATABASE_URL` is unset.

use actix_web::{App, cookie::Cookie, http::StatusCode, test, web};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{NonceCache, SessionCache};
use crate::routes::api_routes;

/// Redis base URL from `REDIS_URL`, defaulting to localhost.
fn redis_base() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// Build a session cache (Redis database 0).
async fn setup_session_cache() -> SessionCache {
    let url = format!("{}/0", redis_base().trim_end_matches('/'));
    SessionCache::new(&url)
        .await
        .expect("Redis required for tests")
}

/// Build a nonce cache (Redis database 1).
async fn setup_nonce_cache() -> NonceCache {
    let url = format!("{}/1", redis_base().trim_end_matches('/'));
    NonceCache::new(&url)
        .await
        .expect("Redis required for tests")
}

/// A lazy pool pointing at a non-existent database.
fn dummy_pool() -> PgPool {
    PgPool::connect_lazy("postgresql://localhost:5432/nonexistent").expect("dummy pool")
}

/// A lazy pool for `DATABASE_URL`, or `None` when it is not configured.
fn real_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect_lazy(&url).ok()
}

/// Insert (or reuse) a user for `wallet` and return its uid.
async fn upsert_user(pool: &PgPool, wallet: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO users (wallet_address) VALUES ($1) ON CONFLICT (wallet_address) DO NOTHING",
    )
    .bind(wallet)
    .execute(pool)
    .await
    .expect("insert user");
    sqlx::query_scalar::<_, Uuid>("SELECT uid FROM users WHERE wallet_address = $1")
        .bind(wallet)
        .fetch_one(pool)
        .await
        .expect("select uid")
}

/// Listing contracts without a session returns `401`.
#[actix_web::test]
async fn test_me_unauthorized() {
    let session_cache = setup_session_cache().await;
    let nonce_cache = setup_nonce_cache().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dummy_pool()))
            .app_data(web::Data::new(session_cache))
            .app_data(web::Data::new(nonce_cache))
            .configure(api_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/contracts/me")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// A session whose wallet has no user returns an empty `200`.
#[actix_web::test]
async fn test_me_no_user_returns_empty() {
    let Some(pool) = real_pool() else { return };
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    let session_cache = setup_session_cache().await;
    let nonce_cache = setup_nonce_cache().await;
    let wallet = "0x0000000000000000000000000000000000000000";
    session_cache
        .insert("test_me_no_user_token".into(), wallet.into())
        .await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(session_cache))
            .app_data(web::Data::new(nonce_cache))
            .configure(api_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/contracts/me")
        .cookie(Cookie::new("session_token", "test_me_no_user_token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body, serde_json::json!([]));
}

/// The endpoint returns only the caller's contracts, without signing material.
#[actix_web::test]
async fn test_me_returns_only_owners_contracts() {
    let Some(pool) = real_pool() else { return };
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    let owner_wallet = "0x00000000000000000000000000000000000000a1";
    let other_wallet = "0x00000000000000000000000000000000000000a2";
    let owner_nonces = ["test_me_owner_1", "test_me_owner_2"];
    let other_nonce = "test_me_other_1";

    // Idempotent: clear any rows left by a prior run before inserting.
    for nonce in [owner_nonces[0], owner_nonces[1], other_nonce] {
        sqlx::query("DELETE FROM contracts WHERE nonce = $1")
            .bind(nonce)
            .execute(&pool)
            .await
            .expect("pre-clean contract");
    }

    let owner_uid = upsert_user(&pool, owner_wallet).await;
    let other_uid = upsert_user(&pool, other_wallet).await;

    for (uid, nonce, ticker) in [
        (owner_uid, owner_nonces[0], "BTC/USDT"),
        (owner_uid, owner_nonces[1], "ETH/USDT"),
        (other_uid, other_nonce, "SOL/USDT"),
    ] {
        sqlx::query(
            r#"INSERT INTO contracts
                   (user_uid, signature, message, nonce, ticker, snap_balance,
                    init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status)
                   VALUES ($1, '0xsig', $2, $3, $4, '1000', '1000', '1000', '0', 2.0, 5.0, 'active')
                   ON CONFLICT (nonce) DO NOTHING"#,
        )
        .bind(uid)
        .bind(nonce)
        .bind(nonce)
        .bind(ticker)
        .execute(&pool)
        .await
        .expect("insert contract");
    }

    let session_cache = setup_session_cache().await;
    let nonce_cache = setup_nonce_cache().await;
    session_cache
        .insert("test_me_owner_token".into(), owner_wallet.into())
        .await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(session_cache))
            .app_data(web::Data::new(nonce_cache))
            .configure(api_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/contracts/me")
        .cookie(Cookie::new("session_token", "test_me_owner_token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(body.len(), 2, "only the owner's contracts: {body:?}");
    for row in &body {
        let ticker = row["ticker"].as_str().unwrap();
        assert!(
            matches!(ticker, "BTC/USDT" | "ETH/USDT"),
            "unexpected ticker {ticker}"
        );
        assert!(row.get("signature").is_none());
        assert!(row.get("message").is_none());
        assert!(row.get("nonce").is_none());
        assert_eq!(row["available_fund"].as_str().unwrap(), "1000");
        assert_eq!(row["sl_pct"].as_f64().unwrap(), 2.0);
        assert!(row.get("user_uid").is_none());
        assert!(row.get("snap_balance").is_none());
    }

    for nonce in [owner_nonces[0], owner_nonces[1], other_nonce] {
        sqlx::query("DELETE FROM contracts WHERE nonce = $1")
            .bind(nonce)
            .execute(&pool)
            .await
            .expect("cleanup contract");
    }
    for wallet in [owner_wallet, other_wallet] {
        sqlx::query("DELETE FROM users WHERE wallet_address = $1")
            .bind(wallet)
            .execute(&pool)
            .await
            .expect("cleanup user");
    }
}
