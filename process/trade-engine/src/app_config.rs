use share::{ProcessConfig, ProcessError};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub db_url: String,
    pub redis_ulr: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ProcessError> {
        let cfg = ProcessConfig::new("trade_engine");

        Ok(Self {
            db_url: cfg.get("DATABASE_URL")?,
            redis_ulr: cfg.get("REDIS_URL")?,
        })
    }
}
