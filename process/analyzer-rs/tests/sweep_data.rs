#![cfg(feature = "harness")]
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::sweep::data::{
    dedup_and_sort, fetch_range_with, missing_candles, provenance, read_cache, write_cache,
    write_provenance, DatasetProvenance,
};
use analyzer_rs::strategy::types::Candle;
use rust_decimal::Decimal;

fn c(t: i64) -> Candle {
    Candle {
        open_time_ms: t,
        open: Decimal::ONE,
        high: Decimal::ONE,
        low: Decimal::ONE,
        close: Decimal::ONE,
        volume: Decimal::ONE,
    }
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "analyzer-rs-sweep-data-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn dedup_sorts_and_reports_missing() {
    let raw = vec![c(2 * 3_600_000), c(0), c(3_600_000), c(3_600_000)];
    let clean = dedup_and_sort(raw);
    assert_eq!(clean.len(), 3);
    assert_eq!(clean[0].open_time_ms, 0);
    assert_eq!(missing_candles(&clean, Timeframe::H1), 0);
    let with_gap = vec![c(0), c(3 * 3_600_000)];
    assert_eq!(missing_candles(&with_gap, Timeframe::H1), 2);
}

#[test]
fn fetch_range_with_walks_pages_until_start_is_covered() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let start_ms = 0;
    let end_ms = 20 * step;
    let mut cursors = Vec::new();
    let (candles, pages, duplicates) =
        fetch_range_with(tf, start_ms, end_ms, 0, i64::MAX, |cursor| {
            cursors.push(cursor);
            if cursor == end_ms {
                Ok(vec![c(10 * step), c(15 * step), c(20 * step)])
            } else {
                Ok(vec![c(0), c(5 * step), c(9 * step), c(15 * step)])
            }
        })
        .unwrap();

    assert_eq!(pages, 2);
    assert_eq!(duplicates, 1);
    assert_eq!(cursors, vec![end_ms, 10 * step - step]);
    let times: Vec<i64> = candles.iter().map(|candle| candle.open_time_ms).collect();
    assert_eq!(
        times,
        vec![0, 5 * step, 9 * step, 10 * step, 15 * step, 20 * step]
    );
    assert!(candles
        .windows(2)
        .all(|pair| pair[0].open_time_ms < pair[1].open_time_ms));
}

#[test]
fn fetch_range_with_keeps_inclusive_bounds_and_drops_unclosed_end() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let start_ms = 10 * step;
    let end_ms = 12 * step;
    // The candle opening at `end_ms` has not closed yet at this clock.
    let now_ms = end_ms + step;
    let (candles, pages, duplicates) =
        fetch_range_with(tf, start_ms, end_ms, 0, now_ms, |_cursor| {
            Ok(vec![
                c(start_ms - 1),
                c(start_ms),
                c(start_ms + step),
                c(end_ms),
                c(end_ms + 1),
            ])
        })
        .unwrap();

    assert_eq!(pages, 1);
    assert_eq!(duplicates, 0);
    let times: Vec<i64> = candles.iter().map(|candle| candle.open_time_ms).collect();
    assert_eq!(times, vec![start_ms, start_ms + step]);
}

#[test]
fn fetch_range_with_stops_on_an_empty_first_page() {
    let tf = Timeframe::H1;
    let mut calls = 0;
    let (candles, pages, duplicates) =
        fetch_range_with(tf, 0, 10 * tf.millis(), 0, i64::MAX, |_cursor| {
            calls += 1;
            Ok(Vec::<Candle>::new())
        })
        .unwrap();

    assert!(candles.is_empty());
    assert_eq!(pages, 0);
    assert_eq!(duplicates, 0);
    assert_eq!(calls, 1);
}

#[test]
fn fetch_range_with_counts_overlap_duplicates_before_filtering() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let start_ms = 5 * step;
    let end_ms = 20 * step;
    let (candles, pages, duplicates) =
        fetch_range_with(tf, start_ms, end_ms, 0, i64::MAX, |cursor| {
            if cursor == end_ms {
                Ok(vec![c(10 * step), c(15 * step), c(20 * step)])
            } else {
                Ok(vec![c(3 * step), c(9 * step), c(15 * step)])
            }
        })
        .unwrap();

    assert_eq!(pages, 2);
    // Only the overlapping 15 * step is a duplicate; 3 * step is removed by
    // the range filter and must not be counted as a duplicate.
    assert_eq!(duplicates, 1);
    let times: Vec<i64> = candles.iter().map(|candle| candle.open_time_ms).collect();
    assert_eq!(times, vec![9 * step, 10 * step, 15 * step, 20 * step]);
}

#[test]
fn fetch_range_with_rejects_inverted_ranges() {
    let error = fetch_range_with(
        Timeframe::H1,
        10 * Timeframe::H1.millis(),
        5 * Timeframe::H1.millis(),
        0,
        i64::MAX,
        |_cursor| Ok(Vec::<Candle>::new()),
    )
    .unwrap_err();
    assert!(error.contains("invalid range"));
}

#[test]
fn provenance_reports_duplicates_before_filtering() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let candles = vec![c(9 * step), c(10 * step), c(15 * step), c(20 * step)];
    let p = provenance("BTC-USDT", tf, 5 * step, 20 * step, 1, &candles, 2);

    assert_eq!(p.candles, 4);
    assert_eq!(p.requested_candles, 16);
    assert_eq!(p.expected_candles, 12);
    assert_eq!(p.missing_candles, 8);
    // Inferring this from raw minus retained candles would be inflated by
    // out-of-range filtering; the explicit count stays at one.
    assert_eq!(p.duplicates_dropped, 1);
}

#[test]
fn provenance_json_round_trips_next_to_the_cache() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let dir = temp_dir("provenance");
    let csv_path = dir.join("BTC-USDT_1hour.csv");
    let p = provenance("BTC-USDT", tf, 0, 3 * step, 2, &[c(0), c(step)], 4);

    write_provenance(&csv_path, &p).unwrap();
    let text = std::fs::read_to_string(csv_path.with_extension("csv.provenance.json")).unwrap();
    let back: DatasetProvenance = serde_json::from_str(&text).unwrap();
    assert_eq!(back.duplicates_dropped, p.duplicates_dropped);
    assert_eq!(back.requested_candles, p.requested_candles);
    assert_eq!(back.content_sha256, p.content_sha256);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cache_round_trips_csv() {
    let dir = temp_dir("cache");
    let path = dir.join("BTC-USDT_1hour.csv");
    write_cache(&path, &[c(0), c(3_600_000)]).unwrap();
    let back = read_cache(&path).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].open_time_ms, 3_600_000);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cache_csv_is_accepted_by_the_dataset_loader() {
    let tf = Timeframe::H1;
    let step = tf.millis();
    let dir = temp_dir("interop");
    let path = dir.join("BTC-USDT_1hour.csv");
    let candles: Vec<Candle> = (0..8).map(|index| c(index * step)).collect();
    write_cache(&path, &candles).unwrap();

    let loaded = load_csv("BTC-USDT", &path, tf, i64::MAX, 0, 0, 5, 8).unwrap();
    let expected = provenance("BTC-USDT", tf, 0, 7 * step, 0, &candles, 1);
    assert_eq!(loaded.provenance.content_sha256, expected.content_sha256);
    std::fs::remove_dir_all(&dir).unwrap();
}
