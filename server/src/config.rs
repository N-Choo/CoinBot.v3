use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub ip: String,
    pub port: u16,
    pub n_worker: usize,
    pub n_queue: u32,
    pub db_url: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            ip: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("TS_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("TS_PORT must be a valid u16 number")?,
            n_worker: env::var("N_WORKER")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
            n_queue: 100,
            db_url: env::var("DATABASE_URL").context("Missing: DATABASE_URL")?,
            api_key: env::var("API_KEY").context("Missing: API_KEY")?,
            api_secret: env::var("API_SECRET").context("Missing: API_SECRET")?,
            api_passphrase: env::var("API_PASSPHRASE").context("Missing: API_PASSPHRASE")?,
        })
    }
}
