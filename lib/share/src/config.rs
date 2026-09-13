use serde::Deserialize;

use crate::error::ProcessError;

#[derive(Debug, Clone, Deserialize)]
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

/// Loads environment variables scoped to a process name prefix.
///
/// Checks `PREFIX_KEY` first, falls back to `KEY` so shared vars like
/// `DATABASE_URL` work without per-process duplication while per-process
/// overrides (e.g. `API_GATEWAY_DATABASE_URL`) are supported.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    prefix: String,
}

impl ProcessConfig {
    pub fn new(name: &str) -> Self {
        Self {
            prefix: name.to_uppercase(),
        }
    }

    /// Get a raw env var. Tries `PREFIX_KEY` then `KEY`.
    pub fn get(&self, key: &str) -> Result<String, ProcessError> {
        let prefixed = format!("{}_{}", self.prefix, key);

        match std::env::var(&prefixed) {
            Ok(val) => Ok(val),
            Err(_) => std::env::var(key).map_err(|_| {
                ProcessError::MissingEnv(format!("neither {} nor {} is set", prefixed, key))
            }),
        }
    }

    /// Get a parsed value with a default fallback.
    pub fn get_or<T: std::str::FromStr>(&self, key: &str, default: T) -> T {
        self.get(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    /// Get a raw env var with a default string fallback.
    pub fn get_or_str(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get and parse a typed value, failing with a descriptive error.
    pub fn get_parsed<T: std::str::FromStr>(&self, key: &str) -> Result<T, ProcessError> {
        let raw = self.get(key)?;
        raw.parse().map_err(|_| {
            ProcessError::InvalidConfig(format!(
                "{} must be a valid {}",
                key,
                std::any::type_name::<T>()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn prefix_is_uppercased() {
        let _lock = ENV_LOCK.lock();
        std::env::set_var("MY_APP_DB_URL", "postgres://ok");
        let cfg = ProcessConfig::new("my_app");
        assert_eq!(cfg.get("DB_URL").unwrap(), "postgres://ok");
        std::env::remove_var("MY_APP_DB_URL");
    }

    #[test]
    fn get_returns_missing_env_when_not_set() {
        let _lock = ENV_LOCK.lock();
        let cfg = ProcessConfig::new("test");
        let result = cfg.get("DOES_NOT_EXIST_XYZ");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProcessError::MissingEnv(_)));
        assert!(err.to_string().contains("TEST_DOES_NOT_EXIST_XYZ"));
    }

    #[test]
    fn get_reads_bare_key_when_prefixed_not_set() {
        let _lock = ENV_LOCK.lock();
        std::env::set_var("DATABASE_URL", "postgres://bare");
        let cfg = ProcessConfig::new("test");
        let result = cfg.get("DATABASE_URL");
        assert_eq!(result.unwrap(), "postgres://bare");
        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn get_prefers_prefixed_key_over_bare() {
        let _lock = ENV_LOCK.lock();
        std::env::set_var("DATABASE_URL", "postgres://bare");
        std::env::set_var("TEST_DATABASE_URL", "postgres://prefixed");
        let cfg = ProcessConfig::new("test");
        let result = cfg.get("DATABASE_URL");
        assert_eq!(result.unwrap(), "postgres://prefixed");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("TEST_DATABASE_URL");
    }

    #[test]
    fn get_or_returns_default_when_not_set() {
        let cfg = ProcessConfig::new("test");
        let val: u32 = cfg.get_or("PORT", 8080u32);
        assert_eq!(val, 8080);
    }

    #[test]
    fn get_or_returns_parsed_value_when_set() {
        let _lock = ENV_LOCK.lock();
        std::env::set_var("TEST_PORT", "3000");
        let cfg = ProcessConfig::new("test");
        let val: u16 = cfg.get_or("PORT", 8080u16);
        assert_eq!(val, 3000);
        std::env::remove_var("TEST_PORT");
    }

    #[test]
    fn get_or_str_returns_default_when_not_set() {
        let cfg = ProcessConfig::new("test");
        let val = cfg.get_or_str("HOST", "127.0.0.1");
        assert_eq!(val, "127.0.0.1");
    }

    #[test]
    fn get_or_str_returns_value_when_set() {
        let _lock = ENV_LOCK.lock();
        std::env::set_var("TEST_HOST", "0.0.0.0");
        let cfg = ProcessConfig::new("test");
        let val = cfg.get_or_str("HOST", "127.0.0.1");
        assert_eq!(val, "0.0.0.0");
        std::env::remove_var("TEST_HOST");
    }

    #[test]
    fn service_config_grpc_addr() {
        let cfg = ServiceConfig {
            grpc_host: "localhost".into(),
            grpc_port: 50051,
            database_url: "postgres://db".into(),
            redis_url: "redis://cache".into(),
        };
        assert_eq!(cfg.grpc_addr(), "localhost:50051");
    }
}
