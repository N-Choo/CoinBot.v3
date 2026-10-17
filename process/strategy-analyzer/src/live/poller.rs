//! Market-data pollers (live spec §3, §3a, §5).
//!
//! The network boundary is the small blocking [`MarketData`] trait: the async loops call it inside
//! `tokio::task::spawn_blocking`, so HTTP stays on the existing blocking `ureq` adapter and tests
//! inject a fake. Two pollers sit on top:
//! - [`SharedTickerPoller`] runs **one** `allTickers` poll and fans the snapshot out to every loop.
//! - [`CandlePoller`] fetches a symbol's 4H candles and emits only **closed** ones, in order, with an
//!   in-order backfill for holes.

use crate::live::clock::Clock;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tokio::time::Instant as TokioInstant;

/// Candles requested when there is no cursor yet (cold re-warm), comfortably above
/// `strategy::types::WINDOW_CANDLES` (64).
pub const CANDLE_LOOKBACK: u32 = 80;

/// A structured error from the network boundary. Distinguishes a **retryable** HTTP status from
/// everything else so the retry wrapper can back off without hammering KuCoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketError {
    /// A retryable HTTP status (`429` or any `5xx`), optionally carrying the server's
    /// `Retry-After` delay in whole seconds.
    Status {
        code: u16,
        retry_after_s: Option<u64>,
    },
    /// A non-retryable failure: transport, JSON, a KuCoin error `code`, or a non-retryable status.
    Fatal(String),
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketError::Status { code, .. } => write!(f, "HTTP {code}"),
            MarketError::Fatal(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for MarketError {}

impl From<String> for MarketError {
    fn from(msg: String) -> Self {
        MarketError::Fatal(msg)
    }
}

/// Classify an HTTP status: `429` and `5xx` are retryable, everything else is fatal. `Retry-After`
/// is carried through only for the retryable statuses.
fn http_error(code: u16, retry_after_s: Option<u64>) -> MarketError {
    if code == 429 || (500..=599).contains(&code) {
        MarketError::Status {
            code,
            retry_after_s,
        }
    } else {
        MarketError::Fatal(format!("HTTP {code}"))
    }
}

/// Parse a `Retry-After` header value: whole seconds only (an HTTP-date falls back to exponential
/// backoff, per live spec §8).
fn parse_retry_after(raw: &str) -> Option<u64> {
    raw.trim().parse().ok()
}

/// Bounded retry/backoff policy for the network boundary (live spec §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Retries **after** the first attempt (total attempts = `max_retries + 1`).
    pub max_retries: u32,
    /// Exponential-backoff base used when no `Retry-After` is present.
    pub retry_base_ms: u64,
    /// Cap on any single backoff sleep.
    pub retry_max_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, retry_base_ms: u64, retry_max_ms: u64) -> Self {
        RetryPolicy {
            max_retries,
            retry_base_ms,
            retry_max_ms,
        }
    }

    /// The delay before retry `retry_index` (0-based). Prefers the server's `Retry-After`; otherwise
    /// `retry_base_ms * 2^retry_index`, capped at `retry_max_ms`.
    pub fn backoff(&self, retry_index: u32, retry_after_s: Option<u64>) -> Duration {
        if let Some(secs) = retry_after_s {
            return Duration::from_secs(secs);
        }
        let factor = 1u64.checked_shl(retry_index).unwrap_or(u64::MAX);
        Duration::from_millis(
            self.retry_base_ms
                .saturating_mul(factor)
                .min(self.retry_max_ms),
        )
    }
}

/// Drive `attempt` under `policy`, sleeping between retryable [`MarketError::Status`] failures.
///
/// - Success or a [`MarketError::Fatal`] returns immediately.
/// - A retryable status sleeps for [`RetryPolicy::backoff`] and retries, up to `max_retries` times.
/// - After exhaustion the last status error is returned (the caller keeps WARN-and-continue).
pub async fn with_retry<T, F, Fut>(policy: RetryPolicy, mut attempt: F) -> Result<T, MarketError>
where
    T: Send,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, MarketError>>,
{
    let mut retries = 0u32;
    loop {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(MarketError::Fatal(msg)) => return Err(MarketError::Fatal(msg)),
            Err(
                err @ MarketError::Status {
                    code,
                    retry_after_s,
                },
            ) => {
                if retries >= policy.max_retries {
                    return Err(err);
                }
                let delay = policy.backoff(retries, retry_after_s);
                log::warn!(
                    "KuCoin HTTP {code}: retry {}/{max} in {delay:?}",
                    retries + 1,
                    max = policy.max_retries,
                );
                tokio::time::sleep(delay).await;
                retries += 1;
            }
        }
    }
}

/// A tiny non-cryptographic PRNG (xorshift64*) for pacer jitter. The live poller is **not** the
/// deterministic core, so randomness here is fine (live spec §8).
#[derive(Debug)]
struct PacerState {
    next_allowed: TokioInstant,
    rng: u64,
}

/// A signed jitter offset in `[-jitter, +jitter]` nanoseconds, driven by the pacer PRNG.
fn jitter_offset(rng: &mut u64, jitter: Duration) -> i64 {
    let span = jitter.as_nanos() as u64;
    if span == 0 {
        return 0;
    }
    // xorshift64* — good enough to spread a burst, no dependency.
    let mut x = *rng;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *rng = x;
    let rand = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
    let mag = rand % (2 * span + 1); // 0..=2*span
    mag as i64 - span as i64
}

fn seed_rng() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1
}

/// A global request pacer shared by every network call (live spec §8 "shared limiter"). It serializes
/// requests so the adapter as a whole respects the Public pool budget, with a bounded per-acquire
/// jitter to spread the 4H boundary burst.
pub struct RateLimiter {
    min_interval: Duration,
    jitter: Duration,
    state: AsyncMutex<PacerState>,
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self::with_jitter(min_interval, 0)
    }

    /// A pacer whose spacing is `min_interval +/- jitter_pct%` (clamped to `0..=100`).
    pub fn with_jitter(min_interval: Duration, jitter_pct: u32) -> Self {
        let jitter = min_interval * jitter_pct.min(100) / 100;
        RateLimiter {
            min_interval,
            jitter,
            state: AsyncMutex::new(PacerState {
                next_allowed: TokioInstant::now(),
                rng: seed_rng(),
            }),
        }
    }

    /// Wait until the next request slot, then reserve the following one (with jitter).
    pub async fn acquire(&self) {
        let mut st = self.state.lock().await;
        let now = TokioInstant::now();
        if st.next_allowed > now {
            tokio::time::sleep_until(st.next_allowed).await;
        }
        let now = TokioInstant::now();
        let base = now + self.min_interval;
        let offset = jitter_offset(&mut st.rng, self.jitter);
        st.next_allowed = if offset >= 0 {
            base + Duration::from_nanos(offset as u64)
        } else {
            base - Duration::from_nanos(offset.unsigned_abs())
        };
    }
}

/// The network boundary for the live adapters. Blocking by design; callers drive it through
/// [`with_retry`] inside `tokio::task::spawn_blocking`.
pub trait MarketData: Send + Sync + 'static {
    /// `GET /api/v1/timestamp` — exchange time in ms.
    fn exchange_time_ms(&self) -> Result<i64, MarketError>;
    /// `GET /api/v1/market/allTickers` — symbol → last price.
    fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, MarketError>;
    /// `GET /api/v1/market/candles` — OHLCV rows (ascending is not required; callers sort).
    fn candles(
        &self,
        symbol: &str,
        interval: &str,
        start_s: i64,
        end_s: i64,
    ) -> Result<Vec<Candle>, MarketError>;
}

/// The `ureq` (blocking HTTP) implementation of [`MarketData`].
pub struct UreqMarketData {
    base_url: String,
}

impl UreqMarketData {
    pub fn new(base_url: impl Into<String>) -> Self {
        UreqMarketData {
            base_url: base_url.into(),
        }
    }

    fn get(&self, path: &str) -> Result<serde_json::Value, MarketError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let response = ureq::get(&url).call().map_err(|e| match e {
            ureq::Error::Status(code, response) => {
                // `Retry-After` lets KuCoin dictate the pause; parsed as seconds only.
                let retry_after_s = response.header("Retry-After").and_then(parse_retry_after);
                http_error(code, retry_after_s)
            }
            other => MarketError::Fatal(other.to_string()),
        })?;
        response
            .into_json()
            .map_err(|e| MarketError::Fatal(e.to_string()))
    }
}

impl MarketData for UreqMarketData {
    fn exchange_time_ms(&self) -> Result<i64, MarketError> {
        parse_timestamp(&self.get("/api/v1/timestamp")?).map_err(MarketError::Fatal)
    }

    fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, MarketError> {
        parse_all_tickers(&self.get("/api/v1/market/allTickers")?).map_err(MarketError::Fatal)
    }

    fn candles(
        &self,
        symbol: &str,
        interval: &str,
        start_s: i64,
        end_s: i64,
    ) -> Result<Vec<Candle>, MarketError> {
        let path = format!(
            "/api/v1/market/candles?symbol={symbol}&type={interval}&startAt={start_s}&endAt={end_s}"
        );
        parse_candles(&self.get(&path)?).map_err(MarketError::Fatal)
    }
}

/// Reject a KuCoin body whose `code` is not `200000`.
fn check_code(body: &serde_json::Value) -> Result<(), String> {
    match body.get("code").and_then(|c| c.as_str()) {
        Some("200000") => Ok(()),
        other => Err(format!(
            "KuCoin error: {} ({other:?})",
            body.get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
        )),
    }
}

/// Parse `/api/v1/timestamp`.
pub fn parse_timestamp(body: &serde_json::Value) -> Result<i64, String> {
    check_code(body)?;
    body.get("data")
        .and_then(|d| d.as_i64())
        .ok_or_else(|| "timestamp: missing data".to_string())
}

/// Parse `/api/v1/market/allTickers` into symbol → last price.
pub fn parse_all_tickers(body: &serde_json::Value) -> Result<BTreeMap<String, Decimal>, String> {
    check_code(body)?;
    let rows = body
        .pointer("/data/ticker")
        .and_then(|t| t.as_array())
        .ok_or_else(|| "allTickers: missing data.ticker".to_string())?;
    let mut out = BTreeMap::new();
    for row in rows {
        let symbol = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "allTickers: ticker without symbol".to_string())?;
        let last = row
            .get("last")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("allTickers: {symbol} without last"))?;
        let price = Decimal::from_str(last).map_err(|e| format!("allTickers: {symbol}: {e}"))?;
        out.insert(symbol.to_string(), price);
    }
    Ok(out)
}

/// Parse `/api/v1/market/candles`. KuCoin columns are `[time_s, open, close, high, low, ...]`.
pub fn parse_candles(body: &serde_json::Value) -> Result<Vec<Candle>, String> {
    check_code(body)?;
    let rows = body
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "candles: missing data".to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cols = row
            .as_array()
            .ok_or_else(|| "candles: row is not an array".to_string())?;
        let col = |i: usize| -> Result<&str, String> {
            cols.get(i)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("candles: missing column {i}"))
        };
        let sec: i64 = col(0)?
            .parse()
            .map_err(|e| format!("candles: bad timestamp: {e}"))?;
        let open = Decimal::from_str(col(1)?).map_err(|e| e.to_string())?;
        let close = Decimal::from_str(col(2)?).map_err(|e| e.to_string())?;
        let high = Decimal::from_str(col(3)?).map_err(|e| e.to_string())?;
        let low = Decimal::from_str(col(4)?).map_err(|e| e.to_string())?;
        out.push(Candle {
            open_time_ms: sec
                .checked_mul(1000)
                .ok_or_else(|| "candles: timestamp out of range".to_string())?,
            open,
            high,
            low,
            close,
        });
    }
    Ok(out)
}

/// The candle-close rule (live spec §3a): `open_time + interval + settle <= exchange_now`.
pub fn candle_is_closed(
    open_time_ms: i64,
    exchange_now_ms: i64,
    interval_ms: i64,
    settle_ms: i64,
) -> bool {
    open_time_ms
        .saturating_add(interval_ms)
        .saturating_add(settle_ms)
        <= exchange_now_ms
}

/// Keep only closed candles, in ascending order, de-duplicated by `open_time_ms`.
pub fn select_closed_candles(
    raw: &[Candle],
    exchange_now_ms: i64,
    interval_ms: i64,
    settle_ms: i64,
) -> Vec<Candle> {
    let mut out: Vec<Candle> = raw
        .iter()
        .copied()
        .filter(|c| candle_is_closed(c.open_time_ms, exchange_now_ms, interval_ms, settle_ms))
        .collect();
    out.sort_by_key(|c| c.open_time_ms);
    out.dedup_by_key(|c| c.open_time_ms);
    out
}

/// The last processed candle, used to detect and fill holes in the next fetch.
#[derive(Debug, Clone, Copy, Default)]
pub struct CandleCursor {
    pub last_open_ms: Option<i64>,
    pub last_close: Option<Decimal>,
}

/// Fill gaps before/between `candles` with flat candles at the previous close (the exchange omits
/// intervals with no trades, and the core rejects gaps). A leading gap is only fillable when a
/// `last_close` is known.
pub fn fill_gaps(candles: Vec<Candle>, cursor: CandleCursor, interval_ms: i64) -> Vec<Candle> {
    let mut out: Vec<Candle> = Vec::with_capacity(candles.len());
    for c in candles {
        if let Some(prev) = out.last().copied() {
            let close = prev.close;
            let mut t = prev.open_time_ms + interval_ms;
            while t < c.open_time_ms {
                out.push(flat_candle(t, close));
                t += interval_ms;
            }
        } else if let (Some(a), Some(close)) = (cursor.last_open_ms, cursor.last_close) {
            // Leading gap: fill from the cursor's next interval up to this candle.
            let mut t = a + interval_ms;
            while t < c.open_time_ms {
                out.push(flat_candle(t, close));
                t += interval_ms;
            }
        }
        out.push(c);
    }
    out
}

fn flat_candle(open_time_ms: i64, price: Decimal) -> Candle {
    Candle {
        open_time_ms,
        open: price,
        high: price,
        low: price,
        close: price,
    }
}

/// Per-symbol 4H candle poller. Stateless between calls; the cursor is supplied by the caller so the
/// loop's persisted `StrategyState` remains the single source of position.
#[derive(Clone)]
pub struct CandlePoller {
    interval: String,
    interval_ms: i64,
    settle_ms: i64,
    lookback: u32,
}

impl CandlePoller {
    pub fn new(
        interval: impl Into<String>,
        interval_ms: i64,
        settle_ms: i64,
        lookback: u32,
    ) -> Self {
        CandlePoller {
            interval: interval.into(),
            interval_ms,
            settle_ms,
            lookback,
        }
    }

    /// Fetch and return the closed candles strictly after the cursor, in order, with one in-order
    /// backfill for a leading hole and a forward-fill for any hole the exchange genuinely omits.
    pub fn poll(
        &self,
        market: &dyn MarketData,
        symbol: &str,
        cursor: CandleCursor,
        now_ms: i64,
    ) -> Result<Vec<Candle>, MarketError> {
        let interval_s = (self.interval_ms / 1000).max(1);
        let end_s = now_ms / 1000;
        let start_s = match cursor.last_open_ms {
            Some(a) => a / 1000,
            None => end_s - (self.lookback as i64) * interval_s,
        };
        let raw = market.candles(symbol, &self.interval, start_s, end_s)?;
        let mut closed = select_closed_candles(&raw, now_ms, self.interval_ms, self.settle_ms);
        if let Some(a) = cursor.last_open_ms {
            closed.retain(|c| c.open_time_ms > a);
        }

        // In-order backfill for a leading hole.
        if let (Some(a), Some(first)) = (cursor.last_open_ms, closed.first().copied()) {
            if first.open_time_ms > a + self.interval_ms {
                let bs = (a / 1000) + interval_s;
                let be = first.open_time_ms / 1000;
                if let Ok(back) = market.candles(symbol, &self.interval, bs, be) {
                    let mut merged =
                        select_closed_candles(&back, now_ms, self.interval_ms, self.settle_ms);
                    merged.extend(closed);
                    merged.sort_by_key(|c| c.open_time_ms);
                    merged.dedup_by_key(|c| c.open_time_ms);
                    closed = merged
                        .into_iter()
                        .filter(|c| cursor.last_open_ms.is_none_or(|a| c.open_time_ms > a))
                        .collect();
                }
            }
        }

        Ok(fill_gaps(closed, cursor, self.interval_ms))
    }
}

/// One `allTickers` snapshot: the exchange time it was taken and the last price per symbol.
#[derive(Debug, Clone)]
pub struct TickerSnapshot {
    pub at_ms: i64,
    pub prices: BTreeMap<String, Decimal>,
}

/// The single shared `allTickers` poller (live spec §3). Fans one snapshot per poll out to every
/// symbol loop through a broadcast channel.
pub struct SharedTickerPoller;

impl SharedTickerPoller {
    /// Spawn the poller. Returns the broadcast sender (loop tasks subscribe) and the task handle.
    pub fn spawn(
        market: Arc<dyn MarketData>,
        clock: Arc<dyn Clock>,
        limiter: Arc<RateLimiter>,
        retry: RetryPolicy,
        poll_interval: Duration,
        mut shutdown: watch::Receiver<bool>,
    ) -> (broadcast::Sender<TickerSnapshot>, JoinHandle<()>) {
        let (tx, _rx) = broadcast::channel(64);
        let tx_out = tx.clone();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(poll_interval);
            loop {
                tokio::select! {
                    res = shutdown.changed() => {
                        // Err means the sender was dropped: also a stop.
                        if res.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = ticker.tick() => {
                        limiter.acquire().await;
                        let m = market.clone();
                        let fetched = with_retry(retry, move || {
                            let m = m.clone();
                            async move {
                                tokio::task::spawn_blocking(move || m.all_tickers_last())
                                    .await
                                    .map_err(|e| MarketError::Fatal(format!("allTickers join error: {e}")))?
                            }
                        })
                        .await;
                        match fetched {
                            Ok(prices) => {
                                let snap = TickerSnapshot { at_ms: clock.exchange_now_ms(), prices };
                                let _ = tx.send(snap);
                            }
                            Err(e) => log::warn!("allTickers poll failed: {e}"),
                        }
                    }
                }
            }
        });
        (tx_out, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn candle(open_time_ms: i64) -> Candle {
        Candle {
            open_time_ms,
            open: dec("1.0"),
            high: dec("1.1"),
            low: dec("0.9"),
            close: dec("1.0"),
        }
    }

    /// A scripted fake market for candle-poll tests and the parse tests.
    struct FakeMarket {
        candles: Vec<Candle>,
        backfill: Vec<Candle>,
        calls: std::sync::Mutex<Vec<(i64, i64)>>,
    }

    impl MarketData for FakeMarket {
        fn exchange_time_ms(&self) -> Result<i64, MarketError> {
            Ok(0)
        }
        fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, MarketError> {
            Ok(BTreeMap::new())
        }
        fn candles(
            &self,
            _s: &str,
            _i: &str,
            start_s: i64,
            end_s: i64,
        ) -> Result<Vec<Candle>, MarketError> {
            self.calls.lock().unwrap().push((start_s, end_s));
            // The first call is the window; a backfill window (start_s == cursor) returns `backfill`.
            if self.calls.lock().unwrap().len() > 1 {
                Ok(self.backfill.clone())
            } else {
                Ok(self.candles.clone())
            }
        }
    }

    const H4: i64 = 14_400_000;
    const SETTLE: i64 = 5_000;

    #[test]
    fn candle_close_rule_includes_the_settle_exactly() {
        // open + 4h + settle == now is closed; one ms earlier is not.
        assert!(candle_is_closed(0, H4 + SETTLE, H4, SETTLE));
        assert!(!candle_is_closed(0, H4 + SETTLE - 1, H4, SETTLE));
    }

    #[test]
    fn select_closed_drops_unclosed_and_sorts() {
        let raw = vec![candle(2 * H4), candle(0), candle(H4)];
        // A candle at `open` closes at `open + 4h + settle`; at now = 2*H4 + settle, candles 0 and
        // H4 are closed, but the candle at 2*H4 (which closes at 3*H4 + settle) is not.
        let now = 2 * H4 + SETTLE;
        let closed = select_closed_candles(&raw, now, H4, SETTLE);
        assert_eq!(
            closed.iter().map(|c| c.open_time_ms).collect::<Vec<_>>(),
            vec![0, H4]
        );
        // The *next* candle (2*H4) is not closed at that time.
        assert!(!closed.iter().any(|c| c.open_time_ms == 2 * H4));
    }

    #[test]
    fn poll_returns_only_candles_after_the_cursor() {
        let market = FakeMarket {
            candles: vec![candle(0), candle(H4), candle(2 * H4)],
            backfill: vec![],
            calls: std::sync::Mutex::new(vec![]),
        };
        let poller = CandlePoller::new("4hour", H4, SETTLE, CANDLE_LOOKBACK);
        let cursor = CandleCursor {
            last_open_ms: Some(0),
            last_close: Some(dec("1.0")),
        };
        let now = 3 * H4 + SETTLE;
        let out = poller.poll(&market, "XRP-USDT", cursor, now).unwrap();
        assert_eq!(
            out.iter().map(|c| c.open_time_ms).collect::<Vec<_>>(),
            vec![H4, 2 * H4]
        );
    }

    #[test]
    fn poll_backfills_a_leading_hole() {
        // The window omits H4 (a hole); the backfill supplies it.
        let market = FakeMarket {
            candles: vec![candle(2 * H4)],
            backfill: vec![candle(H4)],
            calls: std::sync::Mutex::new(vec![]),
        };
        let poller = CandlePoller::new("4hour", H4, SETTLE, CANDLE_LOOKBACK);
        let cursor = CandleCursor {
            last_open_ms: Some(0),
            last_close: Some(dec("1.0")),
        };
        let now = 3 * H4 + SETTLE;
        let out = poller.poll(&market, "XRP-USDT", cursor, now).unwrap();
        assert_eq!(
            out.iter().map(|c| c.open_time_ms).collect::<Vec<_>>(),
            vec![H4, 2 * H4],
            "the backfilled candle is restored in order"
        );
    }

    #[test]
    fn poll_forward_fills_an_unfillable_hole() {
        // The window omits H4 and the backfill returns nothing: forward-fill at the cursor's close.
        let market = FakeMarket {
            candles: vec![candle(2 * H4)],
            backfill: vec![],
            calls: std::sync::Mutex::new(vec![]),
        };
        let poller = CandlePoller::new("4hour", H4, SETTLE, CANDLE_LOOKBACK);
        let cursor = CandleCursor {
            last_open_ms: Some(0),
            last_close: Some(dec("1.0")),
        };
        let now = 3 * H4 + SETTLE;
        let out = poller.poll(&market, "XRP-USDT", cursor, now).unwrap();
        assert_eq!(
            out.iter().map(|c| c.open_time_ms).collect::<Vec<_>>(),
            vec![H4, 2 * H4]
        );
        assert_eq!(out[0].close, dec("1.0"), "flat fill at the cursor close");
    }

    #[test]
    fn parse_timestamp_and_tickers_and_candles() {
        let ts: serde_json::Value =
            serde_json::from_str(r#"{"code":"200000","data":1700000000000}"#).unwrap();
        assert_eq!(parse_timestamp(&ts).unwrap(), 1_700_000_000_000);

        let tick: serde_json::Value = serde_json::from_str(
            r#"{"code":"200000","data":{"ticker":[{"symbol":"XRP-USDT","last":"0.5"}]}}"#,
        )
        .unwrap();
        let prices = parse_all_tickers(&tick).unwrap();
        assert_eq!(prices.get("XRP-USDT"), Some(&dec("0.5")));

        let candles: serde_json::Value = serde_json::from_str(
            r#"{"code":"200000","data":[["1700000000","1","2","3","0.5","10","20"]]}"#,
        )
        .unwrap();
        let parsed = parse_candles(&candles).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].open_time_ms, 1_700_000_000_000);
        assert_eq!(parsed[0].open, dec("1"));
        assert_eq!(parsed[0].close, dec("2"));
        assert_eq!(parsed[0].high, dec("3"));
        assert_eq!(parsed[0].low, dec("0.5"));
    }

    #[test]
    fn parse_rejects_a_kucoin_error_code() {
        let bad: serde_json::Value =
            serde_json::from_str(r#"{"code":"400001","msg":"bad symbol"}"#).unwrap();
        assert!(parse_timestamp(&bad).is_err());
        assert!(parse_all_tickers(&bad).is_err());
        assert!(parse_candles(&bad).is_err());
    }

    #[tokio::test]
    async fn rate_limiter_spaces_requests_by_the_minimum_interval() {
        let interval = Duration::from_millis(10);
        let limiter = RateLimiter::new(interval);
        limiter.acquire().await; // the first slot is immediate
        let start = std::time::Instant::now();
        limiter.acquire().await; // the second waits one interval
        assert!(
            start.elapsed() >= interval,
            "second acquire must wait at least the minimum interval"
        );
    }

    #[test]
    fn backoff_prefers_retry_after_then_doubles_to_the_cap() {
        let p = RetryPolicy::new(4, 100, 5_000);
        // A `Retry-After` wins outright.
        assert_eq!(p.backoff(0, Some(2)), Duration::from_secs(2));
        // Otherwise exponential: base, x2, x4, ... capped at `retry_max_ms`.
        assert_eq!(p.backoff(0, None), Duration::from_millis(100));
        assert_eq!(p.backoff(1, None), Duration::from_millis(200));
        assert_eq!(p.backoff(2, None), Duration::from_millis(400));
        assert_eq!(p.backoff(9, None), Duration::from_millis(5_000));
    }

    #[test]
    fn retry_after_header_parses_seconds_but_not_dates() {
        assert_eq!(parse_retry_after("2"), Some(2));
        assert_eq!(parse_retry_after(" 15 "), Some(15));
        // An HTTP-date is unsupported: fall back to exponential backoff.
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
        assert_eq!(parse_retry_after(""), None);
    }

    #[test]
    fn http_status_classification_is_retryable_only_for_429_and_5xx() {
        assert_eq!(
            http_error(429, Some(2)),
            MarketError::Status {
                code: 429,
                retry_after_s: Some(2)
            }
        );
        assert!(matches!(
            http_error(503, None),
            MarketError::Status { code: 503, .. }
        ));
        assert!(matches!(http_error(400, None), MarketError::Fatal(_)));
        assert!(matches!(http_error(404, None), MarketError::Fatal(_)));
    }

    #[tokio::test(start_paused = true)]
    async fn retry_honors_retry_after_then_succeeds() {
        let policy = RetryPolicy::new(3, 100, 1_000);
        let calls = std::sync::atomic::AtomicU32::new(0);
        let start = TokioInstant::now();
        let result = with_retry(policy, || {
            let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(MarketError::Status {
                        code: 429,
                        retry_after_s: Some(2),
                    })
                } else {
                    Ok(42)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(
            TokioInstant::now() - start,
            Duration::from_secs(2),
            "the retry waits the server's Retry-After"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn retry_uses_exponential_backoff_without_retry_after() {
        let policy = RetryPolicy::new(3, 100, 1_000);
        let calls = std::sync::atomic::AtomicU32::new(0);
        let start = TokioInstant::now();
        let result = with_retry(policy, || {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async {
                Err::<i32, _>(MarketError::Status {
                    code: 429,
                    retry_after_s: None,
                })
            }
        })
        .await;
        assert!(matches!(result, Err(MarketError::Status { code: 429, .. })));
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "the initial attempt plus three retries"
        );
        // 100 + 200 + 400 = 700 ms of backoff.
        assert_eq!(TokioInstant::now() - start, Duration::from_millis(700));
    }

    #[tokio::test(start_paused = true)]
    async fn retry_backoff_is_capped_at_retry_max_ms() {
        let policy = RetryPolicy::new(4, 100, 250);
        let start = TokioInstant::now();
        let _ = with_retry(policy, || async {
            Err::<i32, _>(MarketError::Status {
                code: 503,
                retry_after_s: None,
            })
        })
        .await;
        // 100 + 200 + 250 + 250 (the last two both hit the cap).
        assert_eq!(TokioInstant::now() - start, Duration::from_millis(800));
    }

    #[tokio::test(start_paused = true)]
    async fn retry_exhausts_max_retries_and_returns_the_error() {
        let policy = RetryPolicy::new(1, 50, 1_000);
        let calls = std::sync::atomic::AtomicU32::new(0);
        let result = with_retry(policy, || {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async {
                Err::<i32, _>(MarketError::Status {
                    code: 429,
                    retry_after_s: None,
                })
            }
        })
        .await;
        assert!(matches!(result, Err(MarketError::Status { code: 429, .. })));
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "one initial attempt plus one retry"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn retry_does_not_retry_a_fatal_error() {
        let policy = RetryPolicy::new(5, 100, 1_000);
        let calls = std::sync::atomic::AtomicU32::new(0);
        let start = TokioInstant::now();
        let result = with_retry(policy, || {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { Err::<i32, _>(MarketError::Fatal("boom".into())) }
        })
        .await;
        assert!(matches!(result, Err(MarketError::Fatal(_))));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(TokioInstant::now() - start, Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn retry_returns_the_body_after_a_transient_failure() {
        let policy = RetryPolicy::new(1, 50, 1_000);
        let calls = std::sync::atomic::AtomicU32::new(0);
        let result = with_retry(policy, || {
            let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(MarketError::Status {
                        code: 500,
                        retry_after_s: None,
                    })
                } else {
                    Ok(vec![1, 2, 3])
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn jitter_offset_is_bounded_by_the_configured_percentage() {
        let jitter = Duration::from_millis(10);
        let mut rng = 0x1234_5678_9abc_def1u64;
        let mut saw_negative = false;
        let mut saw_positive = false;
        for _ in 0..10_000 {
            let off = jitter_offset(&mut rng, jitter);
            assert!(
                (-10_000_000..=10_000_000).contains(&off),
                "offset {off} outside +/-10 ms"
            );
            saw_negative |= off < 0;
            saw_positive |= off > 0;
        }
        assert!(saw_negative && saw_positive, "jitter must be two-sided");
    }

    #[tokio::test(start_paused = true)]
    async fn pacer_spacing_stays_within_the_jitter_bounds() {
        let interval = Duration::from_millis(50);
        let limiter = RateLimiter::with_jitter(interval, 20);
        limiter.acquire().await; // the first slot is immediate
        for _ in 0..20 {
            let start = TokioInstant::now();
            limiter.acquire().await;
            let elapsed = TokioInstant::now() - start;
            assert!(
                elapsed >= interval * 80 / 100,
                "spacing below the -20% jitter floor: {elapsed:?}"
            );
            assert!(
                elapsed <= interval * 120 / 100,
                "spacing above the +20% jitter ceiling: {elapsed:?}"
            );
        }
    }
}
