//! Publishes signals to the trade engine.

use std::time::Duration;

use async_trait::async_trait;
use share::engine_service_client::EngineServiceClient;
use share::{Ack, ProcessError, Signal as ProtoSignal};
use tonic::transport::Channel;

use crate::grpc::{lazy_channel, status_error};

/// Publishes signals to the trade engine.
#[async_trait]
pub trait SignalPublisher: Send + Sync {
    /// Publish one signal to the trade engine.
    ///
    /// An `Ack { accepted: false, reason: "duplicate" }` means the engine has
    /// already accepted this `signal_id`; treat it as success.
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError>;
}

/// gRPC-backed [`SignalPublisher`].
pub struct GrpcPublisher {
    client: EngineServiceClient<Channel>,
    timeout: Duration,
    token: String,
}

impl GrpcPublisher {
    /// Lazy channel: startup does not require `trade-engine` to be reachable.
    pub fn new(endpoint: &str, timeout: Duration, token: String) -> Result<Self, ProcessError> {
        Ok(Self {
            client: EngineServiceClient::new(lazy_channel(endpoint, timeout)?),
            timeout,
            token,
        })
    }
}

#[async_trait]
impl SignalPublisher for GrpcPublisher {
    /// Publish with a timeout and map gRPC errors.
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError> {
        let mut client = self.client.clone();
        let request = share::grpc_auth::with_token(signal, &self.token)
            .map_err(|status| ProcessError::Internal(status.to_string()))?;
        let ack = tokio::time::timeout(self.timeout, client.publish(request))
            .await
            .map_err(|_| ProcessError::Network("Publish timed out".into()))?
            .map_err(|status| status_error("Publish", status))?
            .into_inner();
        Ok(ack)
    }
}
