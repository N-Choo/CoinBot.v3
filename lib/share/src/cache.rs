use futures::StreamExt;
use redis::{AsyncCommands, Client};

/// Redis-backed key/value store and pub/sub client.
///
/// Cloning shares the same multiplexed connection.
#[derive(Clone)]
pub struct Cache {
    conn: redis::aio::MultiplexedConnection,
    url: String,
}

impl Cache {
    /// Connect to Redis at `redis_url`.
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            conn,
            url: redis_url.to_string(),
        })
    }

    /// Fetch the string value at `key`, or `None` if absent or on error.
    pub async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.clone();
        conn.get(key).await.ok()
    }

    /// Set `key` to `value` with a TTL in seconds. Failures are logged.
    pub async fn set(&self, key: &str, value: &str, ttl_secs: u64) {
        let mut conn = self.conn.clone();
        if let Err(e) = conn.set_ex::<&str, &str, ()>(key, value, ttl_secs).await {
            log::error!("Redis set failed: {e}");
        }
    }

    /// Delete `key`. Failures are logged.
    pub async fn del(&self, key: &str) {
        let mut conn = self.conn.clone();
        if let Err(e) = conn.del::<&str, ()>(key).await {
            log::error!("Redis del failed: {e}");
        }
    }

    /// Publish `msg` to `channel`. Failures are logged.
    pub async fn publish(&self, channel: &str, msg: &str) {
        let mut conn = self.conn.clone();
        if let Err(e) = conn.publish::<&str, &str, ()>(channel, msg).await {
            log::error!("Redis publish failed: {e}");
        }
    }

    /// Subscribe to `channel` and invoke `handler` for every payload.
    ///
    /// Runs until the subscription ends or the connection fails.
    pub async fn subscribe<F, Fut>(
        &self,
        channel: &str,
        mut handler: F,
    ) -> Result<(), redis::RedisError>
    where
        F: FnMut(String) -> Fut + Send,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let client = Client::open(self.url.clone())?;
        let mut pubsub = client.get_async_pubsub().await?;
        pubsub.subscribe(channel).await?;

        let mut stream = pubsub.on_message();
        while let Some(msg) = stream.next().await {
            let payload: String = msg.get_payload()?;
            handler(payload).await;
        }
        Ok(())
    }
}
