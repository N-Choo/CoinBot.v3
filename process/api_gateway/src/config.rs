use actix_cors::Cors;
use actix_web::http;
use share::{ProcessConfig, ProcessError};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub ip: String,
    pub port: u16,
    pub n_worker: usize,
    pub n_queue: u32,
    pub db_url: String,
    pub session_redis_url: String,
    pub nonce_redis_url: String,
    pub grpc_deposit: String,
    pub allowed_origin: String,
}

impl AppConfig {
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
