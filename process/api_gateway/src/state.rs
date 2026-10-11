use share::ProcessError;
use sqlx::{PgPool, migrate};
use tonic::transport::Channel;

use crate::config::AppConfig;
use crate::handlers::user::auth::{NonceCache, SessionCache};

/// Shared, cloneable application state handed to handlers via `web::Data`.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub db_pool: PgPool,
    /// Redis-backed cache of login nonces.
    pub nonce_cache: NonceCache,
    /// Redis-backed cache of session tokens.
    pub session_cache: SessionCache,
    /// Lazy gRPC channel to the deposit worker.
    pub grpc_deposit: Channel,
}

impl AppState {
    /// Connect to PostgreSQL (running pending migrations), the Redis caches, and
    /// the deposit-worker gRPC endpoint.
    ///
    /// Returns [`ProcessError`] if the database/cache cannot be reached or the
    /// gRPC endpoint is malformed.
    pub async fn new(config: &AppConfig) -> Result<Self, ProcessError> {
        let db_pool = PgPool::connect(&config.db_url).await?;
        migrate!("../migrations").run(&db_pool).await?;

        let nonce_cache = NonceCache::new(&config.nonce_redis_url).await?;
        let session_cache = SessionCache::new(&config.session_redis_url).await?;

        let grpc_deposit = Channel::from_shared(config.grpc_deposit.clone())
            .map_err(|e| ProcessError::InvalidConfig(format!("Invalid gRPC endpoint: {e}")))?
            .connect_lazy();

        Ok(Self {
            db_pool,
            nonce_cache,
            session_cache,
            grpc_deposit,
        })
    }
}
