use std::time::Duration;

use share::ProcessError;
use tonic::transport::{Channel, Endpoint};

/// Build a lazily connected channel with per-request and connect timeouts.
pub fn lazy_channel(endpoint: &str, timeout: Duration) -> Result<Channel, ProcessError> {
    Ok(Endpoint::from_shared(endpoint.to_string())
        .map_err(|e| ProcessError::InvalidConfig(format!("invalid engine endpoint: {e}")))?
        .timeout(timeout)
        .connect_timeout(timeout)
        .connect_lazy())
}

/// Map a gRPC status to a process error, preserving retryability.
pub fn status_error(operation: &str, status: tonic::Status) -> ProcessError {
    let retryable = matches!(
        status.code(),
        tonic::Code::Unavailable
            | tonic::Code::DeadlineExceeded
            | tonic::Code::ResourceExhausted
            | tonic::Code::Internal
            | tonic::Code::Unknown
            | tonic::Code::Cancelled
    );
    if retryable {
        ProcessError::Network(format!("{operation}: {status}"))
    } else {
        ProcessError::InvalidConfig(format!("{operation} rejected: {status}"))
    }
}
