#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, DatasetError};

fn write_csv(name: &str, rows: &[(&str, &str, &str, &str, &str)]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let mut text = String::from("open_time_ms,Open,High,Low,Close,Volume\n");
    for (t, o, h, l, c) in rows {
        text.push_str(&format!("{t},{o},{h},{l},{c},1\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn loads_and_hashes_a_valid_csv() {
    let path = write_csv(
        "analyzer-rs-valid.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 3);
    assert_eq!(dataset.provenance.gaps, 0);
    assert_eq!(dataset.provenance.content_sha256.len(), 64);
    let again = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap();
    assert_eq!(
        dataset.provenance.content_sha256,
        again.provenance.content_sha256
    );
}

#[test]
fn rejects_bad_ohlc() {
    let path = write_csv(
        "analyzer-rs-bad-ohlc.csv",
        &[("1700000000000", "100", "99", "101", "100")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::InvalidOhlc { .. }));
}

#[test]
fn rejects_a_timeframe_mismatch() {
    let path = write_csv(
        "analyzer-rs-tf.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000900000", "101", "103", "100", "102"),
        ],
    );
    // The rows are 15 minutes apart, so they cannot form a D1 series.
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::TimeframeMismatch { .. }));
}

#[test]
fn counts_gaps_and_enforces_the_gap_limit() {
    let path = write_csv(
        "analyzer-rs-gap.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    assert!(matches!(
        load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap_err(),
        DatasetError::Duplicate { .. }
    ));

    let path = write_csv(
        "analyzer-rs-gap2.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0).unwrap();
    assert_eq!(dataset.provenance.gaps, 2);
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 1, 0).unwrap_err();
    assert!(matches!(error, DatasetError::GapTooLarge { .. }));
}
