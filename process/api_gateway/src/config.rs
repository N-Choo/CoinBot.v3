use actix_cors::Cors;
use actix_web::http;
use share::{ProcessConfig, ProcessError};

/// Runtime configuration for the API gateway, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Bind address for the HTTP server.
    pub ip: String,
    /// TCP port for the HTTP server.
    pub port: u16,
    /// Number of Actix HTTP worker threads.
    pub n_worker: usize,
    /// TCP backlog size for pending connections.
    pub n_queue: u32,
    /// PostgreSQL connection string.
    pub db_url: String,
    /// Redis URL (database 0) backing session tokens.
    pub session_redis_url: String,
    /// Redis URL (database 1) backing login/signing nonces.
    pub nonce_redis_url: String,
    /// gRPC endpoint of the deposit worker.
    pub grpc_deposit: String,
    /// Origin permitted by CORS.
    pub allowed_origin: String,
    /// Maximum non-terminal withdrawals a user may have queued.
    pub withdraw_max_pending: i64,
    /// Maximum withdrawals a user may create in 24 hours.
    pub withdraw_daily_cap: i64,
    /// Shared token required on internal gRPC calls.
    pub grpc_auth_token: String,
}

/// Reject wildcard CORS origins; credentials make `*` unsafe.
fn validate_allowed_origin(origin: &str) -> Result<(), ProcessError> {
    if origin.trim() == "*" {
        return Err(ProcessError::InvalidConfig(
            "ALLOWED_ORIGIN must be an explicit origin; wildcards are not allowed".into(),
        ));
    }
    Ok(())
}

impl AppConfig {
    /// Build the configuration from environment variables.
    ///
    /// Reads `api_gateway`-prefixed values (falling back to unprefixed ones)
    /// through [`ProcessConfig`], deriving the session and nonce Redis URLs
    /// from `REDIS_URL` (databases 0 and 1 respectively).
    ///
    /// Returns [`ProcessError`] if a required variable is missing or cannot be
    /// parsed.
    pub fn from_env() -> Result<Self, ProcessError> {
        let cfg = ProcessConfig::new("api_gateway");

        let redis_base = cfg.get("REDIS_URL")?.trim_end_matches('/').to_owned();
        let allowed_origin = cfg.get("ALLOWED_ORIGIN")?;
        validate_allowed_origin(&allowed_origin)?;

        Ok(Self {
            ip: cfg.get("HOST")?,
            port: cfg.get_parsed("PORT")?,
            n_worker: cfg.get_parsed("N_WORKER")?,
            n_queue: cfg.get_parsed("N_QUEUE")?,
            db_url: cfg.get("DATABASE_URL")?,
            session_redis_url: format!("{redis_base}/0"),
            nonce_redis_url: format!("{redis_base}/1"),
            grpc_deposit: cfg.get("GRPC_DEPOSIT_ENDPOINT")?,
            allowed_origin,
            withdraw_max_pending: cfg.get_or("WITHDRAW_MAX_PENDING", 3i64),
            withdraw_daily_cap: cfg.get_or("WITHDRAW_DAILY_CAP", 20i64),
            grpc_auth_token: cfg.get("GRPC_AUTH_TOKEN")?,
        })
    }

    /// Build the CORS policy applied to every response.
    ///
    /// Allows exactly the configured origin, `GET`/`POST`/`OPTIONS`,
    /// credentials, and the authorization/accept/content-type headers.
    /// Wildcards are rejected at startup because credentials are supported.
    pub fn get_cors(&self) -> Cors {
        Cors::default()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600)
            .allowed_origin(&self.allowed_origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header;
    use actix_web::{App, http, test, web};

    fn config_with_origin(origin: &str) -> AppConfig {
        AppConfig {
            ip: "0.0.0.0".to_string(),
            port: 8080,
            n_worker: 1,
            n_queue: 1,
            db_url: String::new(),
            session_redis_url: String::new(),
            nonce_redis_url: String::new(),
            grpc_deposit: String::new(),
            allowed_origin: origin.to_string(),
            withdraw_max_pending: 3,
            withdraw_daily_cap: 20,
            grpc_auth_token: "test-token".into(),
        }
    }

    fn preflight(origin: &str) -> actix_http::Request {
        test::TestRequest::with_uri("/")
            .method(http::Method::OPTIONS)
            .insert_header((header::ORIGIN, origin))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .to_request()
    }

    #[actix_web::test]
    async fn rejects_a_wildcard_origin() {
        assert!(validate_allowed_origin("*").is_err());
        assert!(validate_allowed_origin("http://localhost:5173").is_ok());
    }

    /// A non-wildcard origin only allows that exact origin.
    #[actix_web::test]
    async fn specific_origin_restricts() {
        let app = test::init_service(
            App::new()
                .wrap(config_with_origin("http://example.com").get_cors())
                .route("/", web::get().to(|| async { "ok" })),
        )
        .await;

        let allowed = test::call_service(&app, preflight("http://example.com")).await;
        assert_eq!(allowed.status(), http::StatusCode::OK);

        let denied = test::call_service(&app, preflight("http://evil.com")).await;
        assert!(
            denied
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none()
        );
    }
}
