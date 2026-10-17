use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use std::str::FromStr;

/// How to handle missing intervals. KuCoin omits intervals with no trades, so `Skip`
/// is the default (a gap must not abort a backtest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPolicy { Reject, Skip, ForwardFill }

pub fn load_csv(path: &str) -> Result<Vec<Candle>, String> {
    // Forward-fill by default: KuCoin omits intervals with no trades, and the core rejects
    // gaps, so a default `Reject`/`Skip` would abort a backtest at the first empty interval.
    load_csv_with_policy(path, GapPolicy::ForwardFill, 4 * 60 * 60 * 1000)
}

pub fn load_csv_with_policy(path: &str, policy: GapPolicy, expected_interval_ms: i64) -> Result<Vec<Candle>, String> {
    let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    for rec in rdr.records() {
        let r = rec.map_err(|e| e.to_string())?;
        raw.push(Candle {
            open_time_ms: r[0].parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
            open: Decimal::from_str(&r[1]).map_err(|e| e.to_string())?,
            high: Decimal::from_str(&r[2]).map_err(|e| e.to_string())?,
            low: Decimal::from_str(&r[3]).map_err(|e| e.to_string())?,
            close: Decimal::from_str(&r[4]).map_err(|e| e.to_string())?,
        });
    }
    raw.sort_by_key(|c| c.open_time_ms);
    raw.dedup_by_key(|c| c.open_time_ms);
    let mut out: Vec<Candle> = Vec::with_capacity(raw.len());
    for c in raw {
        if let Some(prev) = out.last().map(|p| p.open_time_ms) {
            let mut t = prev;
            while t + expected_interval_ms < c.open_time_ms {
                t += expected_interval_ms;
                match policy {
                    GapPolicy::Reject => return Err(format!("gap before {}", c.open_time_ms)),
                    GapPolicy::Skip => {}
                    GapPolicy::ForwardFill => {
                        let last = out.last().unwrap().close;
                        out.push(Candle { open_time_ms: t, open: last, high: last, low: last, close: last });
                    }
                }
            }
        }
        out.push(c);
    }
    Ok(out)
}

/// Stable dataset identity: sha256 over the raw file bytes.
pub fn dataset_hash(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_fixture() {
        assert_eq!(load_csv("tests/fixtures/small_4h.csv").unwrap().len(), 5);
    }

    #[test]
    fn hash_is_stable() {
        let a = dataset_hash("tests/fixtures/small_4h.csv").unwrap();
        let b = dataset_hash("tests/fixtures/small_4h.csv").unwrap();
        assert_eq!(a, b);
    }
}
