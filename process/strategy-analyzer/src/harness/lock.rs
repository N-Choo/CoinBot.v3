//! Freeze lock, OOS boundary guard, and the one-shot OOS run log.
//!
//! The lock is a canonical JSON document (not just a bare hash) so it can **include** everything
//! the pre-registration depends on: the git commit, the per-symbol dataset hashes, the
//! in-sample/OOS boundary, and the pre-registered constants. Its identity is the sha256 of that
//! JSON, so changing any of those inputs changes the lock and no OOS run can silently drift.

use crate::harness::perturb;
use crate::harness::report::{AGG_TOLERANCE, BOOTSTRAP_BLOCK_MS};
use crate::harness::verdict::{BOOTSTRAP_RESAMPLES, BOOTSTRAP_SEED, MIN_EXECUTED_TRADES};
use crate::strategy::types::Candle;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

/// Pre-registered in-sample/OOS boundary (UTC ms): data **at/after** this timestamp is OOS. It is
/// committed to the repository before any exploration (the pre-registration) and recorded into the
/// freeze lock, so it cannot be moved after seeing OOS results.
pub const OOS_BOUNDARY_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

/// The first candle at/after `boundary_ms`, if any. `count-signals` uses this to refuse OOS rows so
/// the count-only sizing run can never see the confirmatory segment.
pub fn first_oos_candle(candles: &[Candle], boundary_ms: i64) -> Option<i64> {
    candles
        .iter()
        .find(|c| c.open_time_ms >= boundary_ms)
        .map(|c| c.open_time_ms)
}

/// The pre-registered constants the lock must cover (spec §10.4). Values are read from their single
/// sources of truth (`verdict`, `report`, `perturb`) — never duplicated as literals here — so a
/// constant change is reflected in the lock automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedConstants {
    pub min_executed_trades: usize,
    pub bootstrap_resamples: usize,
    pub bootstrap_seed: u64,
    pub bootstrap_block_ms: i64,
    pub agg_tolerance: String,
    pub oat_positive_fraction_min: String,
    pub joint_positive_fraction_min: String,
    pub joint_draws: usize,
    pub joint_seed: u64,
}

impl LockedConstants {
    /// Snapshot the single-source-of-truth constants. Floats are stored as strings so the lock
    /// JSON stays exactly comparable (and `Eq`).
    pub fn from_constants() -> Self {
        Self {
            min_executed_trades: MIN_EXECUTED_TRADES,
            bootstrap_resamples: BOOTSTRAP_RESAMPLES,
            bootstrap_seed: BOOTSTRAP_SEED,
            bootstrap_block_ms: BOOTSTRAP_BLOCK_MS,
            agg_tolerance: AGG_TOLERANCE.to_string(),
            oat_positive_fraction_min: perturb::OAT_POSITIVE_FRACTION_MIN.to_string(),
            joint_positive_fraction_min: perturb::JOINT_POSITIVE_FRACTION_MIN.to_string(),
            joint_draws: perturb::JOINT_DRAWS,
            joint_seed: perturb::JOINT_SEED,
        }
    }
}

/// Pre-registered backtest parameters (recorded in the lock, §3/§10.4 — not strategy config).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestParams {
    pub policy: String,
    pub mode: String,
    pub fee_bps: String,
    pub entry_slip_bps: String,
    pub stop_slip_bps: String,
    pub borrow_bps_per_day: String,
    /// The entry fill rule: the touching tick fills; latency is a fill-model assumption, not a
    /// tick index (§8.4).
    pub entry_fill: String,
    pub constants: LockedConstants,
}

/// The full lock document. `hash()` is the freeze-lock identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockContents {
    pub git_commit: String,
    pub oos_boundary_ms: i64,
    pub config_version: String,
    pub dataset_hashes: BTreeMap<String, String>,
    pub params: BacktestParams,
}

impl LockContents {
    /// Deterministic canonical JSON (BTreeMap => stable key order).
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("lock is serializable")
    }

    /// sha256 hex of the canonical JSON.
    pub fn hash(&self) -> String {
        lock_hash(&self.canonical_json())
    }
}

/// lock = sha256 hex of the canonical lock JSON (which folds config_version + backtest params +
/// git commit + dataset hashes + boundary).
pub fn lock_hash(canonical_json: &str) -> String {
    let mut h = Sha256::new();
    h.update(canonical_json.as_bytes());
    format!("{:x}", h.finalize())
}

/// The git commit this run is built from. Kept out of the core; this is harness I/O.
pub fn git_commit_hash() -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git rev-parse failed: {e}"))?;
    if !out.status.success() {
        return Err("git rev-parse HEAD failed (not a git checkout?)".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Parse the lock file and verify its hash matches `expected`; returns the parsed contents on
/// success. A missing file is an error (no lock => exploration only).
pub fn check_lock(lock_path: &str, expected: &str) -> Result<LockContents, String> {
    let raw = std::fs::read_to_string(lock_path)
        .map_err(|_| format!("no lock file at {lock_path}; in-sample exploration only"))?;
    let trimmed = raw.trim();
    let got = lock_hash(trimmed);
    if got != expected {
        return Err(format!("lock mismatch: expected {expected}, found {got}"));
    }
    serde_json::from_str(trimmed).map_err(|e| format!("malformed lock at {lock_path}: {e}"))
}

pub fn write_lock(lock_path: &str, value: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(lock_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(lock_path, value)
}

/// One OOS run's record. The log is append-only so the **first (confirmatory)** run's record is
/// preserved even when a later run overrides the one-shot guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OosLogRecord {
    pub dataset_hashes: BTreeMap<String, String>,
    pub verdict: String,
}

/// Append one run record as a JSON line. Never truncates.
pub fn append_oos_log(log_path: &str, record: &OosLogRecord) -> std::io::Result<()> {
    if let Some(parent) = Path::new(log_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(
        f,
        "{}",
        serde_json::to_string(record).expect("record is serializable")
    )
}

/// True if at least one OOS run has been logged (the one-shot guard).
pub fn oos_consumed(log_path: &str) -> bool {
    std::fs::metadata(log_path)
        .map(|m| m.len() > 0)
        .unwrap_or(false)
}

/// The most recently logged OOS record, if any (used to report the earlier run on override).
pub fn last_oos_record(log_path: &str) -> Option<OosLogRecord> {
    let text = std::fs::read_to_string(log_path).ok()?;
    text.lines()
        .rfind(|l| !l.trim().is_empty())
        .and_then(|l| serde_json::from_str(l).ok())
}

/// Outcome of the one-shot OOS guard for a consumed-log path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OosDecision {
    Allow,
    RerunNonConfirmatory,
    Refuse,
}

/// One-shot OOS gate keyed on the append-only run log (`<lock>.oos.log`):
/// - empty/absent log        -> `Allow`: the first, confirmatory OOS run.
/// - log, no override        -> `Refuse`: OOS has already been consumed.
/// - log, `override_flag`    -> `RerunNonConfirmatory`: the caller appends this run and stamps the
///   report non-confirmatory (that stamping is this outcome's contract).
pub fn oos_gate(log_path: &str, override_flag: bool) -> Result<OosDecision, String> {
    if !oos_consumed(log_path) {
        return Ok(OosDecision::Allow);
    }
    if override_flag {
        Ok(OosDecision::RerunNonConfirmatory)
    } else {
        Ok(OosDecision::Refuse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_with(constants: LockedConstants) -> LockContents {
        let mut dataset_hashes = BTreeMap::new();
        dataset_hashes.insert("XRP-USDT".to_string(), "aa".to_string());
        LockContents {
            git_commit: "deadbeef".to_string(),
            oos_boundary_ms: OOS_BOUNDARY_MS,
            config_version: "cfg-v1".to_string(),
            dataset_hashes,
            params: BacktestParams {
                policy: "both".to_string(),
                mode: "both".to_string(),
                fee_bps: "5".to_string(),
                entry_slip_bps: "5".to_string(),
                stop_slip_bps: "15".to_string(),
                borrow_bps_per_day: "2".to_string(),
                entry_fill: "touching_tick".to_string(),
                constants,
            },
        }
    }

    #[test]
    fn lock_hash_depends_on_identity_and_dataset() {
        // Changing the git commit, the boundary, or a dataset hash must change the lock.
        let base = lock_with(LockedConstants::from_constants());
        let mut other_commit = base.clone();
        other_commit.git_commit = "cafef00d".to_string();
        assert_ne!(base.hash(), other_commit.hash());

        let mut other_boundary = base.clone();
        other_boundary.oos_boundary_ms += 1;
        assert_ne!(base.hash(), other_boundary.hash());

        let mut other_dataset = base.clone();
        other_dataset
            .dataset_hashes
            .insert("XRP-USDT".to_string(), "bb".to_string());
        assert_ne!(base.hash(), other_dataset.hash());
    }

    #[test]
    fn changing_a_constant_changes_the_lock_hash() {
        let base = lock_with(LockedConstants::from_constants());
        let mut changed = LockedConstants::from_constants();
        changed.min_executed_trades += 1;
        assert_ne!(base.hash(), lock_with(changed).hash());
    }

    #[test]
    fn first_oos_candle_is_inclusive_of_the_boundary() {
        let d = |ms: i64| Candle {
            open_time_ms: ms,
            open: rust_decimal::Decimal::ONE,
            high: rust_decimal::Decimal::ONE,
            low: rust_decimal::Decimal::ONE,
            close: rust_decimal::Decimal::ONE,
        };
        assert_eq!(first_oos_candle(&[d(0), d(99), d(100)], 100), Some(100));
        assert_eq!(first_oos_candle(&[d(0), d(99)], 100), None);
    }

    #[test]
    fn check_lock_round_trips_contents() {
        let lock = lock_with(LockedConstants::from_constants());
        let path = std::env::temp_dir().join(format!(
            "strategy_analyzer_lock_roundtrip_{}.json",
            std::process::id()
        ));
        write_lock(path.to_str().unwrap(), &lock.canonical_json()).unwrap();
        let parsed = check_lock(path.to_str().unwrap(), &lock.hash()).unwrap();
        assert_eq!(parsed, lock);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn missing_lock_is_error() {
        assert!(check_lock("/nonexistent/lock", "x").is_err());
    }

    #[test]
    fn present_but_mismatched_lock_is_error() {
        let path =
            std::env::temp_dir().join(format!("strategy_analyzer_lock_{}.txt", std::process::id()));
        std::fs::write(&path, "deadbeef").unwrap();
        let err = check_lock(path.to_str().unwrap(), "cafef00d").unwrap_err();
        assert!(
            err.contains("lock mismatch"),
            "wrong hash must be a mismatch, got: {err}"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn oos_gate_covers_all_three_branches() {
        let log = std::env::temp_dir().join(format!(
            "strategy_analyzer_oos_gate_{}.log",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&log);
        let p = log.to_str().unwrap();
        // No log -> Allow, with or without the override flag.
        assert_eq!(oos_gate(p, false).unwrap(), OosDecision::Allow);
        assert_eq!(oos_gate(p, true).unwrap(), OosDecision::Allow);
        // A log record, no override -> Refuse; with override -> non-confirmatory rerun.
        let rec = OosLogRecord {
            dataset_hashes: BTreeMap::new(),
            verdict: "pass".to_string(),
        };
        append_oos_log(p, &rec).unwrap();
        assert_eq!(oos_gate(p, false).unwrap(), OosDecision::Refuse);
        assert_eq!(
            oos_gate(p, true).unwrap(),
            OosDecision::RerunNonConfirmatory
        );
        std::fs::remove_file(&log).unwrap();
    }

    #[test]
    fn append_log_preserves_the_first_record() {
        let log = std::env::temp_dir().join(format!(
            "strategy_analyzer_oos_append_{}.log",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&log);
        let p = log.to_str().unwrap();
        let first = OosLogRecord {
            dataset_hashes: BTreeMap::from([("A".to_string(), "h1".to_string())]),
            verdict: "fail".to_string(),
        };
        let second = OosLogRecord {
            dataset_hashes: BTreeMap::from([("A".to_string(), "h2".to_string())]),
            verdict: "pass".to_string(),
        };
        append_oos_log(p, &first).unwrap();
        append_oos_log(p, &second).unwrap();
        let text = std::fs::read_to_string(p).unwrap();
        assert_eq!(
            text.lines().count(),
            2,
            "both runs must be recorded, not overwritten"
        );
        assert_eq!(last_oos_record(p).unwrap(), second);
        // The first confirmatory record is still present.
        assert!(text.contains("\"h1\""));
        std::fs::remove_file(&log).unwrap();
    }
}
