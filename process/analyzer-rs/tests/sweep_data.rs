#![cfg(feature = "harness")]
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::sweep::data::{dedup_and_sort, missing_candles, read_cache, write_cache};
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
fn cache_round_trips_csv() {
    let dir = std::env::temp_dir().join(format!("sweep-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("BTC-USDT_1hour.csv");
    write_cache(&path, &[c(0), c(3_600_000)]).unwrap();
    let back = read_cache(&path).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].open_time_ms, 3_600_000);
    std::fs::remove_dir_all(&dir).unwrap();
}
