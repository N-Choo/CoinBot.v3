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

/// Minimum spacing between outbound requests: the KuCoin Public pool is 2000/30s (VIP0, ~66.7
/// req/s), so a global 15 ms spacing keeps the adapter well inside budget (live spec §8).
pub const RATE_LIMIT_MIN_INTERVAL: Duration = Duration::from_millis(15);

/// A global request pacer shared by every network call (live spec §8 "shared limiter"). It serializes
/// requests so the adapter as a whole respects the Public pool budget.
pub struct RateLimiter {
    min_interval: Duration,
    next_allowed: AsyncMutex<TokioInstant>,
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        RateLimiter {
            min_interval,
            next_allowed: AsyncMutex::new(TokioInstant::now()),
        }
    }

    /// Wait until the next request slot, then reserve the following one.
    pub async fn acquire(&self) {
        let mut next = self.next_allowed.lock().await;
        let now = TokioInstant::now();
        if *next > now {
            tokio::time::sleep_until(*next).await;
        }
        *next = TokioInstant::now() + self.min_interval;
    }
}

/// The network boundary for the live adapters. Blocking by design; callers use `spawn_blocking`.
pub trait MarketData: Send + Sync + 'static {
    /// `GET /api/v1/timestamp` — exchange time in ms.
    fn exchange_time_ms(&self) -> Result<i64, String>;
    /// `GET /api/v1/market/allTickers` — symbol → last price.
    fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, String>;
    /// `GET /api/v1/market/candles` — OHLCV rows (ascending is not required; callers sort).
    fn candles(
        &self,
        symbol: &str,
        interval: &str,
        start_s: i64,
        end_s: i64,
    ) -> Result<Vec<Candle>, String>;
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

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        ureq::get(&url)
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(code, _) => format!("HTTP {code}"),
                other => other.to_string(),
            })?
            .into_json()
            .map_err(|e| e.to_string())
    }
}

impl MarketData for UreqMarketData {
    fn exchange_time_ms(&self) -> Result<i64, String> {
        parse_timestamp(&self.get("/api/v1/timestamp")?)
    }

    fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, String> {
        parse_all_tickers(&self.get("/api/v1/market/allTickers")?)
    }

    fn candles(
        &self,
        symbol: &str,
        interval: &str,
        start_s: i64,
        end_s: i64,
    ) -> Result<Vec<Candle>, String> {
        let path = format!(
            "/api/v1/market/candles?symbol={symbol}&type={interval}&startAt={start_s}&endAt={end_s}"
        );
        parse_candles(&self.get(&path)?)
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
    ) -> Result<Vec<Candle>, String> {
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
                        match tokio::task::spawn_blocking(move || m.all_tickers_last()).await {
                            Ok(Ok(prices)) => {
                                let snap = TickerSnapshot { at_ms: clock.exchange_now_ms(), prices };
                                let _ = tx.send(snap);
                            }
                            Ok(Err(e)) => log::warn!("allTickers poll failed: {e}"),
                            Err(e) => log::warn!("allTickers poll task failed: {e}"),
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
        fn exchange_time_ms(&self) -> Result<i64, String> {
            Ok(0)
        }
        fn all_tickers_last(&self) -> Result<BTreeMap<String, Decimal>, String> {
            Ok(BTreeMap::new())
        }
        fn candles(
            &self,
            _s: &str,
            _i: &str,
            start_s: i64,
            end_s: i64,
        ) -> Result<Vec<Candle>, String> {
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
}
