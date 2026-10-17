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
}
