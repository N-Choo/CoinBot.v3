use crate::exchange::kucoin::fetch_klines;
use crate::exchange::timeframes::Timeframe;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::str::FromStr;

pub const PAGE: usize = 1500;

#[derive(Debug, Clone, Serialize)]
pub struct DatasetProvenance {
    pub symbol: String,
    pub timeframe: String,
    pub requested_first_ms: i64,
    pub requested_last_ms: i64,
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
    pub expected_candles: usize,
    pub missing_candles: usize,
    pub duplicates_dropped: usize,
    pub pages: usize,
    pub content_sha256: String,
}

pub fn dedup_and_sort(mut candles: Vec<Candle>) -> Vec<Candle> {
    candles.sort_by_key(|candle| candle.open_time_ms);
    candles.dedup_by_key(|candle| candle.open_time_ms);
    candles
}

pub fn missing_candles(candles: &[Candle], tf: Timeframe) -> usize {
    let mut missing = 0usize;
    for pair in candles.windows(2) {
        let spacing = pair[1].open_time_ms - pair[0].open_time_ms;
        let gap = spacing / tf.millis() - 1;
        if gap > 0 {
            missing += gap as usize;
        }
    }
    missing
}

/// Pages backward from `end_ms` until `start_ms` is covered. Closed candles only.
pub fn fetch_range(
    base_url: &str,
    symbol: &str,
    tf: Timeframe,
    start_ms: i64,
    end_ms: i64,
    settle_ms: i64,
    now_ms: i64,
) -> Result<(Vec<Candle>, usize), String> {
    let mut all: Vec<Candle> = Vec::new();
    let mut cursor = end_ms;
    let mut pages = 0usize;
    loop {
        let batch =
            fetch_klines(base_url, symbol, tf, PAGE, Some(cursor)).map_err(|e| e.to_string())?;
        if batch.is_empty() {
            break;
        }
        pages += 1;
        let earliest = batch
            .iter()
            .map(|candle| candle.open_time_ms)
            .min()
            .unwrap_or(end_ms);
        all.extend(batch);
        if earliest <= start_ms {
            break;
        }
        cursor = earliest - tf.millis();
    }
    let mut clean = dedup_and_sort(all);
    clean.retain(|candle| {
        candle.open_time_ms >= start_ms
            && candle.open_time_ms <= end_ms
            && tf.is_closed(candle.open_time_ms, now_ms, settle_ms)
    });
    Ok((clean, pages))
}

pub fn write_cache(path: &Path, candles: &[Candle]) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record(["open_time_ms", "open", "high", "low", "close", "volume"])
        .map_err(|error| error.to_string())?;
    for candle in candles {
        writer
            .write_record([
                candle.open_time_ms.to_string(),
                candle.open.to_string(),
                candle.high.to_string(),
                candle.low.to_string(),
                candle.close.to_string(),
                candle.volume.to_string(),
            ])
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn read_cache(path: &Path) -> Result<Vec<Candle>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|error| error.to_string())?;
    let headers = reader.headers().map_err(|error| error.to_string())?.clone();
    let index = |name: &str| -> Result<usize, String> {
        headers
            .iter()
            .position(|header| {
                header
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .eq_ignore_ascii_case(name)
            })
            .ok_or_else(|| format!("missing column: {name}"))
    };
    let open_time_index = index("open_time_ms")?;
    let open_index = index("open")?;
    let high_index = index("high")?;
    let low_index = index("low")?;
    let close_index = index("close")?;
    let volume_index = index("volume")?;
    let mut candles = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|error| error.to_string())?;
        let field = |index: usize| -> Result<Decimal, String> {
            Decimal::from_str(row.get(index).unwrap_or_default()).map_err(|error| error.to_string())
        };
        candles.push(Candle {
            open_time_ms: row
                .get(open_time_index)
                .unwrap_or_default()
                .parse()
                .map_err(|error: std::num::ParseIntError| error.to_string())?,
            open: field(open_index)?,
            high: field(high_index)?,
            low: field(low_index)?,
            close: field(close_index)?,
            volume: field(volume_index)?,
        });
    }
    Ok(candles)
}

pub fn provenance(
    symbol: &str,
    tf: Timeframe,
    start_ms: i64,
    end_ms: i64,
    raw_count: usize,
    candles: &[Candle],
    pages: usize,
) -> DatasetProvenance {
    let missing = missing_candles(candles, tf);
    DatasetProvenance {
        symbol: symbol.to_string(),
        timeframe: tf.as_kucoin().to_string(),
        requested_first_ms: start_ms,
        requested_last_ms: end_ms,
        first_open_time_ms: candles
            .first()
            .map(|candle| candle.open_time_ms)
            .unwrap_or(0),
        last_open_time_ms: candles
            .last()
            .map(|candle| candle.open_time_ms)
            .unwrap_or(0),
        candles: candles.len(),
        expected_candles: candles.len() + missing,
        missing_candles: missing,
        duplicates_dropped: raw_count.saturating_sub(candles.len()),
        pages,
        content_sha256: hash_candles(candles),
    }
}

fn hash_candles(candles: &[Candle]) -> String {
    let mut hasher = Sha256::new();
    for candle in candles {
        hasher.update(candle.open_time_ms.to_be_bytes());
        hasher.update(candle.open.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.high.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.low.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.close.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.volume.normalize().to_string().as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut text = String::new();
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
