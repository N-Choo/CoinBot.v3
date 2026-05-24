mod proto {
    tonic::include_proto!("wallet");
}

// Re-export everything from the generated proto module at the crate root
pub use proto::*;

// Shared error type for all services
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

// Shared config (each service loads from env)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig {
    pub grpc_host: String,
    pub grpc_port: u16,
    pub database_url: String,
    pub redis_url: String,
}

impl ServiceConfig {
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }
}
