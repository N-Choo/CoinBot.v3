mod proto {
    tonic::include_proto!("wallet");
}

pub use proto::*;

pub mod db;
pub mod rpc;

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
