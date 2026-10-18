//! Per-symbol file state store (live spec §4).
//!
//! - **Atomic writes:** temp file → `fsync(file)` → `rename` → `fsync(dir)`.
//! - **Single writer:** an instance lock file in the state directory, so two analyzers cannot write
//!   the same state.
//! - **Validate-or-re-warm:** missing / corrupt / schema-mismatched / too-old state is discarded and
//!   reported as [`LoadOutcome::Cold`]; the caller re-warms from candle history.
//! - **Pruning:** delete state for symbols no longer active.
//!
//! The serialized payload wraps the real [`StrategyState`] in an envelope carrying the save time, so
//! the age check does not depend on any core field.

use crate::strategy::types::StrategyState;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The lock file name inside the state directory (one per analyzer instance).
const LOCK_FILE: &str = "analyzer.lock";

/// Errors from the state store.
#[derive(Debug, thiserror::Error)]
pub enum StateStoreError {
    #[error("state directory {0} could not be created: {1}")]
    Dir(String, std::io::Error),
    #[error("state store is already locked by another instance ({0})")]
    Locked(String),
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("state for {0} is not serializable: {1}")]
    Serialize(String, serde_json::Error),
}

/// The outcome of a load: a usable state, or a reason a cold re-warm is required.
// Same rationale as the core `Output` enum: the large variant is the full `StrategyState`, and the
// enum is not hot-path allocated, so indirection would only add noise.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome {
    Warm(StrategyState),
    Cold(ColdReason),
}

/// Why a load fell back to a cold re-warm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColdReason {
    Missing,
    Corrupt(String),
    SchemaMismatch { found: u32, expected: u32 },
    TooOld { saved_at_ms: i64, age_ms: i64 },
}

/// The on-disk envelope: adapter metadata next to the untouched core state.
#[derive(Debug, Serialize, Deserialize)]
struct StateEnvelope {
    schema_version: u32,
    saved_at_ms: i64,
    state: StrategyState,
}

/// Holds the instance lock for its lifetime; removing the lock file lets a fresh instance start.
struct InstanceLock {
    path: PathBuf,
}

impl InstanceLock {
    fn acquire(dir: &Path) -> Result<Self, StateStoreError> {
        let path = dir.join(LOCK_FILE);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                let _ = write!(f, "{}", std::process::id());
                Ok(InstanceLock { path })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(StateStoreError::Locked(path.display().to_string()))
            }
            Err(source) => Err(StateStoreError::Io {
                path: path.display().to_string(),
                source,
            }),
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// File-per-symbol state with an instance lock held for the store's lifetime.
pub struct StateStore {
    dir: PathBuf,
    schema_version: u32,
    _lock: InstanceLock,
}

impl StateStore {
    /// Open (creating if needed) the state directory and acquire the single-writer lock.
    pub fn open(dir: impl AsRef<Path>, schema_version: u32) -> Result<Self, StateStoreError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| StateStoreError::Dir(dir.display().to_string(), e))?;
        let lock = InstanceLock::acquire(&dir)?;
        Ok(StateStore {
            dir,
            schema_version,
            _lock: lock,
        })
    }

    /// The state file path for `symbol`.
    pub fn path_for(&self, symbol: &str) -> PathBuf {
        self.dir.join(format!("{symbol}.json"))
    }

    /// The state directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Load `symbol`'s state, validating it against the schema and (optionally) an age threshold.
    pub fn load(&self, symbol: &str, now_ms: i64, max_age_ms: Option<i64>) -> LoadOutcome {
        let path = self.path_for(symbol);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return LoadOutcome::Cold(ColdReason::Missing),
        };
        let envelope: StateEnvelope = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(e) => return LoadOutcome::Cold(ColdReason::Corrupt(e.to_string())),
        };
        // Both the envelope and the inner core state carry the schema version; either mismatch
        // means the shapes are not interchangeable.
        if envelope.schema_version != self.schema_version
            || envelope.state.schema_version != self.schema_version
        {
            let found = envelope.state.schema_version;
            return LoadOutcome::Cold(ColdReason::SchemaMismatch {
                found,
                expected: self.schema_version,
            });
        }
        if envelope.state.symbol != symbol {
            return LoadOutcome::Cold(ColdReason::Corrupt(format!(
                "state file for {symbol} holds symbol {:?}",
                envelope.state.symbol
            )));
        }
        if let Some(max) = max_age_ms {
            let age = now_ms.saturating_sub(envelope.saved_at_ms);
            if age > max {
                return LoadOutcome::Cold(ColdReason::TooOld {
                    saved_at_ms: envelope.saved_at_ms,
                    age_ms: age,
                });
            }
        }
        LoadOutcome::Warm(envelope.state)
    }

    /// Atomically persist `state` (temp → fsync(file) → rename → fsync(dir)).
    pub fn save(&self, state: &StrategyState, now_ms: i64) -> Result<(), StateStoreError> {
        let envelope = StateEnvelope {
            schema_version: self.schema_version,
            saved_at_ms: now_ms,
            state: state.clone(),
        };
        let mut bytes = serde_json::to_vec(&envelope)
            .map_err(|e| StateStoreError::Serialize(state.symbol.clone(), e))?;
        bytes.push(b'\n');
        let path = self.path_for(&state.symbol);
        atomic_write(&path, &bytes)
    }

    /// Delete state files for symbols not in `keep`, returning the removed symbols (sorted).
    pub fn prune(&self, keep: &[String]) -> Result<Vec<String>, StateStoreError> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| StateStoreError::Io {
            path: self.dir.display().to_string(),
            source: e,
        })?;
        let mut removed = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| StateStoreError::Io {
                path: self.dir.display().to_string(),
                source: e,
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if keep.iter().any(|k| k == stem) {
                continue;
            }
            std::fs::remove_file(&path).map_err(|e| StateStoreError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            removed.push(stem.to_string());
        }
        removed.sort();
        Ok(removed)
    }
}

/// Atomic write: temp → fsync(file) → rename → fsync(dir). The temp file lives in the same
/// directory so the rename is a same-filesystem atomic replace.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StateStoreError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("state.json");
    let tmp = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));

    {
        let mut f = File::create(&tmp).map_err(|source| StateStoreError::Io {
            path: tmp.display().to_string(),
            source,
        })?;
        f.write_all(bytes).map_err(|source| StateStoreError::Io {
            path: tmp.display().to_string(),
            source,
        })?;
        f.sync_all().map_err(|source| StateStoreError::Io {
            path: tmp.display().to_string(),
            source,
        })?;
    }
    std::fs::rename(&tmp, path).map_err(|source| StateStoreError::Io {
        path: path.display().to_string(),
        source,
    })?;
    // fsync the directory so the rename itself is durable.
    let df = File::open(dir).map_err(|source| StateStoreError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    df.sync_all().map_err(|source| StateStoreError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(tag: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let d =
            std::env::temp_dir().join(format!("sa_live_state_{tag}_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn state(symbol: &str, v: u32) -> StrategyState {
        let mut s = StrategyState::new(symbol, v);
        s.candle_count = 7;
        s
    }

    #[test]
    fn save_load_round_trips_the_real_state() {
        let dir = temp_dir("roundtrip");
        let store = StateStore::open(&dir, 2).unwrap();
        let original = state("XRP-USDT", 2);
        store.save(&original, 1_000).unwrap();

        let store2 = {
            // Drop the lock so a fresh store can open the same directory.
            drop(store);
            StateStore::open(&dir, 2).unwrap()
        };
        match store2.load("XRP-USDT", 1_000, None) {
            LoadOutcome::Warm(s) => assert_eq!(s, original),
            other => panic!("expected warm state, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_writer_lock_rejects_a_second_instance() {
        let dir = temp_dir("lock");
        let _first = StateStore::open(&dir, 2).unwrap();
        match StateStore::open(&dir, 2) {
            Err(StateStoreError::Locked(_)) => {}
            Err(e) => panic!("expected Locked, got {e:?}"),
            Ok(_) => panic!("expected Locked, but a second store opened"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_state_is_cold() {
        let dir = temp_dir("missing");
        let store = StateStore::open(&dir, 2).unwrap();
        assert_eq!(
            store.load("XRP-USDT", 0, None),
            LoadOutcome::Cold(ColdReason::Missing)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_state_is_cold() {
        let dir = temp_dir("corrupt");
        let store = StateStore::open(&dir, 2).unwrap();
        std::fs::write(store.path_for("XRP-USDT"), b"{not json").unwrap();
        assert!(matches!(
            store.load("XRP-USDT", 0, None),
            LoadOutcome::Cold(ColdReason::Corrupt(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_mismatch_is_cold() {
        let dir = temp_dir("schema");
        // Store schema 2 but write a schema-1 envelope/state.
        let store = StateStore::open(&dir, 2).unwrap();
        let mut old = state("XRP-USDT", 1);
        old.schema_version = 1;
        let envelope = serde_json::json!({
            "schema_version": 1,
            "saved_at_ms": 0,
            "state": old,
        });
        std::fs::write(store.path_for("XRP-USDT"), envelope.to_string()).unwrap();
        assert!(matches!(
            store.load("XRP-USDT", 0, None),
            LoadOutcome::Cold(ColdReason::SchemaMismatch {
                found: 1,
                expected: 2
            })
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn too_old_state_is_cold() {
        let dir = temp_dir("old");
        let store = StateStore::open(&dir, 2).unwrap();
        store.save(&state("XRP-USDT", 2), 1_000).unwrap();
        match store.load("XRP-USDT", 1_000 + 10_000, Some(5_000)) {
            LoadOutcome::Cold(ColdReason::TooOld {
                saved_at_ms,
                age_ms,
            }) => {
                assert_eq!(saved_at_ms, 1_000);
                assert_eq!(age_ms, 10_000);
            }
            other => panic!("expected TooOld, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn within_age_is_warm() {
        let dir = temp_dir("fresh");
        let store = StateStore::open(&dir, 2).unwrap();
        store.save(&state("XRP-USDT", 2), 1_000).unwrap();
        assert!(matches!(
            store.load("XRP-USDT", 1_100, Some(5_000)),
            LoadOutcome::Warm(_)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_removes_inactive_symbols_only() {
        let dir = temp_dir("prune");
        let store = StateStore::open(&dir, 2).unwrap();
        store.save(&state("XRP-USDT", 2), 0).unwrap();
        store.save(&state("BTC-USDT", 2), 0).unwrap();
        let removed = store.prune(&["XRP-USDT".to_string()]).unwrap();
        assert_eq!(removed, vec!["BTC-USDT".to_string()]);
        assert!(store.path_for("XRP-USDT").exists());
        assert!(!store.path_for("BTC-USDT").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
