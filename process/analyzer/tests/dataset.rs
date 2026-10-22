#![cfg(feature = "harness")]

use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{load_csv, DatasetError};
use rust_decimal::Decimal;

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
        "analyzer-valid.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 3);
    assert_eq!(dataset.provenance.gaps, 0);
    assert_eq!(dataset.provenance.content_sha256.len(), 64);
    let again = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(
        dataset.provenance.content_sha256,
        again.provenance.content_sha256
    );
}

#[test]
fn rejects_bad_ohlc() {
    let path = write_csv(
        "analyzer-bad-ohlc.csv",
        &[("1700000000000", "100", "99", "101", "100")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::InvalidOhlc { .. }));
}

#[test]
fn rejects_a_timeframe_mismatch() {
    let path = write_csv(
        "analyzer-tf.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000900000", "101", "103", "100", "102"),
        ],
    );
    // The rows are 15 minutes apart, so they cannot form a D1 series.
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::TimeframeMismatch { .. }));
}

#[test]
fn counts_gaps_and_enforces_the_gap_limit() {
    let path = write_csv(
        "analyzer-gap.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    assert!(matches!(
        load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err(),
        DatasetError::Duplicate { .. }
    ));

    let path = write_csv(
        "analyzer-gap2.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.provenance.gaps, 2);
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 1, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::GapTooLarge { .. }));
}

#[test]
fn loads_the_lowercase_synthetic_fixture() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/synthetic_1day.csv");
    let dataset = load_csv("SYNTH-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 400);
    assert_eq!(dataset.provenance.gaps, 0);
}

#[test]
fn rejects_a_reversed_csv() {
    let path = write_csv(
        "analyzer-reversed.csv",
        &[
            ("1700086400000", "101", "103", "100", "102"),
            ("1700000000000", "100", "102", "99", "101"),
        ],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::OutOfOrder { .. }));
}

#[test]
fn drops_candles_at_the_close_boundary() {
    let third = 1700172800000i64;
    let path = write_csv(
        "analyzer-boundary.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let boundary = third + Timeframe::D1.millis();
    let dropped = load_csv("TEST-USDT", &path, Timeframe::D1, boundary, 0, 0, 0, 0).unwrap();
    assert_eq!(dropped.candles.len(), 2);
    assert_eq!(dropped.candles.last().unwrap().open_time_ms, 1700086400000);

    let kept = load_csv("TEST-USDT", &path, Timeframe::D1, boundary + 1, 0, 0, 0, 0).unwrap();
    assert_eq!(kept.candles.len(), 3);
    assert_eq!(kept.candles.last().unwrap().open_time_ms, third);
}

#[test]
fn enforces_the_warmup_window() {
    let path = write_csv(
        "analyzer-short.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 3, 0).unwrap_err();
    assert!(matches!(error, DatasetError::TooShort { got: 3, need: 5 }));
}

#[test]
fn reads_columns_by_header_name() {
    let path = std::env::temp_dir().join("analyzer-reordered.csv");
    std::fs::write(
        &path,
        "\u{feff} Volume ,High, open_time_ms , Low ,Close,Open\n\
         1,102,1700000000000,99,101,100\n\
         1,103,1700086400000,100,102,101\n",
    )
    .unwrap();
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 2);
    assert_eq!(dataset.candles[0].open_time_ms, 1700000000000);
    assert_eq!(dataset.candles[0].open, Decimal::from(100));
    assert_eq!(dataset.candles[0].high, Decimal::from(102));
    assert_eq!(dataset.candles[0].low, Decimal::from(99));
    assert_eq!(dataset.candles[0].close, Decimal::from(101));
    assert_eq!(dataset.candles[0].volume, Decimal::from(1));
}

#[test]
fn hashes_ignore_decimal_scale_but_not_values() {
    let scaled = write_csv(
        "analyzer-hash-scale-a.csv",
        &[
            ("1700000000000", "100.0", "102.00", "99.000", "101.0"),
            ("1700086400000", "101.00", "103.0", "100.000", "102.00"),
        ],
    );
    let plain = write_csv(
        "analyzer-hash-scale-b.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
        ],
    );
    let scaled = load_csv("TEST-USDT", &scaled, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    let plain = load_csv("TEST-USDT", &plain, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(
        scaled.provenance.content_sha256,
        plain.provenance.content_sha256
    );

    let changed = write_csv(
        "analyzer-hash-changed.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102.5"),
        ],
    );
    let changed = load_csv("TEST-USDT", &changed, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_ne!(
        scaled.provenance.content_sha256,
        changed.provenance.content_sha256
    );
}

#[test]
fn rejects_non_positive_prices() {
    let path = write_csv(
        "analyzer-non-positive.csv",
        &[("1700000000000", "0", "102", "99", "101")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(
        error,
        DatasetError::InvalidOhlc { why, .. } if why == "non-positive price"
    ));
}

#[test]
fn rejects_low_above_high() {
    let path = write_csv(
        "analyzer-low-high.csv",
        &[("1700000000000", "100", "101", "102", "100")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(
        error,
        DatasetError::InvalidOhlc { why, .. } if why.contains("bound")
    ));
}
