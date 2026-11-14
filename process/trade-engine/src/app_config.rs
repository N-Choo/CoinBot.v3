use share::{ProcessConfig, ProcessError};

#[derive(Clone)]
pub struct AppConfig {
    pub db_url: String,
    //pub redis_ulr: String,
    pub kc_key: String,
    pub kc_secret: String,
    pub kc_passphrase: String,
    /// Bind address for the engine's `EngineService` gRPC server (signal-gateway is the client).
    pub grpc_addr: String,
    /// Shared token required on internal gRPC calls.
    pub grpc_auth_token: String,
    /// Cross-margin leverage applied to every contract (`1` = no leverage).
    pub leverage: u32,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("db_url", &"<redacted>")
            //.field("redis_ulr", &"<redacted>")
            .field("kc_key", &"<redacted>")
            .field("kc_secret", &"<redacted>")
            .field("kc_passphrase", &"<redacted>")
            .field("leverage", &self.leverage)
            .finish()
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ProcessError> {
        let cfg = ProcessConfig::new("trade_engine");
        // Default to 1 when unset, but reject a malformed value instead of
        // silently trading unleveraged.
        let leverage = match cfg.get("LEVERAGE_LEVEL") {
            Ok(raw) => raw.parse::<u32>().map_err(|_| {
                ProcessError::InvalidConfig("LEVERAGE_LEVEL must be an integer".into())
            })?,
            Err(ProcessError::MissingEnv(_)) => 1,
            Err(e) => return Err(e),
        };
        if !(1..=crate::kucoin_margin::MAX_LEVERAGE).contains(&leverage) {
            return Err(ProcessError::InvalidConfig(format!(
                "LEVERAGE_LEVEL must be between 1 and {}",
                crate::kucoin_margin::MAX_LEVERAGE
            )));
        }

        Ok(Self {
            db_url: cfg.get("DATABASE_URL")?,
            //redis_ulr: cfg.get("REDIS_URL")?,
            kc_key: cfg.get("KC_KEY")?,
            kc_secret: cfg.get("KC_SECRET")?,
            kc_passphrase: cfg.get("KC_PASSPHRASE")?,
            grpc_addr: cfg.get("GRPC_ADDR")?,
            grpc_auth_token: cfg.get("GRPC_AUTH_TOKEN")?,
            leverage,
        })
    }
}
