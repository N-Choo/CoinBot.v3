#[cfg(test)]
mod tests {
    //! Route-level integration tests: CORS, routing, and status codes.
    //! Require a reachable Redis.

    use actix_cors::Cors;
    use actix_web::http::header;
    use actix_web::{App, http, middleware::NormalizePath, test, web};

    use crate::handlers::user::auth::{NonceCache, SessionCache};
    use crate::routes::api_routes;

    /// Redis base URL from `REDIS_URL`, defaulting to localhost.
    fn redis_base() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    /// Build a test app wired with the real routes and CORS policy.
    async fn create_test_app() -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
    > {
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);

        let base = redis_base().trim_end_matches('/').to_string();
        let nonce_cache = NonceCache::new(&format!("{}/1", base))
            .await
            .expect("Redis required for tests");
        let session_cache = SessionCache::new(&format!("{}/0", base))
            .await
            .expect("Redis required for tests");

        test::init_service(
            App::new()
                .wrap(cors)
                .wrap(NormalizePath::trim())
                .app_data(web::Data::new(nonce_cache))
                .app_data(web::Data::new(session_cache))
                .configure(api_routes),
        )
        .await
    }

    /// CORS preflight on `/api/` returns the allow headers.
    #[actix_web::test]
    async fn test_cors_preflight_root() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/")
            .method(http::Method::OPTIONS)
            .insert_header((header::ORIGIN, "http://127.0.0.1:5173"))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_some()
        );
        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .is_some()
        );
        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .is_some()
        );
        assert!(resp.headers().get(header::ACCESS_CONTROL_MAX_AGE).is_some());
    }

    /// CORS preflight on the auth endpoint succeeds.
    #[actix_web::test]
    async fn test_cors_preflight_auth() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/auth")
            .method(http::Method::OPTIONS)
            .insert_header((header::ORIGIN, "http://127.0.0.1:5173"))
            .insert_header(("Access-Control-Request-Method", "POST"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_some()
        );
    }

    /// The nonce endpoint accepts a `wallet_address` query parameter.
    #[actix_web::test]
    async fn test_auth_nonce_endpoint_exists() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/auth?wallet_address=0x1234567890abcdef")
            .method(http::Method::GET)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    /// A missing `wallet_address` query parameter is rejected with `400`.
    #[actix_web::test]
    async fn test_auth_nonce_missing_wallet() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/auth")
            .method(http::Method::GET)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    /// An unregistered route returns `404`.
    #[actix_web::test]
    async fn test_unknown_route_returns_404() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/nonexistent")
            .method(http::Method::GET)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    /// Logout without a session cookie returns `400`.
    #[actix_web::test]
    async fn test_logout_without_cookie_returns_400() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/logout")
            .method(http::Method::POST)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    /// An unsupported method on the auth endpoint returns `405`.
    #[actix_web::test]
    async fn test_auth_endpoint_method_not_allowed() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/auth")
            .method(http::Method::DELETE)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::METHOD_NOT_ALLOWED);
    }

    /// CORS headers are still present on an error response.
    #[actix_web::test]
    async fn test_cors_headers_on_error_response() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/nonexistent")
            .method(http::Method::GET)
            .insert_header((header::ORIGIN, "http://127.0.0.1:5173"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_some()
        );
    }

    /// The verify endpoint rejects a request with no session (`400`).
    #[actix_web::test]
    async fn test_verify_session_endpoint_exists() {
        let app = create_test_app().await;
        let req = test::TestRequest::with_uri("/api/user/verify")
            .method(http::Method::POST)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
