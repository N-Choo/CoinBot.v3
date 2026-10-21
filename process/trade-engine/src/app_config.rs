use share::{ProcessConfig, ProcessError};

#[derive(Clone)]
pub struct AppConfig {
    pub db_url: String,
    //pub redis_ulr: String,
    pub kc_key: String,
    pub kc_secret: String,
    pub kc_passphrase: String,
    /// Bind address for the engine's `EngineService` gRPC server (the analyzer is the client).
    pub grpc_addr: String,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("db_url", &"<redacted>")
            //.field("redis_ulr", &"<redacted>")
            .field("kc_key", &"<redacted>")
            .field("kc_secret", &"<redacted>")
            .field("kc_passphrase", &"<redacted>")
            .finish()
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ProcessError> {
        let cfg = ProcessConfig::new("trade_engine");

        Ok(Self {
            db_url: cfg.get("DATABASE_URL")?,
            //redis_ulr: cfg.get("REDIS_URL")?,
            kc_key: cfg.get("KC_KEY")?,
            kc_secret: cfg.get("KC_SECRET")?,
            kc_passphrase: cfg.get("KC_PASSPHRASE")?,
            grpc_addr: cfg.get("GRPC_ADDR")?,
        })
    }
}
