use crate::exchange::kucoin::{fetch_klines, ExchangeError};
use crate::exchange::timeframes::Timeframe;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("csv error: {0}")]
    Csv(String),
    #[error("exchange error: {0}")]
    Exchange(#[from] ExchangeError),
    #[error("invalid OHLC at {open_time_ms}: {why}")]
    InvalidOhlc { open_time_ms: i64, why: String },
    #[error("out-of-order candle at {got_ms} after {prev_ms}")]
    OutOfOrder { got_ms: i64, prev_ms: i64 },
    #[error("duplicate candle at {0}")]
    Duplicate(i64),
    #[error("timeframe mismatch: spacing {spacing_ms} != {interval_ms}")]
    TimeframeMismatch { spacing_ms: i64, interval_ms: i64 },
    #[error("gap too large at {open_time_ms}: {missing} missing candles")]
    GapTooLarge { open_time_ms: i64, missing: i64 },
    #[error("only {got} closed candles (need >= {need})")]
    TooShort { got: usize, need: usize },
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestedRange {
    pub limit: usize,
    pub timeframe: String,
    pub as_of_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveRange {
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub source: String,
    pub requested: RequestedRange,
    pub effective_range: EffectiveRange,
    pub content_sha256: String,
    pub fetched_at_ms: Option<i64>,
    pub gaps: usize,
}

#[derive(Debug, Clone)]
pub struct Dataset {
    pub symbol: String,
    pub candles: Vec<Candle>,
    pub provenance: Provenance,
}

pub fn load_csv(
    symbol: &str,
    path: &Path,
    timeframe: Timeframe,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    requested_limit: usize,
) -> Result<Dataset, DatasetError> {
    let mut reader =
        csv::Reader::from_path(path).map_err(|error| DatasetError::Csv(error.to_string()))?;
    let mut candles = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|error| DatasetError::Csv(error.to_string()))?;
        let field = |index: usize| -> Result<Decimal, DatasetError> {
            Decimal::from_str(row.get(index).unwrap_or_default())
                .map_err(|error| DatasetError::Csv(format!("column {index}: {error}")))
        };
        candles.push(Candle {
            open_time_ms: row
                .get(0)
                .unwrap_or_default()
                .parse()
                .map_err(|error| DatasetError::Csv(format!("timestamp: {error}")))?,
            open: field(1)?,
            high: field(2)?,
            low: field(3)?,
            close: field(4)?,
            volume: field(5)?,
        });
    }
    validate(
        symbol,
        candles,
        timeframe,
        as_of_ms,
        settle_ms,
        max_gap_candles,
        format!("csv:{}", path.display()),
        None,
        requested_limit,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn fetch_dataset(
    symbol: &str,
    timeframe: Timeframe,
    limit: usize,
    window: usize,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    base_url: &str,
) -> Result<Dataset, DatasetError> {
    let candles = fetch_klines(base_url, symbol, timeframe, window + limit)?;
    validate(
        symbol,
        candles,
        timeframe,
        as_of_ms,
        settle_ms,
        max_gap_candles,
        "kucoin".into(),
        Some(crate::identity::now_ms()),
        limit,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate(
    symbol: &str,
    mut candles: Vec<Candle>,
    timeframe: Timeframe,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    source: String,
    fetched_at_ms: Option<i64>,
    requested_limit: usize,
) -> Result<Dataset, DatasetError> {
    candles.sort_by_key(|candle| candle.open_time_ms);
    for candle in &candles {
        if candle.open <= Decimal::ZERO
            || candle.high <= Decimal::ZERO
            || candle.low <= Decimal::ZERO
            || candle.close <= Decimal::ZERO
        {
            return Err(DatasetError::InvalidOhlc {
                open_time_ms: candle.open_time_ms,
                why: "non-positive price".into(),
            });
        }
        if candle.high < candle.open
            || candle.high < candle.close
            || candle.low > candle.open
            || candle.low > candle.close
            || candle.high < candle.low
        {
            return Err(DatasetError::InvalidOhlc {
                open_time_ms: candle.open_time_ms,
                why: "high/low do not bound open/close".into(),
            });
        }
    }
    let mut gaps = 0usize;
    for pair in candles.windows(2) {
        let prev = &pair[0];
        let next = &pair[1];
        if next.open_time_ms == prev.open_time_ms {
            return Err(DatasetError::Duplicate(next.open_time_ms));
        }
        if next.open_time_ms < prev.open_time_ms {
            return Err(DatasetError::OutOfOrder {
                got_ms: next.open_time_ms,
                prev_ms: prev.open_time_ms,
            });
        }
        let spacing = next.open_time_ms - prev.open_time_ms;
        if spacing % timeframe.millis() != 0 {
            return Err(DatasetError::TimeframeMismatch {
                spacing_ms: spacing,
                interval_ms: timeframe.millis(),
            });
        }
        let missing = spacing / timeframe.millis() - 1;
        if missing > 0 {
            gaps += missing as usize;
            if max_gap_candles > 0 && missing > max_gap_candles {
                return Err(DatasetError::GapTooLarge {
                    open_time_ms: next.open_time_ms,
                    missing,
                });
            }
        }
    }
    candles.retain(|candle| timeframe.is_closed(candle.open_time_ms, as_of_ms, settle_ms));
    if candles.len() < 2 {
        return Err(DatasetError::TooShort {
            got: candles.len(),
            need: 2,
        });
    }
    let content_sha256 = hash_candles(&candles);
    let provenance = Provenance {
        source,
        requested: RequestedRange {
            limit: requested_limit,
            timeframe: timeframe.as_kucoin().to_string(),
            as_of_ms,
        },
        effective_range: EffectiveRange {
            first_open_time_ms: candles[0].open_time_ms,
            last_open_time_ms: candles[candles.len() - 1].open_time_ms,
            candles: candles.len(),
        },
        content_sha256,
        fetched_at_ms,
        gaps,
    };
    Ok(Dataset {
        symbol: symbol.to_string(),
        candles,
        provenance,
    })
}

fn hash_candles(candles: &[Candle]) -> String {
    let mut hasher = Sha256::new();
    for candle in candles {
        hasher.update(candle.open_time_ms.to_be_bytes());
        hasher.update(candle.open.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.high.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.low.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.close.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.volume.to_string().as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut text = String::new();
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
