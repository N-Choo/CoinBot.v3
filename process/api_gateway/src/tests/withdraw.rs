use actix_web::{App, http::StatusCode, test, web};
use ethers::signers::Signer;
use rust_decimal::Decimal;
use std::str::FromStr;

use share::db::user::User;
use share::db::withdrawals::Withdrawals;
use share::withdraw as intent;

use crate::auth::{NonceCache, SessionCache};
use crate::config::AppConfig;
use crate::kucoin_quote::{WithdrawQuote, WithdrawQuoteCache};
use crate::routes::api_routes;

pub(crate) fn config() -> AppConfig {
    AppConfig {
        ip: "127.0.0.1".into(),
        port: 8080,
        n_worker: 1,
        n_queue: 1,
        db_url: "postgres://localhost/coinbot".into(),
        session_redis_url: "redis://127.0.0.1:6379/0".into(),
        nonce_redis_url: "redis://127.0.0.1:6379/1".into(),
        grpc_deposit: "http://localhost:50051".into(),
        allowed_origin: "*".into(),
        withdraw_max_pending: 3,
        withdraw_daily_cap: 20,
        grpc_auth_token: "test-token".into(),
    }
}

async fn caches() -> (NonceCache, SessionCache, sqlx::PgPool) {
    let base = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let nonce = NonceCache::new(&format!("{base}/1"))
        .await
        .expect("Redis required for tests");
    let session = SessionCache::new(&format!("{base}/0"))
        .await
        .expect("Redis required for tests");
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/coinbot")
        .expect("lazy pool requires a valid URL");
    (nonce, session, pool)
}

macro_rules! test_app {
    ($nonce:expr, $session:expr, $pool:expr, $cache:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new($nonce))
                .app_data(web::Data::new($session))
                .app_data(web::Data::new($pool))
                .app_data(web::Data::new($cache))
                .app_data(web::Data::new(reqwest::Client::new()))
                .app_data(web::Data::new(config()))
                .configure(api_routes),
        )
        .await
    };
}

#[actix_web::test]
async fn quote_requires_a_session() {
    let (nonce, session, pool) = caches().await;
    let app = test_app!(nonce, session, pool, WithdrawQuoteCache::new());

    let request = test::TestRequest::get()
        .uri("/api/transactions/withdraw/quote")
        .to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn submit_requires_a_session() {
    let (nonce, session, pool) = caches().await;
    let app = test_app!(nonce, session, pool, WithdrawQuoteCache::new());

    let request = test::TestRequest::post()
        .uri("/api/transactions/withdraw")
        .set_json(serde_json::json!({"nonce": "n", "message": "m", "signature": "s"}))
        .to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn listing_requires_a_session() {
    let (nonce, session, pool) = caches().await;
    let app = test_app!(nonce, session, pool, WithdrawQuoteCache::new());

    let request = test::TestRequest::get()
        .uri("/api/transactions/withdrawals")
        .to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

struct Fixture {
    nonce: NonceCache,
    session: SessionCache,
    pool: sqlx::PgPool,
    cache: WithdrawQuoteCache,
    wallet: String,
    signer: ethers::signers::LocalWallet,
    token: String,
}

impl Fixture {
    async fn new(balance: Decimal) -> Option<Self> {
        use ethers::core::rand::SeedableRng;
        use ethers::core::rand::rngs::StdRng;

        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = sqlx::PgPool::connect(&url)
            .await
            .expect("DATABASE_URL is set but Postgres is unreachable");
        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .expect("database migrations failed");

        let base =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let nonce = NonceCache::new(&format!("{base}/1"))
            .await
            .expect("Redis required for tests");
        let session = SessionCache::new(&format!("{base}/0"))
            .await
            .expect("Redis required for tests");

        let mut rng = StdRng::from_entropy();
        let signer = ethers::signers::LocalWallet::new(&mut rng);
        let wallet = format!("0x{:x}", signer.address());

        let mut tx = pool.begin().await.expect("database transaction failed");
        let user = User::upsert(&mut tx, &wallet)
            .await
            .expect("user upsert failed");
        user.add_balance(&mut tx, balance)
            .await
            .expect("balance update failed");
        tx.commit().await.expect("transaction commit failed");

        let token = uuid::Uuid::new_v4().to_string();
        session.insert(token.clone(), wallet.clone()).await;

        let cache = WithdrawQuoteCache::new();
        cache
            .set_for_test(WithdrawQuote {
                chain_name: "ERC20".to_string(),
                fee: Decimal::from_str("5.5").unwrap(),
                min: Decimal::from_str("30").unwrap(),
                precision: 6,
            })
            .await;

        Some(Self {
            nonce,
            session,
            pool,
            cache,
            wallet,
            signer,
            token,
        })
    }

    fn intent(&self, nonce: &str, amount: &str, fee: &str, total: &str) -> intent::WithdrawIntent {
        let now = chrono::Utc::now().timestamp();
        intent::WithdrawIntent {
            nonce: nonce.to_string(),
            currency: "USDT".into(),
            chain: "eth".into(),
            chain_id: 1,
            destination: self.wallet.clone(),
            amount: amount.to_string(),
            fee: fee.to_string(),
            total: total.to_string(),
            issued_at: now,
            expires_at: now + 300,
        }
    }

    async fn signed(&self, payload: &intent::WithdrawIntent) -> serde_json::Value {
        use ethers::signers::Signer;

        self.nonce
            .insert_with_ttl(payload.nonce.clone(), self.wallet.clone(), 600)
            .await;
        let signature = format!(
            "0x{}",
            self.signer
                .sign_typed_data(&intent::typed_data(payload))
                .await
                .unwrap()
        );
        serde_json::json!({
            "nonce": payload.nonce,
            "message": serde_json::to_string(payload).unwrap(),
            "signature": signature,
        })
    }

    fn request(&self, body: &serde_json::Value) -> actix_http::Request {
        test::TestRequest::post()
            .uri("/api/transactions/withdraw")
            .insert_header((
                actix_web::http::header::COOKIE,
                format!("session_token={}", self.token),
            ))
            .set_json(body)
            .to_request()
    }

    async fn cleanup(&self) {
        if let Ok(Some(user)) = User::find_by_wallet(&self.pool, &self.wallet).await {
            let _ = sqlx::query("DELETE FROM ledger_entries WHERE user_uid = $1")
                .bind(user.uid)
                .execute(&self.pool)
                .await;
            let _ = sqlx::query("DELETE FROM withdrawals WHERE user_uid = $1")
                .bind(user.uid)
                .execute(&self.pool)
                .await;
            let _ = sqlx::query("DELETE FROM users WHERE uid = $1")
                .bind(user.uid)
                .execute(&self.pool)
                .await;
        }
        self.session.invalidate(&self.token).await;
    }
}

#[actix_web::test]
async fn submit_locks_funds_and_rejects_a_replayed_nonce() {
    let Some(fx) = Fixture::new(Decimal::from(100)).await else {
        return;
    };
    let app = test_app!(
        fx.nonce.clone(),
        fx.session.clone(),
        fx.pool.clone(),
        fx.cache.clone()
    );

    let payload = fx.intent(&uuid::Uuid::new_v4().to_string(), "30", "5.5", "35.5");
    let body = fx.signed(&payload).await;

    let response = test::call_service(&app, fx.request(&body)).await;
    assert_eq!(response.status(), StatusCode::OK);

    let user = User::find_by_wallet(&fx.pool, &fx.wallet)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.balance, Decimal::from_str("64.5").unwrap());
    assert_eq!(user.locked_balance, Decimal::from_str("35.5").unwrap());
    assert_eq!(
        Withdrawals::list_for_user(&fx.pool, user.uid)
            .await
            .unwrap()
            .len(),
        1
    );

    let replay = test::call_service(&app, fx.request(&body)).await;
    assert_eq!(replay.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_slice(&test::read_body(replay).await).unwrap();
    assert_eq!(body["code"], "NONCE_CONFLICT");

    fx.cleanup().await;
}

#[actix_web::test]
async fn submit_rejects_insufficient_funds() {
    let Some(fx) = Fixture::new(Decimal::from(1)).await else {
        return;
    };
    let app = test_app!(
        fx.nonce.clone(),
        fx.session.clone(),
        fx.pool.clone(),
        fx.cache.clone()
    );

    let payload = fx.intent(&uuid::Uuid::new_v4().to_string(), "30", "5.5", "35.5");
    let body = fx.signed(&payload).await;
    let response = test::call_service(&app, fx.request(&body)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    fx.cleanup().await;
}

#[actix_web::test]
async fn submit_rejects_fee_drift() {
    let Some(fx) = Fixture::new(Decimal::from(100)).await else {
        return;
    };
    let app = test_app!(
        fx.nonce.clone(),
        fx.session.clone(),
        fx.pool.clone(),
        fx.cache.clone()
    );

    let payload = fx.intent(&uuid::Uuid::new_v4().to_string(), "30", "4.5", "34.5");
    let body = fx.signed(&payload).await;
    let response = test::call_service(&app, fx.request(&body)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_slice(&test::read_body(response).await).unwrap();
    assert_eq!(body["code"], "FEE_CHANGED");

    fx.cleanup().await;
}

#[actix_web::test]
async fn submit_rejects_a_bad_signature() {
    let Some(fx) = Fixture::new(Decimal::from(100)).await else {
        return;
    };
    let app = test_app!(
        fx.nonce.clone(),
        fx.session.clone(),
        fx.pool.clone(),
        fx.cache.clone()
    );

    let payload = fx.intent(&uuid::Uuid::new_v4().to_string(), "30", "5.5", "35.5");
    let mut body = fx.signed(&payload).await;
    body["signature"] = serde_json::Value::String(format!("0x{}", "11".repeat(65)));

    let response = test::call_service(&app, fx.request(&body)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    fx.cleanup().await;
}
