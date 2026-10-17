use sha2::{Digest, Sha256};
use std::path::Path;

/// lock = hash(config_version + backtest params)
pub fn lock_hash(config_version: &str, backtest_params_json: &str) -> String {
    let mut h = Sha256::new();
    h.update(config_version.as_bytes());
    h.update(b"|");
    h.update(backtest_params_json.as_bytes());
    format!("{:x}", h.finalize())
}

/// Refuse to start OOS without a matching lock.
pub fn check_lock(lock_path: &str, expected: &str) -> Result<(), String> {
    match std::fs::read_to_string(lock_path) {
        Ok(v) if v.trim() == expected => Ok(()),
        Ok(v) => Err(format!("lock mismatch: expected {expected}, found {}", v.trim())),
        Err(_) => Err(format!("no lock file at {lock_path}; in-sample exploration only")),
    }
}

pub fn write_lock(lock_path: &str, value: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(lock_path).parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(lock_path, value)
}

/// True if an OOS run on this data has already been consumed (one-shot guard).
pub fn oos_already_consumed(consumed_path: &str) -> bool {
    Path::new(consumed_path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_depends_on_both_inputs() {
        assert_ne!(lock_hash("cfg1", "{\"fee\":5}"), lock_hash("cfg1", "{\"fee\":6}"));
        assert_ne!(lock_hash("cfg1", "{\"fee\":5}"), lock_hash("cfg2", "{\"fee\":5}"));
    }

    #[test]
    fn missing_lock_is_error() {
        assert!(check_lock("/nonexistent/lock", "x").is_err());
    }
}
