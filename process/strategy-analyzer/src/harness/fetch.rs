/// Plan time-window requests that page KuCoin's 1500-record cap.
pub fn page_windows(start_s: i64, end_s: i64, interval_s: i64) -> Vec<(i64, i64)> {
    let step = 1500 * interval_s;
    let mut out = Vec::new();
    let mut s = start_s;
    while s < end_s {
        let e = (s + step).min(end_s);
        out.push((s, e));
        s = e;
    }
    out
}

/// True when a `fetch_page` error reports an HTTP 429 / rate-limit response (drives backoff).
pub fn is_rate_limited(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    err.contains("429") || lower.contains("rate limit") || lower.contains("too many")
}

/// True when the second-resolution window `[start_s, end_s]` overlaps the OOS window
/// `[boundary_ms, ∞)`. The OOS window is inclusive of the boundary timestamp.
pub fn overlaps_oos(start_s: i64, end_s: i64, boundary_ms: i64) -> bool {
    start_s.saturating_mul(1000) >= boundary_ms || end_s.saturating_mul(1000) >= boundary_ms
}

/// `fetch-data` guard (§10.4): until the freeze lock exists, refuse any fetch range that overlaps
/// the OOS window, so the confirmatory data cannot be pulled before the bar is frozen.
pub fn guard_oos_range(
    start_s: i64,
    end_s: i64,
    boundary_ms: i64,
    lock_exists: bool,
) -> Result<(), String> {
    if !lock_exists && overlaps_oos(start_s, end_s, boundary_ms) {
        return Err(format!(
            "fetch-data: range [{start_s}, {end_s}] overlaps the OOS window (boundary {boundary_ms} ms); the freeze lock must exist first"
        ));
    }
    Ok(())
}

/// Fetch one page from KuCoin and return raw CSV rows `time,open,close,high,low,volume,turnover`.
/// Throttle between calls in the caller. Keep I/O out of the core.
pub fn fetch_page(
    base_url: &str,
    symbol: &str,
    interval: &str,
    start_s: i64,
    end_s: i64,
) -> Result<Vec<[String; 7]>, String> {
    let url = format!("{base_url}/api/v1/market/candles?symbol={symbol}&type={interval}&startAt={start_s}&endAt={end_s}");
    let body: serde_json::Value = ureq::get(&url)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("HTTP {code}"),
            other => other.to_string(),
        })?
        .into_json()
        .map_err(|e| e.to_string())?;
    if body.get("code").and_then(|c| c.as_str()) != Some("200000") {
        return Err(format!(
            "KuCoin error: {}",
            body.get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
        ));
    }
    let rows = body
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("no data")?;
    Ok(rows
        .iter()
        .map(|r| {
            let a = r.as_array().expect("row is an array");
            std::array::from_fn(|i| {
                a.get(i)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_respect_1500_cap() {
        let p = page_windows(0, 3000 * 60, 60);
        assert_eq!(p, vec![(0, 1500 * 60), (1500 * 60, 3000 * 60)]);
    }

    #[test]
    fn oos_range_guard_binds_until_the_lock_exists() {
        let boundary_ms = 1_000_000_000_000;
        let boundary_s = boundary_ms / 1000;
        // A range entirely before the boundary is always allowed.
        assert!(guard_oos_range(0, boundary_s - 1, boundary_ms, false).is_ok());
        // A range touching/after the boundary is refused while no lock exists...
        assert!(guard_oos_range(0, boundary_s, boundary_ms, false).is_err());
        assert!(guard_oos_range(boundary_s, boundary_s + 100, boundary_ms, false).is_err());
        // ...but permitted once the lock exists.
        assert!(guard_oos_range(boundary_s, boundary_s + 100, boundary_ms, true).is_ok());
    }

    #[test]
    fn overlaps_oos_is_inclusive_of_the_boundary() {
        let boundary_ms = 1_000_000_000_000;
        let boundary_s = boundary_ms / 1000;
        assert!(!overlaps_oos(0, boundary_s - 1, boundary_ms));
        assert!(overlaps_oos(0, boundary_s, boundary_ms));
        assert!(overlaps_oos(boundary_s, boundary_s + 1, boundary_ms));
    }
}
