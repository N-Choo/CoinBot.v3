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
    /// Comma-separated origins permitted by CORS.
    pub allowed_origin: String,
    /// Maximum non-terminal withdrawals a user may have queued.
    pub withdraw_max_pending: i64,
    /// Maximum withdrawals a user may create in 24 hours.
    pub withdraw_daily_cap: i64,
    /// Shared token required on internal gRPC calls.
    pub grpc_auth_token: String,
}

/// Split, trim, and drop empty entries from `ALLOWED_ORIGIN`.
fn allowed_origins(raw: &str) -> Vec<&str> {
    raw.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .collect()
}

/// Validate every configured CORS origin.
fn validate_allowed_origins(raw: &str) -> Result<(), ProcessError> {
    let origins = allowed_origins(raw);
    if origins.is_empty() {
        return Err(ProcessError::InvalidConfig(
            "ALLOWED_ORIGIN must contain at least one origin".into(),
        ));
    }

    for origin in origins {
        if origin == "*" {
            return Err(ProcessError::InvalidConfig(
                "ALLOWED_ORIGIN must list explicit origins; wildcards are not allowed".into(),
            ));
        }
        if !origin.starts_with("http://") && !origin.starts_with("https://") {
            return Err(ProcessError::InvalidConfig(format!(
                "ALLOWED_ORIGIN entries must start with http:// or https:// (got {origin})"
            )));
        }
        if origin.ends_with('/') {
            return Err(ProcessError::InvalidConfig(format!(
                "ALLOWED_ORIGIN entries must not end with '/' (got {origin})"
            )));
        }
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
        validate_allowed_origins(&allowed_origin)?;

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
    /// `ALLOWED_ORIGIN` is a comma-separated list of exact origins. Each entry
    /// must include a scheme and must not have a trailing slash. `GET`/`POST`/
    /// `OPTIONS`, credentials, and the authorization/accept/content-type headers
    /// are allowed. Wildcards are rejected at startup.
    pub fn get_cors(&self) -> Cors {
        let origins: Vec<String> = allowed_origins(&self.allowed_origin)
            .into_iter()
            .map(str::to_string)
            .collect();

        Cors::default()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600)
            .allowed_origin_fn(move |origin, _| {
                origins
                    .iter()
                    .any(|allowed| origin.as_bytes() == allowed.as_bytes())
            })
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
    async fn validates_comma_separated_origins() {
        assert!(validate_allowed_origins("*").is_err());
        assert!(validate_allowed_origins("").is_err());
        assert!(validate_allowed_origins("localhost:5173").is_err());
        assert!(validate_allowed_origins("https://coinbot.no/").is_err());
        assert!(validate_allowed_origins("http://localhost:5173").is_ok());
        assert!(
            validate_allowed_origins(
                "http://localhost:5173, https://coinbot.no,https://www.coinbot.no"
            )
            .is_ok()
        );
    }

    /// A comma-separated list allows each listed origin and rejects others.
    #[actix_web::test]
    async fn multiple_origins_are_allowed() {
        let app = test::init_service(
            App::new()
                .wrap(config_with_origin("http://example.com,https://coinbot.no").get_cors())
                .route("/", web::get().to(|| async { "ok" })),
        )
        .await;

        let allowed = test::call_service(&app, preflight("https://coinbot.no")).await;
        assert_eq!(allowed.status(), http::StatusCode::OK);
        assert!(
            allowed
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_some()
        );

        let denied = test::call_service(&app, preflight("https://evil.com")).await;
        assert!(
            denied
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none()
        );
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
