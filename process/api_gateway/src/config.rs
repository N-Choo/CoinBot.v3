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

        Ok(Self {
            ip: cfg.get("HOST")?,
            port: cfg.get_parsed("PORT")?,
            n_worker: cfg.get_parsed("N_WORKER")?,
            n_queue: cfg.get_parsed("N_QUEUE")?,
            db_url: cfg.get("DATABASE_URL")?,
            session_redis_url: format!("{redis_base}/0"),
            nonce_redis_url: format!("{redis_base}/1"),
            grpc_deposit: cfg.get("GRPC_DEPOSIT_ENDPOINT")?,
            allowed_origin: cfg.get("ALLOWED_ORIGIN")?,
        })
    }

    /// Build the CORS policy applied to every response.
    ///
    /// Allows the configured origin, `GET`/`POST`/`OPTIONS`, credentials, and
    /// the authorization/accept/content-type headers.
    pub fn get_cors(&self) -> Cors {
        Cors::default()
            .allowed_origin(&self.allowed_origin)
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600)
    }
}
