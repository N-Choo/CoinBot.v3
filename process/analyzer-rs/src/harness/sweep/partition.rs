use crate::strategy::types::Candle;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Slice {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fold {
    pub train: Slice,
    pub val: Slice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Splits {
    pub train: Slice,
    pub validation: Slice,
    pub holdout: Slice,
    pub folds: Vec<Fold>,
}

fn pct(n: usize, p: usize) -> usize {
    n * p / 100
}

pub fn chronological_splits(n: usize) -> Splits {
    let train = Slice {
        start: 0,
        end: pct(n, 60),
    };
    let validation = Slice {
        start: pct(n, 60),
        end: pct(n, 80),
    };
    let holdout = Slice {
        start: pct(n, 80),
        end: n,
    };
    let folds = vec![
        Fold {
            train: Slice {
                start: 0,
                end: pct(n, 40),
            },
            val: Slice {
                start: pct(n, 40),
                end: pct(n, 55),
            },
        },
        Fold {
            train: Slice {
                start: 0,
                end: pct(n, 55),
            },
            val: Slice {
                start: pct(n, 55),
                end: pct(n, 70),
            },
        },
        Fold {
            train: Slice {
                start: 0,
                end: pct(n, 70),
            },
            val: Slice {
                start: pct(n, 70),
                end: pct(n, 80),
            },
        },
    ];
    Splits {
        train,
        validation,
        holdout,
        folds,
    }
}

#[derive(Debug, Clone)]
pub struct PartitionSlice {
    pub candles: Vec<Candle>,
    /// Index in `candles` where the partition begins **after** the warm-up prefix.
    /// Always equal to `window` for partitions with a full warm-up; `0` only when the
    /// dataset itself starts at the partition (train) and there is no earlier data.
    pub partition_start: usize,
    pub partition_end: usize,
}

/// Returns `window` warm-up candles followed by the partition candles.
pub fn slice_with_warmup(full: &[Candle], s: Slice, window: usize) -> PartitionSlice {
    let warm_start = s.start.saturating_sub(window);
    let candles = full[warm_start..s.end].to_vec();
    let partition_start = s.start - warm_start;
    let partition_end = candles.len();
    PartitionSlice {
        candles,
        partition_start,
        partition_end,
    }
}
