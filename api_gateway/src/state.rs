use std::time::Duration;

use anyhow::{Context, Result};
use kucoin::client::rest::{Credentials, KuCoinClient};
use moka::future::Cache;
use sqlx::PgPool;
use tonic::transport::Channel;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub nonce_cache: Cache<String, String>,
    pub session_cache: Cache<String, String>,
    pub kc_client: KuCoinClient,
    pub grpc_deposit: Channel,
}

impl AppState {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let db_pool = PgPool::connect(&config.db_url)
            .await
            .context("Failed to connect to the Database")?;

        sqlx::migrate!("../migrations")
            .run(&db_pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run database migrations: {e}"))?;

        let nonce_cache = Cache::builder()
            .max_capacity(250)
            .time_to_live(Duration::from_secs(60 * 5))
            .build();

        let session_cache = Cache::builder()
            .max_capacity(250)
            .time_to_live(Duration::from_secs(60 * 60))
            .build();

        let master_key =
            Credentials::new(&config.api_key, &config.api_secret, &config.api_passphrase);

        let kc_client = KuCoinClient::new(master_key);

        let grpc_deposit = Channel::from_shared(config.grpc_deposit.clone())
            .context("Invalid gRPC endpoint")?
            .connect_lazy();

        Ok(Self {
            db_pool,
            nonce_cache,
            session_cache,
            kc_client,
            grpc_deposit,
        })
    }
}
