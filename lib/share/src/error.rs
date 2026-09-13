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

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("missing environment variable: {0}")]
    MissingEnv(String),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("{0}")]
    Internal(String),
}

impl From<sqlx::Error> for ProcessError {
    fn from(e: sqlx::Error) -> Self {
        ProcessError::Database(e.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for ProcessError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        ProcessError::Database(format!("migration error: {e}"))
    }
}
impl From<tonic::transport::Error> for ProcessError {
    fn from(e: tonic::transport::Error) -> Self {
        ProcessError::Network(format!("gRPC transport error: {e}"))
    }
}

impl From<std::net::AddrParseError> for ProcessError {
    fn from(e: std::net::AddrParseError) -> Self {
        ProcessError::InvalidConfig(format!("address parse error: {e}"))
    }
}

impl From<std::io::Error> for ProcessError {
    fn from(e: std::io::Error) -> Self {
        ProcessError::Network(format!("I/O error: {e}"))
    }
}

impl From<redis::RedisError> for ProcessError {
    fn from(e: redis::RedisError) -> Self {
        ProcessError::Internal(format!("redis error: {e}"))
    }
}

impl From<serde_json::Error> for ProcessError {
    fn from(e: serde_json::Error) -> Self {
        ProcessError::Internal(format!("serialization error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_missing_env() {
        let err = ProcessError::MissingEnv("DATABASE_URL".into());
        assert_eq!(err.to_string(), "missing environment variable: DATABASE_URL");
    }

    #[test]
    fn display_invalid_config() {
        let err = ProcessError::InvalidConfig("bad port".into());
        assert_eq!(err.to_string(), "invalid configuration: bad port");
    }

    #[test]
    fn display_database() {
        let err = ProcessError::Database("connection refused".into());
        assert_eq!(err.to_string(), "database error: connection refused");
    }

    #[test]
    fn display_network() {
        let err = ProcessError::Network("timeout".into());
        assert_eq!(err.to_string(), "network error: timeout");
    }

    #[test]
    fn display_internal() {
        let err = ProcessError::Internal("something broke".into());
        assert_eq!(err.to_string(), "something broke");
    }

    #[test]
    fn debug_output() {
        let err = ProcessError::MissingEnv("X".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("MissingEnv"));
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: ProcessError = io.into();
        assert!(matches!(err, ProcessError::Network(_)));
        assert!(err.to_string().contains("file missing"));
    }
}
