use kucoin::client::rest::{Credentials, KuCoinClient};
use share::ProcessError;
use sqlx::PgPool;

use crate::app_config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub kucoin: KuCoinClient,
}

impl AppState {
    pub async fn new(cfg: &AppConfig) -> Result<Self, ProcessError> {
        let db_pool = PgPool::connect(&cfg.db_url).await?;
        let kucoin = KuCoinClient::new(Credentials::new(
            &cfg.kc_key,
            &cfg.kc_secret,
            &cfg.kc_passphrase,
        ));

        Ok(Self {
            db_pool,
            kucoin,
        })
    }
}
