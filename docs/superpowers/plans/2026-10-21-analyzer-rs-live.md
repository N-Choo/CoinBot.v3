# Analyzer-RS Live Per-Symbol Signal Loop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run one live tokio task per symbol, keep a rolling candle window per symbol, recompute the RSI-divergence strategy on each newly closed candle, and emit each new signal to a sink.

**Architecture:** Add a `driver` layer shared by replay and live: one `CandleDriver` owns one `Window`, seeds dedup state at warm-up, and emits a signal once per new divergence. Add an `exchange` client for KuCoin closed candles and a `live` runtime that polls at candle boundaries. The trade engine is the gRPC **server** and the analyzer is the **client**: a supervisor polls `GetActiveSymbols`, spawns one task per active symbol, and a publisher pushes each new signal once via `Publish`. `LIVE_SYMBOLS` is the fallback when the engine returns an empty active set.

**Tech Stack:** Rust 2021, `rust_decimal`, `tokio`, `ureq`, `chrono`, `serde_json`, `thiserror`, `log`/`env_logger`, `tonic`/`prost`, `sha2`.

**Scope:** Live loop, gRPC activation, and at-most-once signal publish. No order placement, no persisted state, no backtest/HTML in this plan.

---

## File map

| File | Responsibility |
|---|---|
| `process/analyzer-rs/src/driver/mod.rs` | Driver exports |
| `src/driver/candle_driver.rs` | Per-symbol rolling window, warm-up seeding, dedup, signal events |
| `src/exchange/mod.rs` | Exchange exports |
| `src/exchange/timeframes.rs` | Fixed intervals, closed-candle rule, boundaries |
| `src/exchange/kucoin.rs` | KuCoin candles fetch + parse |
| `src/live/mod.rs` | Live exports |
| `src/live/config.rs` | Env config + validation (symbols, timeframe, window, endpoint, activation cadence) |
| `src/live/sink.rs` | `Sink` enum (`Log`, `Channel`, `Collect`) |
| `src/live/symbol_task.rs` | Cold start, batch processing, per-symbol loop |
| `src/live/grpc.rs` | Engine client, signal mapping, publisher |
| `src/live/supervisor.rs` | `GetActiveSymbols` polling and per-symbol task reconciliation |
| `build.rs` | Compiles the analyzer proto only for the `grpc` feature |
| `src/bin/analyzer-serve.rs` | Binary entry point, supervisor, publisher, Ctrl-C, `ANALYZER_ONCE` |
| `tests/driver.rs` | Driver integration over the synthetic fixture |
| `tests/live_helpers.rs` | `process_batch` / config tests |
| `tests/grpc_mapping.rs` | Signal -> proto mapping and stable `signal_id` |
| `README.md` | Run instructions |

---

### Task 1: `CandleDriver` — one window per symbol

**Files:**
- Create: `process/analyzer-rs/src/driver/mod.rs`
- Create: `process/analyzer-rs/src/driver/candle_driver.rs`
- Modify: `process/analyzer-rs/src/lib.rs`
- Create: `process/analyzer-rs/tests/driver.rs`

- [ ] **Step 1: Write the failing test**

`tests/driver.rs`:

```rust
mod common;

use analyzer_rs::driver::{CandleDriver, DriverEvent};
use analyzer_rs::strategy::config::Config;

#[test]
fn driver_warms_up_and_emits_only_new_divergences() {
    let f = common::load_fixture("synthetic_1day");
    let candles = f.candles();
    let cfg = Config::default();
    let mut driver = CandleDriver::new("SYNTH-USDT", cfg.clone());
    let mut warmed = false;
    let mut signals = 0usize;

    for (i, candle) in candles.iter().enumerate() {
        let events = driver.push_closed(candle.clone()).unwrap();
        for event in events {
            match event {
                DriverEvent::Warmed { .. } => {
                    assert_eq!(i + 1, cfg.window, "warms exactly when the window fills");
                    warmed = true;
                }
                DriverEvent::Signal { signal, .. } => {
                    assert!(warmed);
                    assert_eq!(signal.ticker, "SYNTH-USDT");
                    signals += 1;
                }
                DriverEvent::Suppressed { .. } => {}
            }
        }
        assert_eq!(driver.window().len(), (i + 1).min(cfg.window));
    }

    assert!(warmed, "driver must warm");
    assert!(signals > 0, "synthetic fixture must produce at least one new signal");
    assert!(driver.push_closed(candles.last().unwrap().clone()).is_err());
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --test driver
```

Expected: compile error (`driver` module missing).

- [ ] **Step 3: Implement the driver**

`src/driver/mod.rs`:

```rust
mod candle_driver;

pub use candle_driver::{CandleDriver, DivergenceKey, DriverEvent, SuppressReason};
```

`src/driver/candle_driver.rs`:

```rust
use crate::strategy::config::Config;
use crate::strategy::error::Result;
use crate::strategy::generator::{candidates, signals};
use crate::strategy::types::{Candle, Direction, DivergenceKind, Signal};
use crate::strategy::window::Window;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DivergenceKey {
    pub symbol: String,
    pub action: Direction,
    pub kind: DivergenceKind,
    pub divergence_open_time_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressReason {
    Duplicate,
    BelowWatermark,
    TooOld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    Warmed { seeded_keys: usize },
    Signal { signal: Signal, key: DivergenceKey },
    Suppressed { key: DivergenceKey, reason: SuppressReason },
}

/// One symbol's rolling window and emission state.
pub struct CandleDriver {
    symbol: String,
    window: Window,
    config: Config,
    emitted: BTreeSet<DivergenceKey>,
    watermark_ms: Option<i64>,
    warmed: bool,
    max_age_candles: Option<usize>,
}

impl CandleDriver {
    pub fn new(symbol: impl Into<String>, config: Config) -> Self {
        let capacity = config.window;
        CandleDriver {
            symbol: symbol.into(),
            window: Window::new(capacity),
            config,
            emitted: BTreeSet::new(),
            watermark_ms: None,
            warmed: false,
            max_age_candles: None,
        }
    }

    pub fn with_max_age(mut self, max_age_candles: Option<usize>) -> Self {
        self.max_age_candles = max_age_candles;
        self
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn window(&self) -> &[Candle] {
        self.window.as_slice()
    }

    pub fn is_warmed(&self) -> bool {
        self.warmed
    }

    pub fn push_closed(&mut self, candle: Candle) -> Result<Vec<DriverEvent>> {
        self.window.push_closed(candle)?;

        if !self.warmed {
            if !self.window.is_full() {
                return Ok(Vec::new());
            }
            let seeded = candidates(self.window.as_slice(), &self.config)?;
            for candidate in &seeded {
                let key = self.key(candidate.kind, candidate.index);
                self.watermark_ms = Some(match self.watermark_ms {
                    Some(current) => current.max(key.divergence_open_time_ms),
                    None => key.divergence_open_time_ms,
                });
                self.emitted.insert(key);
            }
            self.warmed = true;
            return Ok(vec![DriverEvent::Warmed { seeded_keys: seeded.len() }]);
        }

        let Some(signal) = signals(self.window.as_slice(), &self.symbol, &self.config)? else {
            return Ok(Vec::new());
        };
        let key = self.key(signal.divergence_kind, signal.divergence_index);

        if self.emitted.contains(&key) {
            return Ok(vec![DriverEvent::Suppressed { key, reason: SuppressReason::Duplicate }]);
        }
        if self
            .watermark_ms
            .is_some_and(|watermark| key.divergence_open_time_ms <= watermark)
        {
            return Ok(vec![DriverEvent::Suppressed {
                key,
                reason: SuppressReason::BelowWatermark,
            }]);
        }
        if self
            .max_age_candles
            .is_some_and(|max| signal.divergence_age_candles > max)
        {
            return Ok(vec![DriverEvent::Suppressed { key, reason: SuppressReason::TooOld }]);
        }

        self.emitted.insert(key.clone());
        self.watermark_ms = Some(key.divergence_open_time_ms);
        Ok(vec![DriverEvent::Signal { signal, key }])
    }

    fn key(&self, kind: DivergenceKind, index: usize) -> DivergenceKey {
        DivergenceKey {
            symbol: self.symbol.clone(),
            action: kind.action(),
            kind,
            divergence_open_time_ms: self.window.as_slice()[index].open_time_ms,
        }
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod driver;
```

- [ ] **Step 4: Run the driver test**

```bash
cargo test -p analyzer-rs --test driver
```

Expected: PASS. If it fails because the synthetic fixture has no post-warm-up signal, report it before changing the test; the fixture should have 158 positive signals.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src process/analyzer-rs/tests/driver.rs
git commit -m "feat(analyzer-rs): add per-symbol candle driver"
```

---

### Task 2: KuCoin closed-candle exchange client

**Files:**
- Create: `process/analyzer-rs/src/exchange/mod.rs`
- Create: `process/analyzer-rs/src/exchange/timeframes.rs`
- Create: `process/analyzer-rs/src/exchange/kucoin.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Modify: `process/analyzer-rs/src/lib.rs`
- Create: `process/analyzer-rs/tests/exchange.rs`

- [ ] **Step 1: Add dependencies**

`Cargo.toml`:

```toml
[features]
default = []
exchange = ["dep:ureq", "dep:serde_json"]
live = ["exchange", "dep:tokio", "dep:chrono", "dep:log", "dep:env_logger"]

[dependencies]
ureq = { version = "2", features = ["json"], optional = true }
serde_json = { version = "1", optional = true }
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time", "sync", "signal"], optional = true }
chrono = { workspace = true, optional = true }
log = { workspace = true, optional = true }
env_logger = { workspace = true, optional = true }
```

Add to `lib.rs`:

```rust
#[cfg(feature = "exchange")]
pub mod exchange;
```

- [ ] **Step 2: Write the failing tests**

`tests/exchange.rs`:

```rust
#![cfg(feature = "exchange")]

use analyzer_rs::exchange::kucoin::parse_klines;
use analyzer_rs::exchange::timeframes::Timeframe;

#[test]
fn parses_and_sorts_ku_coin_rows() {
    let raw = r#"{"code":"200000","data":[
        ["1700003600","101","103","104","100","12","1200"],
        ["1700000000","100","102","103","99","10","1000"]
    ]}"#;
    let candles = parse_klines(raw).unwrap();
    assert_eq!(candles.len(), 2);
    assert_eq!(candles[0].open_time_ms, 1_700_000_000_000);
    assert_eq!(candles[1].open_time_ms, 1_700_003_600_000);
    assert_eq!(candles[0].close.to_string(), "102");
}

#[test]
fn closed_candle_rule_excludes_the_forming_candle() {
    let tf = Timeframe::H4;
    let open = 1_700_000_000_000;
    assert!(!tf.is_closed(open, open + tf.millis(), 0));
    assert!(tf.is_closed(open, open + tf.millis() + 1, 0));
    assert!(!tf.is_closed(open, open + tf.millis() + 2_000, 5_000));
}

#[test]
fn timeframes_parse_ku_coin_names() {
    assert_eq!(Timeframe::from_kucoin("15min"), Some(Timeframe::M15));
    assert_eq!(Timeframe::from_kucoin("4hour"), Some(Timeframe::H4));
    assert_eq!(Timeframe::from_kucoin("1day"), Some(Timeframe::D1));
    assert_eq!(Timeframe::from_kucoin("nope"), None);
}
```

- [ ] **Step 3: Implement the timeframe table**

`src/exchange/timeframes.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M15,
    H4,
    D1,
}

impl Timeframe {
    pub fn from_kucoin(value: &str) -> Option<Self> {
        match value {
            "15min" => Some(Timeframe::M15),
            "4hour" => Some(Timeframe::H4),
            "1day" => Some(Timeframe::D1),
            _ => None,
        }
    }

    pub fn as_kucoin(self) -> &'static str {
        match self {
            Timeframe::M15 => "15min",
            Timeframe::H4 => "4hour",
            Timeframe::D1 => "1day",
        }
    }

    pub fn millis(self) -> i64 {
        match self {
            Timeframe::M15 => 900_000,
            Timeframe::H4 => 14_400_000,
            Timeframe::D1 => 86_400_000,
        }
    }

    /// A candle is closed once its close time plus the settle delay has passed.
    pub fn is_closed(self, open_time_ms: i64, now_ms: i64, settle_ms: i64) -> bool {
        now_ms - settle_ms >= open_time_ms + self.millis()
    }
}
```

`src/exchange/mod.rs`:

```rust
pub mod kucoin;
pub mod timeframes;
```

- [ ] **Step 4: Implement the KuCoin client**

`src/exchange/kucoin.rs`:

```rust
use crate::strategy::types::Candle;
use crate::exchange::timeframes::Timeframe;
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("kuCoin HTTP {0}")]
    Http(u16),
    #[error("kuCoin API error: {0}")]
    Api(String),
    #[error("kuCoin transport error: {0}")]
    Transport(String),
    #[error("kuCoin parse error: {0}")]
    Parse(String),
}

pub fn parse_klines(raw: &str) -> Result<Vec<Candle>, ExchangeError> {
    let body: Value = serde_json::from_str(raw)
        .map_err(|e| ExchangeError::Parse(format!("invalid JSON: {e}")))?;
    if body.get("code").and_then(Value::as_str) != Some("200000") {
        return Err(ExchangeError::Api(
            body.get("msg").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        ));
    }
    let rows = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ExchangeError::Parse("missing data array".into()))?;
    let mut candles = Vec::with_capacity(rows.len());
    for row in rows {
        let parts = row
            .as_array()
            .ok_or_else(|| ExchangeError::Parse("row is not an array".into()))?;
        if parts.len() < 6 {
            return Err(ExchangeError::Parse("row has fewer than 6 fields".into()));
        }
        let field = |index: usize| -> Result<Decimal, ExchangeError> {
            let text = parts[index]
                .as_str()
                .ok_or_else(|| ExchangeError::Parse("non-string OHLCV field".into()))?;
            Decimal::from_str(text)
                .map_err(|e| ExchangeError::Parse(format!("bad decimal {text:?}: {e}")))
        };
        candles.push(Candle {
            open_time_ms: parts[0]
                .as_str()
                .ok_or_else(|| ExchangeError::Parse("non-string timestamp".into()))?
                .parse::<i64>()
                .map_err(|e| ExchangeError::Parse(format!("bad timestamp: {e}")))?
                * 1_000,
            open: field(1)?,
            close: field(2)?,
            high: field(3)?,
            low: field(4)?,
            volume: field(5)?,
        });
    }
    candles.sort_by_key(|candle| candle.open_time_ms);
    candles.dedup_by_key(|candle| candle.open_time_ms);
    Ok(candles)
}

pub fn fetch_klines(
    base_url: &str,
    symbol: &str,
    timeframe: Timeframe,
    limit: usize,
) -> Result<Vec<Candle>, ExchangeError> {
    let now_s = chrono::Utc::now().timestamp();
    let span_s = ((limit as i64) * timeframe.millis() / 1_000) + 120;
    let start_s = now_s - span_s;
    let response = ureq::get(&format!("{base_url}/api/v1/market/candles"))
        .query("type", timeframe.as_kucoin())
        .query("symbol", symbol)
        .query("startAt", &start_s.to_string())
        .query("endAt", &now_s.to_string())
        .timeout(Duration::from_secs(30))
        .call();
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, _)) => return Err(ExchangeError::Http(code)),
        Err(ureq::Error::Transport(error)) => {
            return Err(ExchangeError::Transport(error.to_string()))
        }
    };
    let text = response
        .into_string()
        .map_err(|e| ExchangeError::Transport(e.to_string()))?;
    parse_klines(&text)
}
```

- [ ] **Step 5: Run the exchange tests**

```bash
cargo test -p analyzer-rs --features exchange --test exchange
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add process/analyzer-rs/Cargo.toml process/analyzer-rs/src/lib.rs process/analyzer-rs/src/exchange process/analyzer-rs/tests/exchange.rs
git commit -m "feat(analyzer-rs): add KuCoin closed-candle client"
```

---

### Task 3: Live config, signal sink, and per-symbol loop

**Files:**
- Create: `process/analyzer-rs/src/live/mod.rs`
- Create: `process/analyzer-rs/src/live/config.rs`
- Create: `process/analyzer-rs/src/live/sink.rs`
- Create: `process/analyzer-rs/src/live/symbol_task.rs`
- Modify: `process/analyzer-rs/src/lib.rs`
- Create: `process/analyzer-rs/tests/live_helpers.rs`

- [ ] **Step 1: Write the failing tests**

`tests/live_helpers.rs`:

```rust
#![cfg(feature = "live")]

use analyzer_rs::driver::CandleDriver;
use analyzer_rs::live::config::LiveConfig;
use analyzer_rs::live::sink::{SignalSink, VecSink};
use analyzer_rs::live::symbol_task::process_batch;
use analyzer_rs::strategy::config::Config;
use std::sync::Arc;

mod common;

#[test]
fn live_config_reads_symbols_and_defaults() {
    let cfg = LiveConfig::new(
        vec!["BTC-USDT".into(), "ETH-USDT".into()],
        "15min",
        100,
        5_000,
        1_000,
        "https://api.kucoin.com".into(),
    )
    .unwrap();
    assert_eq!(cfg.symbols.len(), 2);
    assert_eq!(cfg.timeframe.as_kucoin(), "15min");
    assert_eq!(cfg.window, 100);
}

#[test]
fn process_batch_emits_signals_through_the_sink() {
    let f = common::load_fixture("synthetic_1day");
    let candles = f.candles();
    let strategy = Config::default();
    let mut driver = CandleDriver::new("SYNTH-USDT", strategy.clone());
    let sink = Arc::new(VecSink::default());
    let sink_dyn: Arc<dyn SignalSink> = sink.clone();

    let emitted = process_batch(&mut driver, &candles, &sink_dyn).unwrap();
    assert_eq!(emitted, sink.signals().len());
    assert!(emitted > 0);
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --features live --test live_helpers
```

Expected: compile error (`live` module missing).

- [ ] **Step 3: Implement config**

`src/live/config.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LiveConfigError {
    #[error("no live symbols configured (set LIVE_SYMBOLS)")]
    NoSymbols,
    #[error("unsupported KUCOIN_TIMEFRAME {0:?} (use 15min, 4hour, or 1day)")]
    UnsupportedTimeframe(String),
    #[error("invalid {0}={1}")]
    Invalid(&'static str, String),
    #[error("window must be >= 8")]
    WindowTooSmall,
}

#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub symbols: Vec<String>,
    pub timeframe: crate::exchange::timeframes::Timeframe,
    pub window: usize,
    pub settle_ms: i64,
    pub poll_ms: u64,
    pub base_url: String,
}

impl LiveConfig {
    pub fn new(
        symbols: Vec<String>,
        timeframe: &str,
        window: usize,
        settle_ms: i64,
        poll_ms: u64,
        base_url: String,
    ) -> Result<Self, LiveConfigError> {
        if symbols.is_empty() {
            return Err(LiveConfigError::NoSymbols);
        }
        let timeframe = crate::exchange::timeframes::Timeframe::from_kucoin(timeframe)
            .ok_or_else(|| LiveConfigError::UnsupportedTimeframe(timeframe.to_string()))?;
        if window < 8 {
            return Err(LiveConfigError::WindowTooSmall);
        }
        Ok(LiveConfig { symbols, timeframe, window, settle_ms, poll_ms, base_url })
    }

    pub fn from_env() -> Result<Self, LiveConfigError> {
        let symbols = std::env::var("LIVE_SYMBOLS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty())
            .map(str::to_string)
            .collect();
        let timeframe = std::env::var("KUCOIN_TIMEFRAME").unwrap_or_else(|_| "15min".into());
        let window = parse("ANALYZER_WINDOW_CANDLES", 100)?;
        let settle_ms = parse("ANALYZER_CANDLE_SETTLE_MS", 5_000)?;
        let poll_ms = parse("ANALYZER_POLL_INTERVAL_MS", 1_000)?;
        let base_url = std::env::var("KUCOIN_BASE_URL")
            .unwrap_or_else(|_| "https://api.kucoin.com".into());
        Self::new(symbols, &timeframe, window, settle_ms, poll_ms, base_url)
    }
}

fn parse<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, LiveConfigError> {
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|_| LiveConfigError::Invalid(key, value)),
        Err(_) => Ok(default),
    }
}
```

- [ ] **Step 4: Implement the sink**

`src/live/sink.rs`:

```rust
use crate::strategy::types::Signal;
use std::sync::Mutex;

pub trait SignalSink: Send + Sync {
    fn publish(&self, signal: &Signal);
}

#[derive(Debug, Default)]
pub struct LogSink;

impl SignalSink for LogSink {
    fn publish(&self, signal: &Signal) {
        println!(
            "SIGNAL symbol={} action={:?} entry={} sl={} tp={} reason={} divergence_age={}c",
            signal.ticker,
            signal.action,
            signal.entry_price,
            signal.sl_price,
            signal.tp_price,
            signal.reason,
            signal.divergence_age_candles,
        );
    }
}

#[derive(Debug, Default)]
pub struct VecSink {
    signals: Mutex<Vec<Signal>>,
}

impl VecSink {
    pub fn signals(&self) -> Vec<Signal> {
        self.signals.lock().unwrap().clone()
    }
}

impl SignalSink for VecSink {
    fn publish(&self, signal: &Signal) {
        self.signals.lock().unwrap().push(signal.clone());
    }
}
```

- [ ] **Step 5: Implement batch processing and the live loop**

`src/live/symbol_task.rs`:

```rust
use crate::driver::{CandleDriver, DriverEvent};
use crate::exchange::kucoin::{fetch_klines, ExchangeError};
use crate::live::config::LiveConfig;
use crate::live::sink::SignalSink;
use crate::strategy::config::Config;
use crate::strategy::error::Error;
use crate::strategy::types::Candle;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error(transparent)]
    Strategy(#[from] Error),
    #[error(transparent)]
    Exchange(#[from] ExchangeError),
    #[error("not enough closed candles for {symbol}: {got} < {need}")]
    InsufficientHistory { symbol: String, got: usize, need: usize },
    #[error("fetch task failed: {0}")]
    Join(String),
}

/// Feed already-fetched candles through the driver and publish signals.
pub fn process_batch(
    driver: &mut CandleDriver,
    candles: &[Candle],
    sink: &Arc<dyn SignalSink>,
) -> Result<usize, LiveError> {
    let mut emitted = 0usize;
    for candle in candles {
        for event in driver.push_closed(candle.clone())? {
            if let DriverEvent::Signal { signal, .. } = event {
                sink.publish(&signal);
                emitted += 1;
            }
        }
    }
    Ok(emitted)
}

fn fetch_blocking(
    cfg: &LiveConfig,
    symbol: &str,
    limit: usize,
) -> Result<Vec<Candle>, LiveError> {
    Ok(fetch_klines(&cfg.base_url, symbol, cfg.timeframe, limit)?)
}

pub async fn warm_driver(
    cfg: &LiveConfig,
    strategy: &Config,
    symbol: &str,
) -> Result<(CandleDriver, i64), LiveError> {
    let cfg_owned = cfg.clone();
    let symbol_owned = symbol.to_string();
    let limit = cfg.window + 2;
    let candles = tokio::task::spawn_blocking(move || {
        fetch_blocking(&cfg_owned, &symbol_owned, limit)
    })
    .await
    .map_err(|e| LiveError::Join(e.to_string()))??;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut closed: Vec<Candle> = candles
        .into_iter()
        .filter(|candle| cfg.timeframe.is_closed(candle.open_time_ms, now_ms, cfg.settle_ms))
        .collect();
    if closed.len() < cfg.window {
        return Err(LiveError::InsufficientHistory {
            symbol: symbol.to_string(),
            got: closed.len(),
            need: cfg.window,
        });
    }
    let start = closed.len() - cfg.window;
    let mut driver = CandleDriver::new(symbol, strategy.clone());
    let _ = process_batch(&mut driver, &closed[start..], &Arc::new(crate::live::sink::LogSink))?;
    let cursor = closed.last().map(|c| c.open_time_ms).unwrap_or(0);
    Ok((driver, cursor))
}

pub async fn run_symbol(
    cfg: LiveConfig,
    strategy: Config,
    symbol: String,
    sink: Arc<dyn SignalSink>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), LiveError> {
    let (mut driver, mut cursor) = warm_driver(&cfg, &strategy, &symbol).await?;
    println!(
        "warmed symbol={symbol} candles={} window={}",
        driver.window().len(),
        cfg.window
    );

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(cfg.poll_ms)) => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                if now_ms - cfg.settle_ms < cursor + 2 * cfg.timeframe.millis() {
                    continue;
                }
                let cfg_owned = cfg.clone();
                let symbol_owned = symbol.clone();
                let limit = cfg.window + 5;
                let result = tokio::task::spawn_blocking(move || {
                    fetch_blocking(&cfg_owned, &symbol_owned, limit)
                })
                .await
                .map_err(|e| LiveError::Join(e.to_string()))?;
                let candles = match result {
                    Ok(candles) => candles,
                    Err(error) => {
                        eprintln!("fetch failed symbol={symbol}: {error}");
                        continue;
                    }
                };
                let now_ms = chrono::Utc::now().timestamp_millis();
                let mut new_candles: Vec<Candle> = candles
                    .into_iter()
                    .filter(|candle| {
                        candle.open_time_ms > cursor
                            && cfg.timeframe.is_closed(candle.open_time_ms, now_ms, cfg.settle_ms)
                    })
                    .collect();
                new_candles.sort_by_key(|candle| candle.open_time_ms);
                for candle in &new_candles {
                    cursor = candle.open_time_ms;
                }
                let emitted = process_batch(&mut driver, &new_candles, &sink)?;
                if emitted > 0 {
                    println!("emitted {emitted} signal(s) symbol={symbol}");
                }
            }
        }
    }
    println!("stopped symbol={symbol}");
    Ok(())
}

pub fn run_symbol_once(
    cfg: &LiveConfig,
    strategy: &Config,
    symbol: &str,
    sink: &Arc<dyn SignalSink>,
) -> Result<(usize, usize), LiveError> {
    let limit = cfg.window + 3;
    let candles = fetch_blocking(cfg, symbol, limit)?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let closed: Vec<Candle> = candles
        .into_iter()
        .filter(|candle| cfg.timeframe.is_closed(candle.open_time_ms, now_ms, cfg.settle_ms))
        .collect();
    if closed.len() < cfg.window + 1 {
        return Err(LiveError::InsufficientHistory {
            symbol: symbol.to_string(),
            got: closed.len(),
            need: cfg.window + 1,
        });
    }
    let mut driver = CandleDriver::new(symbol, strategy.clone());
    // Warm on the first `window` candles, then feed the remaining closed candles one at a time.
    process_batch(&mut driver, &closed[..cfg.window], sink)?;
    let emitted = process_batch(&mut driver, &closed[cfg.window..], sink)?;
    Ok((driver.window().len(), emitted))
}
```

`src/live/mod.rs`:

```rust
pub mod config;
pub mod sink;
pub mod symbol_task;
```

Add to `lib.rs`:

```rust
#[cfg(feature = "live")]
pub mod live;
```

- [ ] **Step 6: Run the live helper tests**

```bash
cargo test -p analyzer-rs --features live --test live_helpers
```

Expected: 2 tests pass.

- [ ] **Step 7: Commit**

```bash
git add process/analyzer-rs/src process/analyzer-rs/tests/live_helpers.rs
git commit -m "feat(analyzer-rs): add live per-symbol loop and log sink"
```

---

### Task 4: `analyzer-serve` binary and run instructions

**Files:**
- Create: `process/analyzer-rs/src/bin/analyzer-serve.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Modify: `process/analyzer-rs/README.md`

- [ ] **Step 1: Add the binary**

`Cargo.toml`:

```toml
[[bin]]
name = "analyzer-serve"
path = "src/bin/analyzer-serve.rs"
required-features = ["live"]
```

`src/bin/analyzer-serve.rs`:

```rust
use analyzer_rs::live::config::LiveConfig;
use analyzer_rs::live::sink::{LogSink, SignalSink};
use analyzer_rs::live::symbol_task::{run_symbol, run_symbol_once};
use analyzer_rs::strategy::config::Config;
use std::sync::Arc;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cfg = LiveConfig::from_env()?;
    let strategy = Config::default();
    let sink: Arc<dyn SignalSink> = Arc::new(LogSink);

    if std::env::var("ANALYZER_ONCE").as_deref() == Ok("1") {
        for symbol in &cfg.symbols {
            match run_symbol_once(&cfg, &strategy, symbol, &sink) {
                Ok((window, emitted)) => {
                    println!("once symbol={symbol} window={window} emitted={emitted}");
                }
                Err(error) => eprintln!("once failed symbol={symbol}: {error}"),
            }
        }
        return Ok(());
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut handles = Vec::new();
    for symbol in cfg.symbols.clone() {
        let cfg = cfg.clone();
        let strategy = strategy.clone();
        let sink = sink.clone();
        let shutdown = shutdown_rx.clone();
        handles.push(tokio::spawn(async move {
            if let Err(error) = run_symbol(cfg, strategy, symbol.clone(), sink, shutdown).await {
                eprintln!("symbol={symbol} stopped: {error}");
            }
        }));
    }

    tokio::signal::ctrl_c().await?;
    let _ = shutdown_tx.send(true);
    for handle in handles {
        let _ = handle.await;
    }
    Ok(())
}
```

- [ ] **Step 2: Add README run instructions**

Append to `process/analyzer-rs/README.md`:

```markdown
## Live per-symbol loop

```bash
LIVE_SYMBOLS=BTC-USDT,ETH-USDT \
KUCOIN_TIMEFRAME=15min \
ANALYZER_WINDOW_CANDLES=100 \
ANALYZER_CANDLE_SETTLE_MS=5000 \
ANALYZER_POLL_INTERVAL_MS=1000 \
cargo run -p analyzer-rs --features live --bin analyzer-serve
```

Each symbol gets its own tokio task and rolling window. After the window warms, every newly
closed candle is fed through the same `signals()` core used by the parity tests. A new divergence
is printed as:

```text
SIGNAL symbol=BTC-USDT action=Long entry=... sl=... tp=... reason=regular_bullish_divergence ...
```

`ANALYZER_ONCE=1` runs one closed-candle scan per symbol and exits (useful for a smoke test).
Use `RUST_LOG=info` for more detail. No orders are placed and no gRPC is used yet; the `SignalSink`
trait is the seam for the trade-engine publisher.
```

- [ ] **Step 3: Build and run the one-shot smoke test**

```bash
cargo run -p analyzer-rs --features live --bin analyzer-serve --locked
```

with:

```bash
ANALYZER_ONCE=1 LIVE_SYMBOLS=BTC-USDT KUCOIN_TIMEFRAME=15min \
cargo run -p analyzer-rs --features live --bin analyzer-serve
```

Expected: a `once symbol=BTC-USDT window=100 emitted=N` line. `N` may be 0 if the newest divergence was already in the warm-up window; that is normal. To see a positive signal deterministically, use the driver test with the synthetic fixture.

- [ ] **Step 4: Commit**

```bash
git add process/analyzer-rs/src/bin/analyzer-serve.rs process/analyzer-rs/Cargo.toml process/analyzer-rs/README.md
git commit -m "feat(analyzer-rs): add live analyzer-serve binary"
```

---

### Task 5: gRPC activation and at-most-once publish

The engine is the gRPC server (`process/proto/analyzer.proto`, `EngineService`); the analyzer is
the client. Activation polls `GetActiveSymbols`; each signal is published exactly once, with no
restart resend.

**Files:**
- Create: `process/analyzer-rs/build.rs`
- Create: `process/analyzer-rs/src/live/grpc.rs`
- Create: `process/analyzer-rs/src/live/supervisor.rs`
- Create: `process/analyzer-rs/tests/grpc_mapping.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Modify: `process/analyzer-rs/src/live/mod.rs`
- Modify: `process/analyzer-rs/src/bin/analyzer-serve.rs`
- Modify: `process/analyzer-rs/README.md`

- [ ] **Step 1: Add the `grpc` feature and proto build**

`Cargo.toml`:

```toml
[features]
default = []
exchange = ["dep:ureq", "dep:serde_json", "dep:chrono"]
live = ["exchange", "dep:tokio", "dep:chrono", "dep:log", "dep:env_logger"]
grpc = ["live", "dep:tonic", "dep:prost", "dep:sha2"]
```

Add the optional `tonic = "0.12"`, `prost = "0.13"`, and `sha2 = "0.10"` dependencies, the
`tonic-build = "0.12"` build dependency, and make `analyzer-serve` require
`["live", "grpc"]`.

`build.rs` compiles `../../process/proto/analyzer.proto` only when `CARGO_FEATURE_GRPC` is set,
so non-gRPC builds never need `protoc`.

- [ ] **Step 2: Map signals to the wire message**

`src/live/grpc.rs` wraps `EngineServiceClient<Channel>` with `active_symbols()` and
`publish()`. `to_proto_signal` maps a core `Signal` to a `ProtoSignal`:

- `ticker` uses `BTC/USDT` form (`-` -> `/`).
- `action` is `BUY` for `Direction::Long`, `SELL` for `Direction::Short`.
- `entry_price`, `sl_price`, `tp_price` are the `Decimal` -> `f64` boundary; a value that cannot
  be represented is dropped rather than defaulted.
- `trigger_time_ms = signal_candle_open_time_ms + timeframe.millis()`.
- `expires_at_ms = now_ms + signal_ttl_ms(timeframe)`, where the TTL is
  `timeframe.millis() / 10` clamped to `[60_000, 3_600_000]`.
- `signal_id` is a SHA-256 of the ticker, action, divergence kind, divergence open time, and
  `config_version` (`rsi-v1-w{window}`), prefixed with `v1:`. It is stable across retries/restarts
  and changes when the divergence or config changes.

`tests/grpc_mapping.rs` asserts the mapping, the 15-minute trigger time, the `v1:` prefix, stable
ids for identical input, and different ids for a different divergence time.

- [ ] **Step 3: Poll activation and reconcile one task per symbol**

`src/live/supervisor.rs` polls `GetActiveSymbols` every `ANALYZER_ACTIVATION_POLL_MS` (default
`30000`). The engine's ticker list is normalized from `BTC/USDT` to the KuCoin `BTC-USDT` form.
When the engine returns an empty set, `LIVE_SYMBOLS` is used as the fallback. Desired symbols are
diffed against the running tasks: newcomers get a `watch` stop channel and a `run_symbol` task;
symbols no longer desired are signalled to stop.

- [ ] **Step 4: Publish each signal once**

`src/bin/analyzer-serve.rs` connects a `Sink::Channel`, spawns `run_publisher`, and runs the
supervisor until Ctrl-C. `run_publisher` assigns a global monotonic `seq`, converts each signal,
and makes **one** `Publish` call. A transport error logs `publish failed ... (dropped)`; there is
no retry loop and no resend after a restart (the engine owns dedup via `signal_id`).

`ANALYZER_ONCE=1` still works: it reads the engine's active symbols (falling back to
`LIVE_SYMBOLS`), runs `run_symbol_once` per symbol with the log sink, and exits.

- [ ] **Step 5: Run the gRPC gate**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features grpc
cargo clippy -p analyzer-rs --features grpc --all-targets --no-deps -- -D warnings
cargo build -p analyzer-rs --features grpc --locked
```

Expected: all pass. `protoc` is required only for the `grpc` feature.

---

### Task 6: Final verification

- [ ] **Step 1: Run the full gate**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features grpc
cargo clippy -p analyzer-rs --features grpc --all-targets --no-deps -- -D warnings
cargo build -p analyzer-rs --features grpc --locked
```

Expected: all pass.

- [ ] **Step 2: Run the live one-shot smoke test**

```bash
ANALYZER_ONCE=1 \
GRPC_TRADE_ENGINE_ENDPOINT=http://127.0.0.1:50051 \
LIVE_SYMBOLS=BTC-USDT KUCOIN_TIMEFRAME=15min \
cargo run -p analyzer-rs --features grpc --bin analyzer-serve
```

Expected: `once symbol=... window=... emitted=...` output; no panic. If the network or engine is
unavailable, report that and cite the deterministic `driver` / `grpc_mapping` tests instead.

- [ ] **Step 3: Commit any formatting changes**

```bash
git add process/analyzer-rs
git commit -m "chore(analyzer-rs): finalize live loop verification"
```
