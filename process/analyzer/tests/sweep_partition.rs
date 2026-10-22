#![cfg(feature = "harness")]
use analyzer::harness::sweep::partition::{chronological_splits, slice_with_warmup};
use analyzer::strategy::types::Candle;
use rust_decimal::Decimal;

fn candles(n: usize) -> Vec<Candle> {
    (0..n)
        .map(|i| Candle {
            open_time_ms: i as i64 * 3_600_000,
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
            volume: Decimal::ONE,
        })
        .collect()
}

#[test]
fn splits_are_chronological_and_holdout_is_last() {
    let s = chronological_splits(1000);
    assert_eq!(s.train.start, 0);
    assert_eq!(s.train.end, 600);
    assert_eq!(s.validation.start, 600);
    assert_eq!(s.validation.end, 800);
    assert_eq!(s.holdout.start, 800);
    assert_eq!(s.holdout.end, 1000);
    assert_eq!(s.folds.len(), 3);
    for f in &s.folds {
        assert!(f.val.end <= 800, "fold val must not reach holdout");
    }
    // Expanding folds: train end grows, never shrinks.
    assert!(s.folds[0].train.end < s.folds[1].train.end);
    assert!(s.folds[1].train.end < s.folds[2].train.end);
}

#[test]
fn warmup_precedes_partition_and_is_not_counted() {
    let full = candles(1000);
    let s = chronological_splits(1000);
    let slice = slice_with_warmup(&full, s.validation, 100);
    assert_eq!(slice.candles.len(), 100 + 200);
    assert_eq!(slice.partition_start, 100);
    assert_eq!(slice.candles[100].open_time_ms, full[600].open_time_ms);
}
