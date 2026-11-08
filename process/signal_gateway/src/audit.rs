use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use share::ProcessError;

#[derive(Debug, Serialize)]
pub struct AuditRecord {
    pub at: String,
    pub candle_open: String,
    pub symbol: String,
    pub prompt: String,
    pub response: String,
    pub verdict: String,
    pub validation: String,
    pub signal_id: Option<String>,
    pub ack: Option<String>,
}

pub fn append(path: &Path, record: &AuditRecord) -> Result<(), ProcessError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            ProcessError::Network(format!("create audit dir {}: {e}", parent.display()))
        })?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| ProcessError::Network(format!("open audit {}: {e}", path.display())))?;

    let line = serde_json::to_string(record)?;
    writeln!(file, "{line}").map_err(|e| ProcessError::Network(format!("write audit: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn appends_valid_json_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let record = AuditRecord {
            at: "2026-11-07T00:01:30Z".into(),
            candle_open: "2026-11-07T00:00:00Z".into(),
            symbol: "BTC-USDT".into(),
            prompt: "prompt".into(),
            response: "response".into(),
            verdict: "LONG".into(),
            validation: "valid".into(),
            signal_id: Some("sg:1:BTC-USDT".into()),
            ack: Some("accepted=true".into()),
        };
        append(&path, &record).unwrap();
        append(&path, &record).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
    }
}
