pub mod cache;
pub mod config;
pub mod db;
pub mod erc20;
pub mod error;
pub mod logger;
pub mod models;
pub mod rpc;

mod wallet {
    tonic::include_proto!("wallet");
}

mod analyzer {
    tonic::include_proto!("analyzer");
}

pub use wallet::*;
pub use analyzer::*;
pub use config::{ProcessConfig, ServiceConfig};
pub use error::{ProcessError, ServiceError};
