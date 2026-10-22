use crate::exchange::kucoin::fetch_klines;
use crate::exchange::timeframes::Timeframe;
use crate::harness::dataset::{hash_candles, load_csv};
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

pub const PAGE: usize = 1500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProvenance {
    pub symbol: String,
    pub timeframe: String,
    pub requested_first_ms: i64,
    pub requested_last_ms: i64,
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
    /// Timeframe-aligned slots in the requested inclusive range
    /// `[requested_first_ms, requested_last_ms]`.
    pub requested_candles: usize,
    /// Timeframe-aligned slots spanned by the effective range of the returned
    /// candles (first through last open time), including gaps.
    pub expected_candles: usize,
    /// Gaps inside the effective range. Assumes the candles are sorted by
    /// `open_time_ms` and aligned to `timeframe`.
    pub missing_candles: usize,
    /// Overlapping duplicates removed before range and closed-candle filtering.
    pub duplicates_dropped: usize,
    pub pages: usize,
    pub content_sha256: String,
}

pub fn dedup_and_sort(mut candles: Vec<Candle>) -> Vec<Candle> {
    candles.sort_by_key(|candle| candle.open_time_ms);
    candles.dedup_by_key(|candle| candle.open_time_ms);
    candles
}

/// Retains only candles inside the requested inclusive range.
pub fn retain_range(candles: &mut Vec<Candle>, start_ms: i64, end_ms: i64) {
    candles.retain(|candle| candle.open_time_ms >= start_ms && candle.open_time_ms <= end_ms);
}

/// Returns whether `provenance` describes exactly the requested cache range.
pub fn cache_matches(
    provenance: &DatasetProvenance,
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    end_ms: i64,
) -> bool {
    provenance.symbol == symbol
        && provenance.timeframe == timeframe.as_kucoin()
        && provenance.requested_first_ms == start_ms
        && provenance.requested_last_ms == end_ms
}

/// Counts the gaps between consecutive candles for `tf`.
///
/// Assumes the candles are sorted by `open_time_ms` and aligned to `tf`;
/// unsorted or misaligned input is not detected here.
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

/// Pages backward from `end_ms` until `start_ms` is covered, fetching batches
/// through `fetch`. Returns the retained candles, the number of pages fetched,
/// and the number of duplicates removed before filtering. Closed candles only.
pub fn fetch_range_with<F>(
    tf: Timeframe,
    start_ms: i64,
    end_ms: i64,
    settle_ms: i64,
    now_ms: i64,
    mut fetch: F,
) -> Result<(Vec<Candle>, usize, usize), String>
where
    F: FnMut(i64) -> Result<Vec<Candle>, String>,
{
    if start_ms > end_ms {
        return Err(format!(
            "invalid range: start_ms {start_ms} is after end_ms {end_ms}"
        ));
    }
    // A paginating source that never reaches `start_ms` must not loop forever.
    let max_pages = ((end_ms - start_ms) / tf.millis() / PAGE as i64 + 4) as usize;
    let mut all: Vec<Candle> = Vec::new();
    let mut cursor = end_ms;
    let mut pages = 0usize;
    loop {
        let batch = fetch(cursor)?;
        if batch.is_empty() {
            break;
        }
        pages += 1;
        if pages > max_pages {
            return Err(format!(
                "pagination exceeded {max_pages} pages for range {start_ms}..={end_ms}"
            ));
        }
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
    let raw = all.len();
    let mut clean = dedup_and_sort(all);
    let duplicates = raw - clean.len();
    clean.retain(|candle| {
        candle.open_time_ms >= start_ms
            && candle.open_time_ms <= end_ms
            && tf.is_closed(candle.open_time_ms, now_ms, settle_ms)
    });
    Ok((clean, pages, duplicates))
}

/// Pages backward from `end_ms` until `start_ms` is covered using the KuCoin
/// client. Closed candles only.
pub fn fetch_range(
    base_url: &str,
    symbol: &str,
    tf: Timeframe,
    start_ms: i64,
    end_ms: i64,
    settle_ms: i64,
    now_ms: i64,
) -> Result<(Vec<Candle>, usize, usize), String> {
    fetch_range_with(tf, start_ms, end_ms, settle_ms, now_ms, |cursor| {
        fetch_klines(base_url, symbol, tf, PAGE, Some(cursor)).map_err(|error| error.to_string())
    })
}

pub fn write_cache(path: &Path, candles: &[Candle]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    // Write the whole cache before replacing the destination so a partial write
    // can never leave a truncated cache behind.
    let temp_path = path.with_extension(format!("csv.{}.tmp", std::process::id()));
    write_cache_file(&temp_path, candles)?;
    std::fs::rename(&temp_path, path).map_err(|error| error.to_string())
}

fn write_cache_file(path: &Path, candles: &[Candle]) -> Result<(), String> {
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

fn provenance_path(csv_path: &Path) -> std::path::PathBuf {
    csv_path.with_extension("csv.provenance.json")
}

/// Reads the provenance JSON stored next to a cache CSV.
pub fn read_provenance(csv_path: &Path) -> Result<DatasetProvenance, String> {
    let text =
        std::fs::read_to_string(provenance_path(csv_path)).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

/// Writes the provenance next to the cache as pretty JSON.
pub fn write_provenance(csv_path: &Path, provenance: &DatasetProvenance) -> Result<(), String> {
    let json = serde_json::to_string_pretty(provenance).map_err(|error| error.to_string())?;
    let path = provenance_path(csv_path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, json).map_err(|error| error.to_string())
}

/// Writes `candles` to a temporary CSV, validates it through [`load_csv`], and
/// only renames it into `path` when validation succeeds. A failed validation
/// removes the temporary file so a bad fetch can never poison the cache.
#[allow(clippy::too_many_arguments)]
fn commit_cache(
    path: &Path,
    candles: &[Candle],
    symbol: &str,
    timeframe: Timeframe,
    as_of_ms: i64,
    settle_ms: i64,
    window: usize,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temp_path = path.with_extension(format!("csv.{}.tmp", std::process::id()));
    if let Err(error) = write_cache_file(&temp_path, candles) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }
    if let Err(error) = load_csv(
        symbol,
        &temp_path,
        timeframe,
        as_of_ms,
        settle_ms,
        0,
        window,
        candles.len(),
    ) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error.to_string());
    }
    if let Err(error) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error.to_string());
    }
    Ok(())
}

/// Candles and provenance returned by [`load_with_cache`].
#[derive(Debug, Clone)]
pub struct CachedCandles {
    pub candles: Vec<Candle>,
    pub provenance: DatasetProvenance,
    /// True when the cache was reused, false when the fetcher ran.
    pub used_cache: bool,
}

/// Reuses a cache only when its provenance matches the requested range,
/// symbol, and timeframe; otherwise calls `fetch` and commits the result.
///
/// Both paths retain only candles inside `[start_ms, end_ms]` before returning.
/// Fetched data is validated through [`load_csv`] before it replaces the cache.
#[allow(clippy::too_many_arguments)]
pub fn load_with_cache<F>(
    cache_path: &Path,
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    end_ms: i64,
    refresh: bool,
    settle_ms: i64,
    window: usize,
    mut fetch: F,
) -> Result<CachedCandles, String>
where
    F: FnMut() -> Result<(Vec<Candle>, usize, usize), String>,
{
    if !refresh && cache_path.is_file() {
        if let Ok(provenance) = read_provenance(cache_path) {
            if cache_matches(&provenance, symbol, timeframe, start_ms, end_ms) {
                let mut candles = read_cache(cache_path)?;
                retain_range(&mut candles, start_ms, end_ms);
                return Ok(CachedCandles {
                    candles,
                    provenance,
                    used_cache: true,
                });
            }
        }
    }

    let (mut candles, pages, duplicates) = fetch()?;
    retain_range(&mut candles, start_ms, end_ms);
    let data_provenance = provenance(
        symbol, timeframe, start_ms, end_ms, duplicates, &candles, pages,
    );
    commit_cache(
        cache_path, &candles, symbol, timeframe, end_ms, settle_ms, window,
    )?;
    write_provenance(cache_path, &data_provenance)?;
    Ok(CachedCandles {
        candles,
        provenance: data_provenance,
        used_cache: false,
    })
}

/// `duplicates` is the count removed by [`dedup_and_sort`] before range and
/// closed-candle filtering; it cannot be inferred from the retained candles.
pub fn provenance(
    symbol: &str,
    tf: Timeframe,
    start_ms: i64,
    end_ms: i64,
    duplicates: usize,
    candles: &[Candle],
    pages: usize,
) -> DatasetProvenance {
    let missing = missing_candles(candles, tf);
    let requested_candles = ((end_ms - start_ms).max(0) / tf.millis() + 1) as usize;
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
        requested_candles,
        expected_candles: candles.len() + missing,
        missing_candles: missing,
        duplicates_dropped: duplicates,
        pages,
        content_sha256: hash_candles(candles),
    }
}
