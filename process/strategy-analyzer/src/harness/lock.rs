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

/// Outcome of the one-shot OOS guard for a consumed-marker path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OosDecision { Allow, RerunNonConfirmatory, Refuse }

/// One-shot OOS gate keyed on the consumed marker (`<lock>.consumed`):
/// - no marker              -> `Allow`: the first, confirmatory OOS run.
/// - marker, no override    -> `Refuse`: OOS has already been consumed.
/// - marker, `override_flag`-> `RerunNonConfirmatory`: the caller keeps/writes the marker and
///   stamps the report non-confirmatory (that stamping is this outcome's contract).
pub fn oos_gate(marker_path: &str, override_flag: bool) -> Result<OosDecision, String> {
    if !Path::new(marker_path).exists() { return Ok(OosDecision::Allow); }
    if override_flag { Ok(OosDecision::RerunNonConfirmatory) } else { Ok(OosDecision::Refuse) }
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

    #[test]
    fn present_but_mismatched_lock_is_error() {
        let path = std::env::temp_dir().join(format!("strategy_analyzer_lock_{}.txt", std::process::id()));
        std::fs::write(&path, "deadbeef").unwrap();
        let err = check_lock(path.to_str().unwrap(), "cafef00d").unwrap_err();
        assert!(err.contains("lock mismatch"), "wrong hash must be a mismatch, got: {err}");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn oos_gate_covers_all_three_branches() {
        let marker = std::env::temp_dir().join(format!("strategy_analyzer_oos_gate_{}.marker", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let p = marker.to_str().unwrap();
        // No marker -> Allow, with or without the override flag.
        assert_eq!(oos_gate(p, false).unwrap(), OosDecision::Allow);
        assert_eq!(oos_gate(p, true).unwrap(), OosDecision::Allow);
        // Marker present, no override -> Refuse; with override -> non-confirmatory rerun.
        std::fs::write(&marker, "prior-run").unwrap();
        assert_eq!(oos_gate(p, false).unwrap(), OosDecision::Refuse);
        assert_eq!(oos_gate(p, true).unwrap(), OosDecision::RerunNonConfirmatory);
        std::fs::remove_file(&marker).unwrap();
    }
}
