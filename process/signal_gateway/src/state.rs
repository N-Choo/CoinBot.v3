use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use share::ProcessError;

pub const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LastPublished {
    pub candle_open_ms: i64,
    pub signal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub version: u32,
    pub seq: u64,
    pub cursor: usize,
    pub last_processed: BTreeMap<String, i64>,
    #[serde(default)]
    pub last_published: BTreeMap<String, LastPublished>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            seq: 0,
            cursor: 0,
            last_processed: BTreeMap::new(),
            last_published: BTreeMap::new(),
        }
    }
}

impl State {
    pub fn load(path: &Path, reset: bool) -> Result<Self, ProcessError> {
        if reset {
            if path.exists() {
                let backup = path.with_extension("json.bak");
                fs::rename(path, &backup).map_err(|e| {
                    ProcessError::Network(format!(
                        "backup state {} -> {}: {e}",
                        path.display(),
                        backup.display()
                    ))
                })?;
            }
            return Ok(Self::default());
        }

        match fs::read_to_string(path) {
            Ok(raw) => {
                let state: State = serde_json::from_str(&raw).map_err(|e| {
                    ProcessError::InvalidConfig(format!("corrupt state {}: {e}", path.display()))
                })?;
                if state.version != STATE_VERSION {
                    return Err(ProcessError::InvalidConfig(format!(
                        "unsupported state version {}",
                        state.version
                    )));
                }
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(ProcessError::Network(format!(
                "read state {}: {e}",
                path.display()
            ))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ProcessError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|e| {
                ProcessError::Network(format!("create state dir {}: {e}", parent.display()))
            })?;
        }

        let tmp = path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(&tmp, raw)
            .map_err(|e| ProcessError::Network(format!("write state {}: {e}", tmp.display())))?;
        fs::rename(&tmp, path)
            .map_err(|e| ProcessError::Network(format!("rename state {}: {e}", path.display())))?;
        Ok(())
    }

    pub fn next_seq(&mut self) -> u64 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    pub fn mark_processed(&mut self, symbol: &str, candle_open_ms: i64) {
        self.last_processed
            .insert(symbol.to_string(), candle_open_ms);
    }

    pub fn is_processed(&self, symbol: &str, candle_open_ms: i64) -> bool {
        self.last_processed.get(symbol) == Some(&candle_open_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_starts_fresh() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        let state = State::load(&path, false).unwrap();
        assert_eq!(state, State::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/state.json");
        let mut state = State::default();
        state.next_seq();
        state.mark_processed("BTC-USDT", 1000);
        state.cursor = 2;
        state.save(&path).unwrap();

        let loaded = State::load(&path, false).unwrap();
        assert_eq!(loaded.seq, 1);
        assert_eq!(loaded.cursor, 2);
        assert!(loaded.is_processed("BTC-USDT", 1000));
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, "not json").unwrap();
        assert!(State::load(&path, false).is_err());
    }

    #[test]
    fn reset_moves_corrupt_file_aside() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, "not json").unwrap();
        let state = State::load(&path, true).unwrap();
        assert_eq!(state, State::default());
        assert!(dir.path().join("state.json.bak").exists());
    }

    #[test]
    fn seq_is_monotonic() {
        let mut state = State::default();
        assert_eq!(state.next_seq(), 0);
        assert_eq!(state.next_seq(), 1);
        assert_eq!(state.next_seq(), 2);
    }
}
