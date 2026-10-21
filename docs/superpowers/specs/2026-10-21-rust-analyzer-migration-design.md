# Rust Analyzer Migration — RSI-Divergence Analyzer (Backtest + Live)

- **Status:** Design revised per review; awaiting written-spec review
- **Date:** 2026-10-21
- **Scope:** Replace the Python `process/analyzer` with a new Rust crate that supports an offline
  backtest/replay path and a live path.
- **Related:**
  - `docs/superpowers/specs/2026-08-02-analyzer-module-design.md`
  - `docs/superpowers/specs/2026-08-02-benchmark-module-design.md`
  - `docs/superpowers/specs/2026-10-20-benchmark-replay-report-design.md`
  - `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md` (architecture reference)
  - `docs/superpowers/specs/2026-10-17-strategy-analyzer-live-phase1-2-design.md`

---

## 0. Context

The repository currently contains two analyzers:

1. **`process/analyzer` (Python)** — the RSI-divergence strategy. It has a live Redis-publishing
   path, a benchmark `SimEngine`, a text ledger, and a self-contained HTML replay report.
2. **`process/strategy-analyzer` (Rust)** — a separate supply-and-demand strategy. It established the
   layered architecture this migration follows: a pure deterministic core, an offline harness, and a
   live adapter that talks to the trade engine over gRPC.

This spec ports only analyzer (1) to Rust. The supply-and-demand analyzer is unchanged. The Rust
analyzer must reproduce the Python strategy's signal generation, then add a realistic replay, then a
gRPC live path. Python remains the behavioral oracle until parity is proven.

### 0.1 Verified repo facts that shape this design

- `process/analyzer/main.py` triggers on a Redis `tickers:analyze` batch, fetches KuCoin klines per
  ticker, calls `generate_signals(df)`, and publishes results to `signals:result`.
- Python live calls `fetch_klines` without `limit`; KuCoin's default response is 100 candles, and that
  response includes the in-progress (forming) candle. The strategy's effective live input is therefore
  100 candles whose last close may be unclosed.
- The Python indicators are:
  - `indicators/sma.py` — pandas `rolling(window).mean()`
  - `indicators/rsi.py` — Wilder RSI via `ewm(alpha=1/period, adjust=False)`, and divergence
    detection built on scipy `find_peaks(distance=order)`
  - `indicators/atr.py` — Wilder ATR via `ewm(alpha=1/period, adjust=False)`
- `signals/generator.py` scans the full input series for the most recent regular/hidden divergence,
  applies the optional SMA filter, picks the newest candidate, and derives SL/TP from ATR. A divergence
  from 90 candles ago can therefore still produce a signal at the latest close.
- `benchmark/engine.py` mirrors the trade engine: budget is `init_fund / 2`, multiple concurrent
  positions are allowed while `available_fund >= budget`, TP is checked before SL, fees are charged on
  both sides, and a loss is capped at the committed budget.
- `process/trade-engine` opens a position per accepted signal while the contract has
  `available_fund >= init_fund / 2`; there is no one-position-per-symbol cap.
- `process/proto/analyzer.proto` already defines the gRPC contract used by `strategy-analyzer`
  (`GetActiveSymbols`, `Publish`). The Rust analyzer will reuse it.
- `process/analyzer/fetchers/kucoin.py` converts epoch seconds with `datetime.fromtimestamp`
  (host-local time) and `benchmark/replay.py` formats dates with `strftime`. This is a legacy quirk, not
  a requirement (see decision 18).

### 0.2 Prerequisites to verify before implementation (not yet verified)

| # | Prerequisite | If false |
|---|---|---|
| P1 | The trade engine's `Publish` handler dedups by `signal_id` and returns an ack for duplicates | Add dedup to the engine before cutover, or downgrade the delivery guarantee in §7.4 to at-most-once with a persisted key |
| P2 | The engine honors `expires_at_ms` and rejects expired signals | The analyzer enforces expiry locally only; document the difference |
| P3 | A price-scale source exists per symbol (KuCoin `priceIncrement` from `/api/v2/symbols`, or an engine-side value) | Fall back to a per-symbol config table; fail-fast at startup if a symbol has no scale |
| P4 | KuCoin kline interval alignment (especially `1day`, `1week`, `1month`) is captured by the boundary fixture in §8 | Boundary function is corrected before live is enabled |

---

## 1. Goals and non-goals

### 1.1 Goals

- A new Rust crate that is a drop-in replacement for the Python analyzer's **backtest** and **live**
  features.
- **Exact behavioral parity of signal generation** with Python on identical closed-candle input
  windows:
  - all Python indicators ported (SMA, RSI, ATR, divergence detection);
  - identical action, reason, entry, SL/TP, confidence, and divergence index;
  - numeric values within `1e-9` relative.
- A **realistic replay** that drives the same signal-generation code as live, candle by candle, using
  the same rolling window and no future data.
- A **live path** that runs one tokio task per active symbol and publishes signals to the trade engine
  over gRPC with at-least-once delivery.
- The existing text ledger and the self-contained HTML replay report are preserved.

### 1.2 Non-goals

- Changing the supply-and-demand analyzer.
- Changing the Python strategy's rules, indicator periods, or defaults.
- A new fill model for live trading. The live path only publishes signals; the trade engine still owns
  order placement and position exits.
- Websocket feeds, exchange order placement, or account/risk state in the analyzer.
- Persisting the whole candle window across restarts. History is cheap to refetch.
- Multi-process state sharing for one symbol. The state store keeps the existing single-writer lock.
- Parity with Python's use of the forming candle. Rust uses closed candles only; the shadow
  comparison (§12) aligns the two explicitly.

---

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | New sibling crate `process/analyzer-rs` (package `analyzer-rs`) | Keeps the supply-and-demand analyzer stable; Python can run until parity. |
| 2 | Layers mirror `strategy-analyzer`: `strategy` → `driver` → `harness`/`live` | Proven dependency direction and test boundaries. |
| 3 | Core is `Decimal`-only, no `f32`/`f64`, no `HashMap` iteration, checked arithmetic only | Purity rule from `strategy-analyzer`, plus no overflow panics. |
| 4 | Parity is defined on signal generation over identical closed-candle windows, not on trade PnL | The replay intentionally changes fills for realism. |
| 5 | Rolling window `ANALYZER_WINDOW_CANDLES`, default 100; `W` is strategy semantics and is part of `config_version` and `signal_id` | EWM seeding makes RSI/ATR depend on where the window starts. |
| 6 | Backtest and live use the same rolling window and the same driver | Replay as if it were real; no growing `df[..i+1]` shortcut. |
| 7 | Preserve the backtest outputs (text ledger + HTML replay) | Full replacement of `make benchmark` and `make benchmark-web`. |
| 8 | Live uses gRPC `EngineService` like `strategy-analyzer` | Consistent with the current trade engine. |
| 9 | One tokio task per active symbol; a single publisher task owns the gRPC client and runs a bounded number of publishes concurrently | Per-symbol ownership, no N clients, no head-of-line blocking. |
| 10 | Signals travel over an awaited bounded `mpsc`; no silent drops. Expiry and stale-catch-up drops are explicit, logged, and counted | Backpressure plus observable loss. |
| 11 | Entry fills at the next candle open; SL/TP stay absolute (not rebased) | A signal is only known at close `i`; absolute levels match what the engine receives. |
| 12 | When a candle's range contains both TP and SL, assume SL first. Gap-through fills at the open | Conservative; avoids optimistic same-candle results. |
| 13 | Closed candles only (`open + interval <= exchange_now - settle`); the symbol task sleeps until the next candle boundary; no ticker poller and no fixed-rate poll | The strategy consumes candles; a `1day` candle closes once a day. |
| 14 | One position per divergence key (dedup across candles), multiple concurrent positions across different keys while `available_fund >= budget` | Both rules apply together: dedup prevents re-acting on one setup; the funding rule still allows distinct setups concurrently. |
| 15 | Python benchmark trade results are not a parity target | Next-open + SL-first intentionally differ from the old optimistic `SimEngine`. |
| 16 | `f64` is allowed only in `harness`/`live` reporting and serialization | Keeps the core pure while matching JSON/report needs. |
| 17 | Divergence keys include the divergence kind, and the driver keeps a bounded `BTreeSet` of emitted keys plus a watermark | Handles regular vs hidden on the same candle and A→B→A repainting. |
| 18 | Dates are UTC by default; `--tz local` is an opt-in for Python-identical ledgers | Removes host-timezone dependence from tests and ops. |
| 19 | Optional `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES`, off by default (parity); every signal reports its divergence age | Python can fire on very old divergences; operators decide whether to cap. |
| 20 | Price rounding to the symbol scale happens only in the publisher, after the parity boundary | Parity compares unrounded values. |
| 21 | Delivery is at-least-once: the cursor, emitted key, and a durable outbox entry are committed atomically; the outbox entry is cleared only after ack, expiry, or terminal rejection | Crash-safe; the engine dedups by `signal_id` (prerequisite P1). |
| 22 | Quarantine applies only to symbol-specific failures; transient/exchange-wide failures back off and trip a global circuit breaker | An exchange outage must not quarantine every symbol. |

---

## 3. Crate layout

New workspace member: **`process/analyzer-rs`** (package `analyzer-rs`).

```
process/analyzer-rs/
├── Cargo.toml
├── build.rs                    # compile process/proto/analyzer.proto (grpc feature only)
├── src/
│   ├── lib.rs
│   ├── strategy/               # PURE CORE
│   │   ├── mod.rs              # #![deny(clippy::float_arithmetic)]
│   │   ├── decimal.rs          # checked ops, Option<Decimal> helpers
│   │   ├── error.rs            # includes Error::Overflow, Error::DivByZero
│   │   ├── config.rs
│   │   ├── types.rs            # Candle, Direction, DivergenceKind, DivergenceKey, Signal
│   │   ├── window.rs
│   │   ├── indicators/{sma,rsi,atr,peaks}.rs
│   │   ├── divergence.rs
│   │   └── generator.rs
│   ├── driver/                 # shared by harness and live: no I/O, no clock
│   │   ├── mod.rs
│   │   └── candle_driver.rs
│   ├── exchange/               # KuCoin REST adapter, shared by fetch-data and live
│   │   ├── mod.rs
│   │   ├── kucoin.rs           # klines (paginated), symbols (priceIncrement), server time
│   │   ├── intervals.rs        # supported intervals + boundary arithmetic (§8)
│   │   └── retry.rs            # retry/backoff/rate limiter
│   ├── harness/
│   │   ├── mod.rs
│   │   ├── dataset.rs          # CSV load, provenance, hash
│   │   ├── fetch.rs            # fetch-data command
│   │   ├── sim.rs              # next-open / SL-first simulator
│   │   ├── ledger.rs
│   │   ├── recorder.rs
│   │   └── report.rs
│   └── live/
│       ├── mod.rs
│       ├── config.rs
│       ├── clock.rs            # exchange-offset-corrected time
│       ├── schedule.rs         # next boundary + jitter + miss-retry schedule
│       ├── symbol_task.rs
│       ├── supervisor.rs       # activation, task lifecycle, quarantine
│       ├── state_store.rs      # cursor + emitted keys + outbox + quarantine
│       ├── writer.rs           # dedicated blocking-write task
│       ├── publisher.rs        # Publisher trait, LogOnlyPublisher, metrics hooks
│       └── grpc.rs             # GrpcPublisher + activation client (feature grpc)
├── src/bin/analyzer-backtest.rs  # required-features = ["harness"]
├── src/bin/analyzer-serve.rs     # required-features = ["live"]
├── templates/replay.html         # copy of the current replay template
├── tools/parity/                 # Python oracle scripts (outside the crate src)
└── tests/
    ├── fixtures/ parity/
    ├── indicator_parity.rs  signal_parity.rs  driver_equivalence.rs
    ├── driver_seeding.rs  repaint.rs  intervals.rs
    ├── sim.rs  ledger_snapshot.rs  report_snapshot.rs
    ├── live_loop.rs  delivery.rs  purity.rs
```

Binaries are named `analyzer-backtest` and `analyzer-serve` to avoid colliding with the existing
`strategy-analyzer` bins `backtest`/`serve`.

### 3.1 Cargo features

```toml
[features]
default = []
exchange = ["dep:ureq", "dep:serde_json"]
harness  = ["exchange", "dep:clap", "dep:csv", "dep:sha2", "dep:chrono"]
live     = ["exchange", "dep:tokio", "dep:chrono", "dep:tracing"]
grpc     = ["live", "dep:tonic", "dep:prost"]
```

- The core (`strategy`, `driver`) has no optional dependencies beyond `rust_decimal`, `serde`,
  `thiserror`; it builds with `--no-default-features`.
- One HTTP stack: blocking `ureq`, wrapped in `spawn_blocking` for live. `fetch-data` and live share
  the same KuCoin adapter, retry policy, and rate limiter.
- `live` without `grpc` is a supported build: it ships `LogOnlyPublisher`, which is what shadow mode
  (§12) runs. `grpc` adds `GrpcPublisher` and the activation client.
- `chrono` appears under both `harness` and `live` because both format or compute times.

### 3.2 Dependency direction

`strategy` depends on neither `driver`, `harness`, nor `live`. `driver` depends only on `strategy`.
`exchange`, `harness`, and `live` depend on `driver` + `strategy`. The network, filesystem, clocks, and
gRPC stay outside the core.

---

## 4. Pure core

### 4.1 Candle and window

```rust
pub struct Candle {
    pub open_time_ms: i64,   // UTC ms, candle open
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

pub struct Window { candles: Vec<Candle>, capacity: usize }

impl Window {
    pub fn new(capacity: usize) -> Self;
    pub fn push_closed(&mut self, candle: Candle) -> Result<(), Error>; // rejects non-increasing open_time_ms
    pub fn as_slice(&self) -> &[Candle];
    pub fn len(&self) -> usize;
    pub fn is_full(&self) -> bool;
}
```

`push_closed` appends and evicts the oldest candle when `len > capacity`. A `Vec` is used instead of a
`VecDeque` so `as_slice(&self)` is possible without `make_contiguous(&mut self)`; eviction is
`candles.remove(0)` and costs an O(W) memmove of at most a few thousand candles once per interval,
which is negligible. It returns an error for a duplicate or out-of-order timestamp. The core never
resizes the window and never sees future candles. Gaps in time are not a core concern; the exchange
layer backfills holes before the driver sees them (§7.2).

### 4.2 Numeric policy

- `rust_decimal::Decimal` everywhere in `strategy`. Working precision is the `rust_decimal` default
  (28 significant digits). Constants such as `1e-10` are parsed once.
- **NaN has no `Decimal` equivalent.** Series are `Vec<Option<Decimal>>`; `None` means pandas `NaN`.
  Any comparison involving `None` is false, exactly as in pandas (`NaN > x`, `NaN < x`, `NaN == x` are
  all false). Arithmetic with `None` yields `None`.
- **Checked arithmetic only.** `rust_decimal`'s operators panic on overflow. The core uses
  `checked_add`/`checked_sub`/`checked_mul`/`checked_div` through helpers in `decimal.rs` that return
  `Error::Overflow` or `Error::DivByZero`. Bare `+ - * /` on `Decimal` in `strategy/` is forbidden;
  the purity test greps for it.
- No `f32`/`f64` tokens in `strategy/`; the existing purity token test is copied and run in CI. No
  `HashMap`/`HashSet` iteration; use `BTreeMap`/`BTreeSet`/`Vec`.
- **Rounding:** the core never rounds to a price scale. Signal levels are carried at working precision.
  The publisher rounds half-up to the symbol's price scale (prerequisite P3) at the last step before
  the wire. Parity tests compare unrounded values.
- **Determinism:** fixed operation order, no RNG, no wall clock in the core.
- **Float-noise policy:** pandas computes in `f64`; Rust computes in `Decimal`. Differences near a
  strict threshold are possible (see §10.3 for the dual-oracle triage).

### 4.3 Indicators

All indicator functions take the window slice and return `Vec<Option<Decimal>>` (or divergence flags).

- **SMA** — mean of the last `period` closes; `period-1` leading `None`. Port exactly, including the
  warm-up rule.
- **RSI** — Python's exact sequence:
  - `delta[i] = close[i] - close[i-1]`; the first delta is `None`.
  - `gain[i] = delta[i] > 0 ? delta[i] : 0`; `loss[i] = delta[i] < 0 ? -delta[i] : 0`. Because
    comparisons with `None` are false, the first gain and loss are `0`, not `None` (Python's `where`
    behavior).
  - `avg_gain`, `avg_loss` via `ewm(alpha = 1/period, adjust=False)`, seeded with the first value and
    recursing `avg = alpha * x + (1 - alpha) * prev`.
  - `rs = avg_gain / (avg_loss + 1e-10)`; `rsi = 100 - 100 / (1 + rs)`.
  - Flat series and zero denominators return values, not panics.
- **ATR** — true range `max(high-low, |high-prev_close|, |low-prev_close|)`; the first bar's TR is
  `high-low` (pandas `max` skips the `None` prev-close terms), then Wilder
  `ewm(alpha = 1/period, adjust=False)`.
- **Peaks** — a `Decimal` port of scipy `find_peaks(values, distance=order)`; see Appendix A.1 for the
  algorithm including plateau handling and the tie-order caveat.
- **Divergence** — port `detect_rsi_divergences`: peaks on `High`, troughs on `Low`; peaks/troughs on
  RSI; `_match_extrema` one-to-one temporal pairing (`rsi_idx` strictly increasing, distance
  `<= order`; Appendix A.2); regular/hidden bullish/bearish flag assignment.

### 4.4 Signal generation

`generator::signals(window: &[Candle], config: &Config) -> Result<Option<Signal>, Error>` is a
line-for-line port of Python `generate_signals`:

1. Compute RSI + divergence flags.
2. `price_diff = (close[last] - close[last-7]) / close[last-7]` (`0` when `len < 7`).
3. Scan newest → oldest for the most recent of each of the four divergence kinds.
4. Build confidence/reason candidates and apply the optional SMA filter.
5. Sort by divergence index (newest wins) and choose `best`.
6. Compute ATR-based SL/TP, including `atr_pct_high` scaling, `atr_high_adjust`, `atr_sl_max_pct`, and
   the long/short sign rules.
7. Return `None` when no candidate survives.

`Signal` keeps the Python fields (ticker, action, confidence, entry price, reason, SL, TP, divergence
index) and adds: `divergence_kind`, `divergence_open_time_ms`, `divergence_age_candles`
(`last_index - divergence_index`), and `signal_candle_open_time_ms`. SL/TP are zero when ATR is
unavailable, and the caller rejects non-positive levels, exactly as Python does.

The optional age cap (decision 19) is applied after `generator::signals` by the driver, never inside
the ported function, so the generator stays a pure parity port.

The core is stateless over the window: no cached EMA, no incremental peak state. Every call recomputes
the batch. This is what guarantees replay/live equivalence.

### 4.5 Window configuration

- `ANALYZER_WINDOW_CANDLES`, default `100`.
- Validation (fail-fast at startup):
  `W >= max(rsi_period, atr_period, sma_period if sma_enabled) + rsi_divergence_order + 2` and
  `W >= 8` (for `price_diff`).
- The same capacity is used by backtest and live. Because RSI/ATR EWM are seeded at the window's first
  candle, `W` changes signal values; it is therefore included in `config_version` (hash of every
  strategy parameter plus `W`) and in `signal_id`.
- If `W` exceeds KuCoin's 1500-candle response limit, the exchange adapter paginates
  (`startAt`/`endAt`) until the window is full.

---

## 5. Shared candle driver

The driver owns the rolling window and the emission rules. It performs no I/O and has no clock. Live
and harness both call exactly this code, so replay sees the same window, the same signal function, and
the same suppression rules as live.

```rust
pub struct DivergenceKey {
    pub symbol: String,
    pub action: Direction,
    pub kind: DivergenceKind,          // regular/hidden x bullish/bearish
    pub divergence_open_time_ms: i64,  // open time of the divergence candle
}

pub struct CandleDriver {
    symbol: String,
    window: Window,
    config: Config,
    emitted: BTreeSet<DivergenceKey>,  // pruned to keys still inside the window
    watermark_ms: Option<i64>,         // max divergence_open_time_ms ever emitted or seeded
    warmed: bool,
}

pub enum DriverEvent {
    Warmed { seeded_keys: usize },
    Signal { signal: Signal, key: DivergenceKey, candle_open_time_ms: i64 },
    Suppressed { key: DivergenceKey, reason: SuppressReason }, // Duplicate | BelowWatermark | TooOld
}

impl CandleDriver {
    pub fn push_closed(&mut self, candle: Candle) -> Result<Vec<DriverEvent>, Error>;
}
```

Pending entries are a simulator concern (§6.3); the driver only emits signals.

### 5.1 `push_closed` rules

1. Push the candle into the window. Reject duplicate or out-of-order timestamps with an error.
2. While `len < capacity`, return no events (still warming).
3. **Warm-up seeding.** On the push that fills the window for the first time, evaluate the generator
   once over the full window, collect every candidate (all four kinds, not only the best), insert all
   of their keys into `emitted`, set `watermark_ms` to the newest candidate's divergence time, set
   `warmed = true`, and return `Warmed { seeded_keys }`. No `Signal` is emitted. This prevents the
   first post-warm-up candle from firing a stale divergence that was already sitting in the warm-up
   window. (`generator::candidates()` returns all candidates; `generator::signals()` picks the best
   from it, so there is a single code path.)
4. After warm-up, run `signals(window, config)`. If it returns a best signal, build its key and apply,
   in order:
   - **Duplicate:** key already in `emitted` → `Suppressed(Duplicate)`.
   - **Watermark:** `divergence_open_time_ms <= watermark_ms` → `Suppressed(BelowWatermark)`. A setup
     is only eligible if it is strictly newer than anything already emitted or seeded. This closes the
     repaint hole: `find_peaks(distance=...)` can change which peaks survive when a new candle
     arrives, so divergence A can vanish, B can appear, and A can reappear (A→B→A). A single
     `last_emitted` key would re-emit A; the set plus `<=` watermark does not, and it also suppresses a
     same-candle regular↔hidden flip.
   - **Age cap (optional):** if `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES` is set and
     `divergence_age_candles` exceeds it → `Suppressed(TooOld)`. Off by default to preserve parity.
   - Otherwise emit `Signal`, insert the key, advance the watermark.
5. Prune `emitted` to keys whose divergence candle is still in the window. At most one `Signal` is
   emitted per candle.
6. Keys include `kind`, so a regular and a hidden divergence on the same candle are distinct keys (the
   generator still returns only the best one per call, and the `<=` watermark prevents a same-candle
   flip from emitting twice).

---

## 6. Backtest and replay

### 6.1 CLI

```
analyzer-backtest fetch-data --symbol SYM --interval 1day --days N [--window W] --out PATH
analyzer-backtest run        --data PATH [--data SYM=PATH ...] [--symbols ...] [--tz utc|local]
analyzer-backtest web        --data ... --out benchmark.html [--tz utc|local]
```

- `fetch-data` is the only command that touches the network. It fetches `BENCHMARK_DAYS` worth of
  replay candles plus `W` warm-up candles, so `BENCHMARK_DAYS=90` with `W=100` pulls 190 daily candles
  and replays 90. It writes CSV plus `<out>.provenance.json` (symbol, interval, range, row count, `W`,
  sha256), paginating past 1500 rows when needed.
- `run` prints the ledger for each symbol and a final summary. `web` runs the same replay and writes
  the self-contained HTML report.
- `run` and `web` print the number of replayable candles (`rows - W`) and fail with a clear message
  when it is zero.
- `--data` and `--symbols` are repeatable; defaults come from `BENCHMARK_TICKERS`.

### 6.2 Replay loop

For each symbol, with window capacity `W`:

1. Load candles ascending, reject duplicate or non-increasing timestamps, require at least `W + 1`.
2. Feed the first `W` candles to the driver. The driver seeds its keys and emits only `Warmed`.
3. For each subsequent candle `i`:
   1. Fill the pending entry (if any) at `candle[i].open`; the signal was produced at close `i-1`
      (see §6.3 for fill-time checks).
   2. Evaluate exits for all open positions, including one just filled, using candle `i` and the
      conservative rules below.
   3. Feed candle `i` to the driver. A new `Signal` becomes pending for candle `i+1`.
   4. Record the replay event for the report.
4. At end of data: a signal produced on the final candle is recorded as `unfilled_at_end`; open
   positions are valued at the last close. No forced close.

The signal is recorded at the close candle; the simulated entry is recorded on the next candle's open.

### 6.3 Simulator

Port Python's funding and PnL math (Appendix B), with these fill changes:

- **Scope:** one account per symbol, each starting with `init_fund`, as in the Python benchmark. Long
  and short positions may coexist in the same account; there is no netting (the engine has no
  per-symbol cap). A shared multi-symbol account is an open question (OQ-4).
- **Budget:** `order_budget = init_fund / 2`.
- **Funding check happens at fill time,** not signal time. If `available_fund < budget` when the
  pending entry is due, the entry is skipped and recorded as `skipped_funds`.
- **Entry price:** next candle's open.
- **SL/TP stay absolute.** Levels computed from close `i` are not rebased to the fill price. This is
  what the engine receives live.
- **Gap at the fill open:** if the fill open is already at or beyond SL or TP for that direction (long:
  `open <= SL` or `open >= TP`; short mirrored), the entry is skipped and recorded as `skipped_gap`.
  Default behavior; see OQ-3 for the alternative (fill and exit at the open).
- **Dedup interplay (decision 14):** the driver guarantees one entry per divergence key. Distinct keys
  may open concurrent positions while funds allow.
- **Fees:** `fee_rate` on entry notional and on exit proceeds, as in Python.
- **Exit precedence** for a long:
  1. gap: if `open <= SL`, fill at `open`; if `open >= TP`, fill at `open`;
  2. if both `low <= SL` and `high >= TP`, fill at **SL**;
  3. otherwise `low <= SL` → SL, `high >= TP` → TP.
  Short is the mirror.
- **Loss cap:** realized PnL is capped at `-E` (the committed budget); on close, credit `E + pnl` to
  `available_fund`.
- **No slippage in v1.** Slippage can be added later without changing the driver.
- `pnl_pct = pnl / E * 100`; entry/exit/reason fields match the Python `ClosedTrade` shape.

### 6.4 Ledger

The text ledger mirrors the Python output (ticker header, one row per trade with entry date,
direction, entry price, exit date, exit price, pnl percent, pnl amount, and reason; open rows; and the
summary footer) and adds counts for `skipped_funds`, `skipped_gap`, and `unfilled_at_end`. Dates are
formatted in UTC by default (decision 18); `--tz local` uses `chrono::Local` to reproduce Python's
host-local output exactly. Snapshot tests run both modes.

### 6.5 HTML report

- Reuse the current `process/analyzer/benchmark/replay_template.html` verbatim as
  `templates/replay.html`, embedded with `include_str!`.
- Port `build_ticker_replay`, `config_dict`, and `account_state` so the JSON payload keeps its existing
  keys: `generated_at`, `tickers[]` with `symbol`, `config`, `warmup`, `candles`, `trades`,
  `open_positions`, `events`, and `summary`, plus `failures`. New fields are additive only (signal age,
  divergence kind, skip reasons), so the unmodified template still renders.
- Decimal values are serialized as JSON numbers only at the harness boundary.
- The report remains a single file with no server or build step.

---

## 7. Live path

### 7.1 Supervisor

```
Supervisor
  ├─ activation poll (GetActiveSymbols) every ANALYZER_ACTIVATION_POLL_MS (30s)
  ├─ reconcile: spawn/stop one task per active symbol
  ├─ global circuit breaker shared by all symbol tasks
  └─ shutdown: stop tasks, flush state, abort stragglers after ANALYZER_SHUTDOWN_TIMEOUT_MS
```

Activation result handling distinguishes three cases:

| Result | Behavior |
|---|---|
| `Ok(non-empty)` | Reconcile to exactly that set. |
| `Ok(empty)` | Use the `LIVE_SYMBOLS` fallback set if configured; otherwise run no symbols. |
| `Err(unreachable/timeout)` | Keep the current set unchanged and log a warning. Do not fall back. Exception: at cold start with no current set, use `LIVE_SYMBOLS` if configured, else retry activation. |

Without the `grpc` feature there is no activation client: the supervisor runs `LIVE_SYMBOLS` only and
logs that activation polling is disabled.

Each symbol task has its own `watch` stop channel and `JoinHandle`; the supervisor tracks task id →
symbol. Panics are logged and counted; the task is respawned only on the next reconcile after a
backoff, never in a tight loop.

Failure classification (this replaces the blanket three-errors rule):

| Class | Examples | Action |
|---|---|---|
| Transient | timeouts, connection errors, HTTP 429/5xx | Exponential backoff with the shared retry policy. Never quarantines. Counts toward the global breaker. |
| Exchange-wide | many symbols failing at once | Global circuit breaker opens: all tasks pause fetches until a probe succeeds. No quarantine. |
| Symbol-specific | unknown/invalid symbol, delisted pair, persistently malformed or out-of-order candles after backfill | Quarantine that symbol until `quarantined_until` (persisted). |

The quarantine cooldown defaults to `clamp(3 * interval, 1h, 24h)`, so a 15min strategy is not benched
for a day, and can be overridden with `ANALYZER_QUARANTINE_COOLDOWN_MS`. Reconcile honors
`quarantined_until` so a bad symbol cannot respawn-loop on activation ticks.

### 7.2 Symbol task

One tokio task per symbol. It is schedule-driven, not tick-driven.

**Closed-candle rule.** A candle is closed iff
`open_time + interval <= exchange_now - settle_ms`, where `exchange_now` is local time corrected by the
exchange clock offset (refreshed at startup and at each wake-up). The forming candle is always
excluded.

**Startup / warm-up (cold or invalid state):**

1. Fetch the most recent `W + 2` candles (paginated if `W` is large), drop every candle that fails the
   closed-candle rule, and take the last `W` closed candles in ascending order. If fewer than `W`
   closed candles exist, the symbol is symbol-specific-failed (quarantine with reason
   `insufficient_history`).
2. Feed exactly those `W` candles to the driver. It seeds its keys (§5.1) and emits `Warmed`. Nothing
   is published.
3. Persist the cursor (open time of the last fed candle).

**Main loop:**

```
cursor = last processed closed candle open time
loop {
    next_check = close_time(next_candle_after(cursor)) + settle_ms + symbol_jitter
    // on restart with a stale cursor, next_check is already in the past: fetch immediately
    select! {
        _ = stop.changed()          => break,
        _ = sleep_until(next_check) => fetch_and_process()
    }
}
```

- **Normal path:** one wake-up and one REST call per candle per symbol (1 per day for `1day`, 96 per
  day for `15min`). The task sleeps for the whole interval; `select!` on the stop channel keeps
  shutdown instant even when it is asleep for 24 hours.
- **Jitter:** `symbol_jitter` is a stable per-symbol offset in `[0, ANALYZER_BOUNDARY_JITTER_MS)`
  (hash of the symbol), so N symbols do not hit REST in the same instant.
- **Miss-retry:** if the fetch returns no new closed candle, retry on the short schedule
  `ANALYZER_MISS_RETRY_SCHEDULE_MS` (default `2000,5000,10000`). If still missing, wait for the next
  boundary; the following fetch catches up from the cursor with no special case. A missing candle is
  not a failure and never causes quarantine: KuCoin omits candles for intervals with zero trades.
- **Holes:** if the returned candles skip an expected open time, re-request that range once. If the
  exchange still has no candle and a later one exists, accept the gap and do not synthesize a candle
  (Python does not either). Log and count gaps.
- **Processing:** for each closed candle in ascending order, call `driver.push_closed`. Out-of-order or
  duplicate candles are an error that triggers the backfill path; persistent failure is
  symbol-specific (quarantine).
- **Catch-up policy:** when several candles are processed in one batch (restart after downtime), a
  `Signal` is published only if the candle being processed (`signal_candle_open_time_ms`) is within
  `ANALYZER_MAX_CATCHUP_CANDLES` (default 2) of the newest closed candle. Older signals are logged as
  `stale_catchup`, counted, and still recorded in `emitted`/watermark so they are never replayed later.
  A restart after a week of downtime therefore cannot publish week-old signals.
- **Persistence:** every processed batch commits state (cursor, emitted keys, watermark, outbox; §7.3)
  through the dedicated writer task, and appends the input candles to the JSONL stream.

### 7.3 Publisher and delivery

A single publisher task owns the gRPC client behind a `Publisher` trait (`GrpcPublisher` under
`grpc`, `LogOnlyPublisher` otherwise).

**Pipeline**

- Symbol tasks send `PublishRequest { signal, deadline }` over a bounded `mpsc` with an awaited send
  (backpressure, no silent drop).
- The publisher assigns a monotonic `seq`, computes the stable
  `signal_id = hash(symbol, action, kind, divergence_open_time_ms, config_version)`
  (`config_version` includes `W`), and runs up to `ANALYZER_PUBLISH_CONCURRENCY` (default 4) publishes
  concurrently in a `JoinSet`. One stuck `Publish` retry therefore cannot stall other symbols.
- **Rounding:** the publisher rounds entry/SL/TP half-up to the symbol's price scale (P3) just before
  building the wire message. The core values are never rounded.
- **Mapping to `analyzer.Signal`:** `ticker` in slash form, `action` `BUY`/`SELL`, `entry_price`,
  `sl_price`, `tp_price`, `trigger_time_ms` = **close of the signal candle**
  (`signal_candle_open_time_ms + interval`), `reason` = the Python reason string exactly (divergence
  time and age are logged and carried in `signal_id`, not appended to `reason`), `zone_id` empty. The
  divergence candle is not the trigger time: Python can fire on an old divergence at a current close,
  and the engine would reject an old trigger as stale.

**Deadline and TTL**

- `expires_at_ms = enqueue_time + ANALYZER_SIGNAL_TTL_MS`. The default scales with the timeframe:
  `clamp(interval / 10, 60s, 1h)`, i.e. 1h for `1day` and 90s for `15min`. A flat 60s was unsuitable
  for daily candles.
- Each publish retries with backoff until ack, deadline, or shutdown. On deadline the signal is logged
  and counted (`signals_expired_total`) and the outbox entry is closed as expired. Transport errors
  retry; an engine rejection is a terminal outcome: log, count, close the outbox entry.

**Crash-safe ordering (at-least-once)**

1. The state commit that advances the cursor and records the emitted key also writes the signal into a
   durable outbox in the same atomic file write (write temp, fsync, rename).
2. The signal is then sent to the publisher. The symbol task does not block on the ack.
3. On ack, expiry, or terminal rejection, a second commit removes the outbox entry.
4. On restart, non-expired outbox entries are re-sent with the same `signal_id`; expired ones are
   dropped and counted.

Crash after commit but before ack: the outbox resends and the engine dedups (P1). Crash before commit:
the candle is reprocessed, the deterministic generator reproduces the same signal and the same
`signal_id`, and the engine dedups. No ordering of crash and ack can lose a signal, and none can create
a distinct duplicate.

### 7.4 Live semantics versus Python

- **Content parity:** action, reason, entry, SL/TP, and confidence are identical to Python for the same
  closed-candle window.
- **Forming candle:** Python live uses the in-progress candle as the last row and its unclosed close as
  entry. Rust never does. Drift against raw Python live output is therefore expected and is not a
  parity failure; the shadow comparison (§12) removes it by construction.
- **Cadence:** Python republishes the latest divergence on every trigger; the Rust driver emits each
  divergence exactly once.
- **Warm-up:** signals present in the warm-up window are seeded as already seen, not published, so a
  restart does not replay historical setups.
- **Transport:** gRPC replaces Redis. The trade engine is unchanged; the analyzer becomes an
  `EngineService` client like `strategy-analyzer`.

---

## 8. Timeframes and boundaries

Supported intervals (all KuCoin kline types): `1min`, `3min`, `5min`, `15min`, `30min`, `1hour`,
`2hour`, `4hour`, `6hour`, `8hour`, `12hour`, `1day`, `1week`, `1month`. Default `1day`. Unsupported
values fail at startup.

- `exchange/intervals.rs` is table-driven: each interval has a fixed length in ms, except `1month`,
  which advances by calendar month (UTC).
- `next_open(open)` is `open + length` for fixed intervals and the first instant of the next UTC month
  for `1month`. The code must not assume that `1week` candles start at an arbitrary epoch multiple of
  7 days.
- `close_time(open) = next_open(open)`.
- **Boundary fixture (P4).** A committed fixture of real KuCoin responses for every supported interval
  asserts that successive candle opens equal `next_open(previous)` and records the observed alignment
  (UTC midnight for `1day`; the weekday for `1week`; the first of the month for `1month`). The fixture
  is regenerated from a live fetch before enabling a new timeframe; a mismatch blocks live.
- `BENCHMARK_DAYS` is converted to a candle count per interval
  (`ceil(days * 86_400_000 / length)`; months use `ceil(days / 30)`), and `fetch-data` adds `W` on top.
- Coarse timeframes (`1week`, `1month`) with `W=100` need years of history; `fetch-data` warns when the
  range would exceed what KuCoin serves.

---

## 9. Configuration

Reuse the Python variable names so existing `.env` files keep working. Startup fails fast on any
invalid value or inconsistent combination (§4.5, §11).

| Variable | Default | Purpose |
|---|---|---|
| `KUCOIN_TIMEFRAME` | `1day` | Candle interval |
| `RSI_PERIOD` | `14` | RSI period |
| `RSI_DIVERGENCE_ORDER` | `5` | Peak/trough distance and pairing order |
| `PRICE_DIFF_THRESHOLD` | `0.05` | Weekly move confidence boost |
| `SMA_ENABLED` | `false` | Trend filter |
| `SMA_PERIOD` | `20` | SMA period |
| `ATR_PERIOD` | `14` | ATR period |
| `ATR_SL_MULTIPLIER` | `2.0` | Stop distance in ATR |
| `ATR_TP_MULTIPLIER` | `3.0` | Target distance in ATR |
| `ATR_PCT_HIGH` | `5.0` | High-volatility threshold |
| `ATR_HIGH_ADJUST` | `1.5` | Multiplier applied above the threshold |
| `ATR_SL_MAX_PCT` | `0.0` | Absolute SL cap (0 = off) |
| `ANALYZER_WINDOW_CANDLES` | `100` | Rolling window (backtest + live); part of `config_version` |
| `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES` | unset | Optional staleness cap (unset = Python parity) |
| `BENCHMARK_DAYS` | `90` | Replay length (warm-up `W` is added on top by `fetch-data`) |
| `BENCHMARK_INIT_FUND` | `1000` | Simulator capital per symbol |
| `BENCHMARK_FEE_RATE` | `0.001` | Per-side fee |
| `BENCHMARK_TICKERS` | `BTC-USDT,ETH-USDT,SOL-USDT` | Backtest symbols |
| `GRPC_TRADE_ENGINE_ENDPOINT` | `http://127.0.0.1:50051` | Engine gRPC endpoint |
| `LIVE_SYMBOLS` | empty | Fallback activation set (see §7.1) |
| `ANALYZER_ACTIVATION_POLL_MS` | `30000` | Engine activation poll |
| `ANALYZER_CANDLE_SETTLE_MS` | `5000` | Wait after a boundary before fetching |
| `ANALYZER_BOUNDARY_JITTER_MS` | `5000` | Per-symbol stable offset window |
| `ANALYZER_MISS_RETRY_SCHEDULE_MS` | `2000,5000,10000` | Short retries when the new candle is missing |
| `ANALYZER_MAX_CATCHUP_CANDLES` | `2` | Publish only signals this recent after downtime |
| `ANALYZER_SIGNAL_TTL_MS` | derived | `clamp(interval/10, 60s, 1h)` |
| `ANALYZER_PUBLISH_CONCURRENCY` | `4` | In-flight publishes |
| `ANALYZER_QUARANTINE_COOLDOWN_MS` | derived | `clamp(3*interval, 1h, 24h)`; symbol-specific failures only |
| `ANALYZER_BREAKER_FAILURES` / `ANALYZER_BREAKER_PROBE_MS` | `5` / `30000` | Global circuit breaker |
| `ANALYZER_SHUTDOWN_TIMEOUT_MS` | `10000` | Graceful shutdown before abort |
| `ANALYZER_STATE_DIR` | `data/analyzer/state` | Per-symbol state and outbox |
| `ANALYZER_STREAM_DIR` | `data/analyzer/stream` | Recorded JSONL input streams |
| `ANALYZER_HEARTBEAT_PATH` | `data/analyzer/heartbeat` | Touched by the supervisor for the healthcheck |
| `ANALYZER_SHADOW_MODE` | `false` | Use `LogOnlyPublisher` |
| `KUCOIN_BASE_URL` | `https://api.kucoin.com` | Exchange REST base |
| `ANALYZER_MAX_RETRIES` | `3` | Retry count for retryable HTTP |
| `ANALYZER_RETRY_BASE_MS` / `ANALYZER_RETRY_MAX_MS` | `250` / `5000` | Backoff base and cap |
| `ANALYZER_RATE_LIMIT_MIN_INTERVAL_MS` | `30` | Shared request spacing |

Removed: `ANALYZER_POLL_INTERVAL_MS` and `BENCHMARK_WINDOW_CANDLES`. The old 5-second poll was a
leftover from the supply-and-demand analyzer's live-ticker design and has no role for closed candles.

Feature flags: `harness` enables the backtest CLI/report; `live` enables the runtime (log-only publisher
included); `grpc` enables the gRPC publisher and activation client. The core builds with
`--no-default-features`.

---

## 10. Testing and parity

### 10.1 Python oracle

During migration, `tools/parity/` contains Python scripts that emit:

- indicator series (RSI, SMA, ATR) and the four divergence flags per dataset and window size;
- per-window signal outputs (action, reason, confidence, entry, SL/TP, divergence index, kind, age);
- a rolling-window trade oracle for sanity, clearly labelled non-authoritative for PnL because of the
  new fill model.

Artifacts are committed under `process/analyzer-rs/tests/parity/` with dataset hashes and the exact
config (including `W`) used to produce them.

### 10.2 Fixtures and gates

**Fixture sets**

- Handcrafted: first-value RSI/ATR behavior, flat series, `ATR = 0`, plateaus, equal extrema, ties at
  thresholds, SMA on/off, high-ATR scaling, `ATR_SL_MAX_PCT`, non-positive SL/TP rejection, A→B→A
  repaint, regular/hidden same-candle flip.
- Real data: committed slices for BTC/ETH/SOL/XRP at `1day`, `4hour`, and `15min`, each with
  provenance and hash sidecars.

**Gates**

| Gate | Rule |
|---|---|
| Indicators | `1e-9` relative tolerance for every value |
| Divergence flags / peak indices | exact |
| Signals | timestamp, action, reason, kind, divergence index exact; prices `1e-9` relative |
| Driver | same input stream produces identical events |
| Replay equivalence | recorded live JSONL replayed through the driver reproduces the same signals |
| Simulator | unit tests for next-open, SL-first, gap fills/skips, fees, loss cap, concurrency |
| HTML | payload snapshot + self-contained smoke test |
| Live | fake market + fake gRPC; activation add/remove, circuit breaker, quarantine cooldown, retry/expiry, backpressure, outbox restart |
| Purity | `strategy/` has no float tokens, no bare `Decimal` arithmetic, no hash-map iteration |

### 10.3 Dual-oracle triage for float/Decimal mismatches

If a Rust value differs from Python at a strict comparison (for example `pci < threshold`), run:

1. The normal pandas/`f64` Python oracle.
2. A high-precision Python mirror (same operation order, `decimal.Decimal` or `mpmath`).

Classify the mismatch:

- **Rust matches high precision, Python `f64` does not:** boundary noise. Change the port to apply the
  same last-bit-sensitive rounding pandas applies at that comparison (typically 15 significant digits
  before the strict comparison), document the operation, and re-run. Do not change the tolerance.
- **Rust matches neither:** a port bug. Fix the operation order or seeding.
- **Rust matches Python `f64` but not high precision:** acceptable; Python is the contract.

Every accepted compatibility shim must be recorded in the parity fixture notes. Widening a tolerance
or adding floats to the core requires a spec update.

### 10.4 Commands and CI

Make targets (names avoid colliding with the existing analyzer targets):

```
make analyzer-rs-check      # purity + fmt + clippy + core tests
make analyzer-rs-test       # all tests
make benchmark-rs           # analyzer-backtest run
make benchmark-web-rs       # analyzer-backtest web
```

---

## 11. Risks

| Risk | Mitigation |
|---|---|
| scipy `find_peaks` semantics differ (plateaus, distance ties) | Port the algorithm (Appendix A.1) and test exact peak indices against Python on adversarial fixtures. |
| Decimal vs pandas `f64` differences flip a strict comparison | Dual-oracle triage (§10.3); operation-order matching; `1e-9` gate. |
| `W` changes signal behavior | `W` is part of `config_version` and `signal_id`; baseline 100; the oracle regenerates for any configured `W`. |
| Dynamic `W > 1500` exceeds KuCoin's per-request limit | Paginate warm-up and `fetch-data`; multi-page tests. |
| More positions than a real account would take | Replay mirrors the trade engine's `init_fund/2` funding rule; flagged in the report. |
| Repeated signals for one divergence or repaint | Emitted set plus `<=` watermark; stable `signal_id`; engine dedup (P1). |
| Outbox growth during a long engine outage | Entries expire by TTL and are dropped on restart; add a bounded outbox/backpressure threshold (open item). |
| Blocking fsync/write on tokio workers | Dedicated writer task with a bounded queue. |
| Quarantine respawn loop | Persist `quarantined_until`; reconcile honors it. |
| Exchange outage quarantines every symbol | Failure classification plus global circuit breaker (decision 22). |
| `trigger_time_ms` stale because the divergence is old | Send the signal candle's close, not the divergence candle's close (§7.3). |
| Python local-time date formatting | UTC by default; `--tz local` opt-in (decision 18). |
| Missing/duplicate KuCoin candles | Reject duplicates/out-of-order, backfill holes, accept zero-trade gaps, quarantine only persistent symbol-specific errors. |

Open questions (not blockers):

- **OQ-1:** Should the outbox have a hard cap that pauses symbol tasks when the engine is unreachable?
- **OQ-2:** Should `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES` be enabled by default in production?
- **OQ-3:** Should a gap at the entry open fill-and-exit at the open instead of `skipped_gap`?
- **OQ-4:** Should the replay model a single shared multi-symbol account instead of one account per
  symbol?
- **OQ-5:** Should slippage be added to the simulator in v1?

---

## 12. Rollout and cutover

1. **Core + parity** — land the crate with indicators, divergence, generator, and the parity gate.
2. **Backtest** — land the driver, simulator, ledger, and HTML report; run the Rust and Python
   backtests side by side and diff **signals** per candle.
3. **Shadow live** — run `analyzer-serve` with `ANALYZER_SHADOW_MODE=true` (`LogOnlyPublisher`) next to
   Python; compare emitted signals on the same active symbols until zero signal drift.
4. **Cut over live** — switch the trade engine's analyzer to `analyzer-serve` over gRPC; keep Python
   stopped but deployed for one release.
5. **Remove Python** — delete `process/analyzer`, its Dockerfile/requirements, Python test targets, and
   `.env` entries that only Python used. Tag the last Python commit for reference.
6. **Docs** — update `process/analyzer-rs/README.md`, the root README, Makefile help, and CI.

### 12.1 Shadow comparison

Raw Python live output is not directly comparable because Python includes the forming candle. The
shadow harness instead:

1. Fetches the same closed-candle window from KuCoin (or a recorded stream) for each active symbol.
2. Runs Python `generate_signals` on the closed window and Rust `signals` on the identical window.
3. Compares action, reason, entry, SL/TP, confidence, kind, and divergence index per candle.
4. Applies the same dedup/warm-up rules to both before comparing the emitted sequence.

Cutover requires zero signal drift over a pre-agreed window (for example 7 days across the configured
symbols), replay-equivalence passing, gRPC acks observed, and no unexpected quarantines. No period runs
both analyzers publishing signals; Python is either log-only or stopped during shadowing.

---

## 13. Acceptance criteria

- `process/analyzer-rs` builds with `--no-default-features`, `--features harness`, `--features live`,
  and `--all-features`.
- The purity gate passes: no `f32`/`f64` tokens in `strategy/`, no bare `Decimal` arithmetic, no
  hash-map iteration.
- Indicator, divergence, and signal parity gates pass against committed Python artifacts at `1e-9`
  relative / exact discrete outcomes.
- `analyzer-backtest run` reproduces the ledger shape and the signal sequence for the fixture datasets;
  `analyzer-backtest web` writes a single-file report that opens and replays.
- Replay equivalence passes: a recorded live stream replayed through the driver produces identical
  signals.
- `analyzer-serve` reconciles active symbols, emits each divergence once, delivers at-least-once over
  gRPC, survives fetch failures, honors quarantine cooldowns, and does not publish stale catch-up
  signals.
- Python `process/analyzer` is removed only after the shadow run shows zero signal drift and the
  README/Makefile/CI reference the Rust crate.

---

## Appendix A — Python peak/divergence port

### A.1 scipy `find_peaks(values, distance=order)`

The port must reproduce scipy for `Vec<Option<Decimal>>` input.

1. **Local maxima** (endpoints are never peaks):
   - interior `i` is a peak when `values[i-1] < values[i] > values[i+1]`;
   - a plateau (equal consecutive values) is a peak only when it is bounded by strictly lower values;
     the reported index is the plateau's midpoint, matching scipy's `_local_maxima_1d`
     (`left + width / 2`).
2. **Distance filtering** — port scipy's `_select_by_peak_distance`:
   - process candidate peaks by descending height, stable by ascending index on ties;
   - keep the current peak and remove every candidate within `distance` indices;
   - the surviving set is re-sorted by ascending index before comparison.
   The exact tie order matters; the parity fixture includes equal-height peaks.
3. Comparisons with `None` are false, so `None` never forms a peak and never blocks one.

Troughs are found by applying the same procedure to `-values` for price (`-Low`) and RSI, exactly as
Python does.

### A.2 `_match_extrema(price_extrema, rsi_extrema, order)`

Port exactly:

```text
matches = []
used = set()
prev_rsi = -1
for p_idx in price_extrema:            # ascending
    best_r = -1
    best_dist = infinity
    for r_idx in rsi_extrema:          # ascending
        if r_idx in used or r_idx <= prev_rsi:
            continue
        dist = abs(r_idx - p_idx)
        if dist <= order and dist < best_dist:
            best_dist = dist
            best_r = r_idx
    if best_r != -1:
        matches.append((p_idx, best_r))
        used.add(best_r)
        prev_rsi = best_r
```

Ties on `best_dist` keep the first RSI extremum (smallest index), matching Python's strict `<`.
Divergence flags are then assigned exactly as in `detect_rsi_divergences`:

- bullish: `price_low[curr] < prev` and `rsi[curr] > prev` → regular;
  `price_low[curr] > prev` and `rsi[curr] < prev` → hidden;
- bearish: `price_high[curr] > prev` and `rsi[curr] < prev` → regular;
  `price_high[curr] < prev` and `rsi[curr] > prev` → hidden.

---

## Appendix B — Simulator math

Given `init_fund`, `fee_rate`, and a position with committed budget `E = init_fund / 2`:

- Entry: `available_fund -= E`; `total_traded_fund += E`.
- Exit proceeds: `proceeds = E * (exit_price / entry_price)`.
- Fees: `fees = fee_rate * E + fee_rate * proceeds`.
- Realized PnL:
  - long: `pnl = proceeds - E - fees`;
  - short: `pnl = E - proceeds - fees`.
- Loss cap: `pnl = max(pnl, -E)`.
- Credit on close: `available_fund += E + pnl`.
- `pnl_pct = pnl / E * 100` (`0` when `E == 0`).
- Multiple positions are allowed while `available_fund >= E`.

This matches `benchmark/engine.py`; only the fill timing, gap handling, and SL-first ambiguity differ
from the Python benchmark.
