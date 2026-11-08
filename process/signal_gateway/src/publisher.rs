use std::time::Duration;

use async_trait::async_trait;
use share::engine_service_client::EngineServiceClient;
use share::{Ack, ProcessError, Signal as ProtoSignal};
use tonic::transport::{Channel, Endpoint};

#[async_trait]
pub trait SignalPublisher: Send + Sync {
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError>;
}

pub struct GrpcPublisher {
    client: EngineServiceClient<Channel>,
    timeout: Duration,
}

impl GrpcPublisher {
    /// Lazy channel: startup does not require `trade-engine` to be reachable.
    pub fn new(endpoint: &str, timeout: Duration) -> Result<Self, ProcessError> {
        let channel = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| {
                ProcessError::InvalidConfig(format!("invalid engine endpoint {endpoint:?}: {e}"))
            })?
            .timeout(timeout)
            .connect_timeout(timeout)
            .connect_lazy();
        Ok(Self {
            client: EngineServiceClient::new(channel),
            timeout,
        })
    }
}

#[async_trait]
impl SignalPublisher for GrpcPublisher {
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError> {
        let mut client = self.client.clone();
        let ack = tokio::time::timeout(self.timeout, client.publish(signal))
            .await
            .map_err(|_| ProcessError::Network("Publish timed out".into()))?
            .map_err(|e| ProcessError::Network(format!("Publish: {e}")))?
            .into_inner();
        Ok(ack)
    }
}
