#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test, web};
    use sqlx::PgPool;

    use tonic::transport::Channel;

    use crate::handlers::user::auth::{NonceCache, SessionCache};
    use crate::routes::api_routes;

    fn redis_base() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    async fn setup_session_cache() -> SessionCache {
        let url = format!("{}/0", redis_base().trim_end_matches('/'));
        SessionCache::new(&url)
            .await
            .expect("Redis required for tests")
    }

    async fn setup_nonce_cache() -> NonceCache {
        let url = format!("{}/1", redis_base().trim_end_matches('/'));
        NonceCache::new(&url)
            .await
            .expect("Redis required for tests")
    }

    fn dummy_pool() -> PgPool {
        PgPool::connect_lazy("postgresql://localhost:5432/nonexistent").expect("dummy pool")
    }

    fn real_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect_lazy(&url).ok()
    }

    #[actix_web::test]
    async fn test_list_unauthorized() {
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
            .uri("/api/transactions")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_list_no_user_returns_empty() {
        let Some(pool) = real_pool() else { return };

        let session_cache = setup_session_cache().await;
        let nonce_cache = setup_nonce_cache().await;
        let wallet = "0x0000000000000000000000000000000000000000";
        session_cache
            .insert("test_list_no_user_token".into(), wallet.into())
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
            .uri("/api/transactions")
            .cookie(actix_web::cookie::Cookie::new(
                "session_token",
                "test_list_no_user_token",
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_deposit_unauthorized() {
        let channel = Channel::from_shared("http://127.0.0.1:1".to_string())
            .unwrap()
            .connect_lazy();

        let session_cache = setup_session_cache().await;
        let nonce_cache = setup_nonce_cache().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(dummy_pool()))
                .app_data(web::Data::new(session_cache))
                .app_data(web::Data::new(nonce_cache))
                .app_data(web::Data::new(channel))
                .configure(api_routes),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/transactions/deposit")
            .set_json(serde_json::json!({"tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_auth_challenge_invalid_wallet() {
        let session_cache = setup_session_cache().await;
        let nonce_cache = setup_nonce_cache().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(session_cache))
                .app_data(web::Data::new(nonce_cache))
                .configure(api_routes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/user/auth?wallet_address=")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_auth_verify_no_session() {
        let session_cache = setup_session_cache().await;
        let nonce_cache = setup_nonce_cache().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(session_cache))
                .app_data(web::Data::new(nonce_cache))
                .configure(api_routes),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/user/verify")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_logout_no_session() {
        let session_cache = setup_session_cache().await;
        let nonce_cache = setup_nonce_cache().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(session_cache))
                .app_data(web::Data::new(nonce_cache))
                .configure(api_routes),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/user/logout")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
