//! Login, logout, and session-verification tests. Require a reachable Redis.

use crate::{
    auth::{NonceCache, SessionCache},
    handlers::auth::AuthController,
    models::auth::VerifySignatureRequest,
};

use actix_web::{
    Responder,
    cookie::Cookie,
    http::{StatusCode, header::SET_COOKIE},
    test, web,
};
use ethers::{
    core::rand::{SeedableRng, rngs::StdRng},
    signers::{LocalWallet, Signer},
};
use sqlx::PgPool;

/// Redis base URL from `REDIS_URL`, defaulting to localhost.
fn redis_base() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// A lazy pool pointing at a non-existent database (never queried).
fn dummy_pool() -> PgPool {
    PgPool::connect_lazy("postgresql://localhost:5432/nonexistent").expect("dummy pool")
}

/// A lazy pool for `DATABASE_URL`, or `None` when it is not configured.
fn real_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect_lazy(&url).ok()
}

/// Build nonce (db 1) and session (db 0) caches, panicking if Redis is down.
async fn setup_caches() -> (NonceCache, SessionCache) {
    let base = redis_base().trim_end_matches('/').to_string();
    let nonce_cache = NonceCache::new(&format!("{}/1", base))
        .await
        .expect("Redis required for tests");
    let session_cache = SessionCache::new(&format!("{}/0", base))
        .await
        .expect("Redis required for tests");
    (nonce_cache, session_cache)
}

/// A valid signature over the stored nonce yields a session cookie.
#[actix_web::test]
async fn test_post_auth_success() {
    let Some(pool) = real_pool() else { return };
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    let mut seeded_rng = StdRng::seed_from_u64(84);
    let wallet = LocalWallet::new(&mut seeded_rng);
    let wallet_address = format!("0x{:x}", wallet.address()).to_lowercase();
    let nonce = "test_nonce";
    let signature = wallet.sign_message(nonce).await.unwrap();

    let (nonce_cache, session_cache) = setup_caches().await;
    let nonce_cache_data = web::Data::new(nonce_cache.clone());
    let session_cache_data = web::Data::new(session_cache.clone());

    nonce_cache
        .insert(wallet_address.clone(), nonce.to_string())
        .await;

    let payload = VerifySignatureRequest {
        signature: format!("0x{}", signature),
        msg: nonce.to_string(),
    };

    let resp = AuthController::login(
        web::Json(payload),
        nonce_cache_data.clone(),
        session_cache_data.clone(),
        web::Data::new(pool.clone()),
    )
    .await;

    let req = test::TestRequest::default().to_http_request();
    let http_resp = resp.respond_to(&req);
    assert_eq!(http_resp.status(), StatusCode::OK);

    let cookie_header = http_resp
        .headers()
        .get(SET_COOKIE)
        .expect("No Set-Cookie header")
        .to_str()
        .unwrap();

    let cookie = Cookie::parse(cookie_header).unwrap();
    let token = cookie.value();

    let cached_wallet = session_cache.get(token).await;
    assert_eq!(cached_wallet.unwrap(), wallet_address);

    // Login get-or-creates the user row for the wallet.
    let user = share::db::user::User::find_by_wallet(&pool, &wallet_address)
        .await
        .expect("find_by_wallet");
    assert!(user.is_some(), "login should have created the user row");
}

/// Signing a nonce other than the stored one is rejected with `400`.
#[actix_web::test]
async fn test_post_auth_nonce_mismatch() {
    let mut seeded_rng = StdRng::seed_from_u64(85);
    let wallet = LocalWallet::new(&mut seeded_rng);
    let wallet_address = format!("0x{:x}", wallet.address()).to_lowercase();

    let (nonce_cache, session_cache) = setup_caches().await;

    nonce_cache
        .insert(wallet_address.clone(), "correct_nonce".to_string())
        .await;

    let wrong_nonce = "evil_nonce";
    let signature = wallet.sign_message(wrong_nonce).await.unwrap();

    let payload = VerifySignatureRequest {
        signature: format!("0x{}", signature),
        msg: wrong_nonce.to_string(),
    };

    let resp = AuthController::login(
        web::Json(payload),
        web::Data::new(nonce_cache),
        web::Data::new(session_cache),
        web::Data::new(dummy_pool()),
    )
    .await;

    let req = test::TestRequest::default().to_http_request();
    let http_resp = resp.respond_to(&req);

    assert_eq!(http_resp.status(), StatusCode::BAD_REQUEST);
}

/// A malformed signature is rejected with `401`.
#[actix_web::test]
async fn test_post_auth_invalid_signature() {
    let (nonce_cache, session_cache) = setup_caches().await;

    let payload = VerifySignatureRequest {
        signature: "not-a-hex-string".to_string(),
        msg: "some_nonce".to_string(),
    };

    let resp = AuthController::login(
        web::Json(payload),
        web::Data::new(nonce_cache),
        web::Data::new(session_cache),
        web::Data::new(dummy_pool()),
    )
    .await;

    let req = test::TestRequest::default().to_http_request();
    let http_resp = resp.respond_to(&req);

    assert_eq!(http_resp.status(), StatusCode::UNAUTHORIZED);
}

/// Logout invalidates the session token.
#[actix_web::test]
async fn test_logout() {
    let (_, session_cache) = setup_caches().await;

    let token = "test_logout_token";
    let wallet_address = "0x1234567890abcdef".to_string();
    session_cache
        .insert(token.to_string(), wallet_address.clone())
        .await;

    let req = test::TestRequest::default()
        .cookie(Cookie::build("session_token", token).finish())
        .to_http_request();

    let resp = AuthController::logout(req.clone(), web::Data::new(session_cache.clone())).await;

    let http_resp = resp.respond_to(&req);
    assert_eq!(http_resp.status(), StatusCode::OK);

    let cached_wallet = session_cache.get(token).await;
    assert!(cached_wallet.is_none());
}

/// A known session token verifies successfully.
#[actix_web::test]
async fn test_verify_session_ok() {
    let (_, session_cache) = setup_caches().await;

    let token = "test_verify_session_ok_token";
    let wallet_address = "0x1234567890abcdef".to_string();
    session_cache
        .insert(token.to_string(), wallet_address.clone())
        .await;

    let req = test::TestRequest::default()
        .cookie(Cookie::build("session_token", token).finish())
        .to_http_request();

    let resp =
        AuthController::verify_session(req.clone(), web::Data::new(session_cache.clone())).await;

    let http_resp = resp.respond_to(&req);
    assert_eq!(http_resp.status(), StatusCode::OK);

    let cached_wallet = session_cache.get(token).await;
    assert_eq!(cached_wallet.unwrap(), wallet_address);
}

/// An unknown session token is rejected with `401`.
#[actix_web::test]
async fn test_verify_session_invalid() {
    let (_, session_cache) = setup_caches().await;

    let req = test::TestRequest::default()
        .cookie(Cookie::build("session_token", "invalid_token").finish())
        .to_http_request();

    let resp =
        AuthController::verify_session(req.clone(), web::Data::new(session_cache.clone())).await;

    let http_resp = resp.respond_to(&req);
    assert_eq!(http_resp.status(), StatusCode::UNAUTHORIZED);

    let cached_wallet = session_cache.get("invalid_token").await;
    assert!(cached_wallet.is_none());
}
