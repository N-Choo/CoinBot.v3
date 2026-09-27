use share::cache::Cache;

/// Redis-backed store of session tokens.
///
/// Maps an opaque session token to the wallet address that owns it.
#[derive(Clone)]
pub struct SessionCache {
    cache: Cache,
}

/// Redis-backed store of one-time signing nonces.
///
/// Maps a nonce to the wallet it was issued for.
#[derive(Clone)]
pub struct NonceCache {
    cache: Cache,
}

impl SessionCache {
    /// Session lifetime, in seconds (1 hour).
    const TTL: u64 = 3600;

    /// Connect to the Redis instance at `redis_url`.
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            cache: Cache::new(redis_url).await?,
        })
    }
    /// Look up the wallet bound to a session token.
    pub async fn get(&self, k: &str) -> Option<String> {
        self.cache.get(k).await
    }
    /// Store `v` under key `k` with the session TTL.
    pub async fn insert(&self, k: String, v: String) {
        self.cache.set(&k, &v, Self::TTL).await
    }
    /// Remove a session token.
    pub async fn invalidate(&self, k: &str) {
        self.cache.del(k).await
    }
}

impl NonceCache {
    /// Nonce lifetime, in seconds (5 minutes).
    const TTL: u64 = 300;

    /// Connect to the Redis instance at `redis_url`.
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            cache: Cache::new(redis_url).await?,
        })
    }
    /// Look up the wallet bound to a nonce.
    pub async fn get(&self, k: &str) -> Option<String> {
        self.cache.get(k).await
    }
    /// Store `v` under key `k` with the nonce TTL.
    pub async fn insert(&self, k: String, v: String) {
        self.cache.set(&k, &v, Self::TTL).await
    }
    /// Remove a nonce (single-use).
    pub async fn invalidate(&self, k: &str) {
        self.cache.del(k).await
    }
}
