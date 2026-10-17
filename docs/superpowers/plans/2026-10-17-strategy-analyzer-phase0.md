# Rust Strategy Analyzer — Phase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `process/strategy-analyzer`, a pure deterministic Rust core implementing the supply-and-demand 4H strategy, plus an offline backtest/replay harness that runs the core identically to how it will run live.

**Architecture:** The crate splits into `strategy/` (pure, I/O-free, `Decimal`-only, no float arithmetic, no wall-clock, no `HashMap` iteration) and `harness/` (feature-gated; owns data fetching, tick generation, position simulation, and reporting). `harness → strategy` is a one-way dependency. The core exposes `step(&StrategyState, Input, &Config) -> Result<(StrategyState, Vec<Output>), StrategyError>` with validate-then-commit semantics.

**Tech Stack:** Rust 1.98.1, `rust_decimal` (+serde), `serde`/`serde_json`, `thiserror`, `sha2`; harness-only: `clap`, `ureq`, `csv`. Spec: `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md`.

**Conventions:** TDD per task (write failing test → run → implement → run → commit). Never use `f32`/`f64` in `strategy/`. Run `cargo test -p strategy-analyzer` from the repo root. Commit prefixes: `feat:`/`test:`/`chore:`/`docs:`.

> **Revision (2026-10-17, second review).** Applied after the first plan draft: (1) bounded rolling-window state + running ATR (no per-tick full-history clone); (2) BOS marks **every** cleared level and follows the §6.3 eligibility table, and trend stays `Unknown` when no valid low/high exists; (3) a separate `TerminalReason` enum; (4) `try_entry` takes the touching tick's time and uses the zone's incremental window extremes for the obstacle; (5) `ATR == 0` skips only BOS/zone creation; (6) sim fills TP at its limit and the stop at the stop price (or the gap tick); (7) calendar-block bootstrap with trade timestamps; (8) dataset gap policy; (9) fetch sorting/dedup/ms/retry; (10) signal-producing fixture + edge-case/verdict/perturbation/calibration tasks (27–31) and review gates.
>
> **Revision (2026-10-17, third review — Tasks 30–31 implemented + CI hardening).** Tasks 30/31 are implemented with pre-registered constants, not ad-hoc logic: `perturb::DIAGNOSTICS_BUDGET_SECS` (4H-synthesis-only, seeded via `report::Lcg`, reports variant counts, diagnostics-only — never a selector) and `report::AGG_TOLERANCE` = 1e-9 relative (1m aggregate vs. fetched 4H: mismatches reported, the run refused above tolerance, never patched; offline — network stays in `fetch-data`). The verdict is a three-state `VerdictOutcome { Insufficient, Fail, Pass }` with `n < MIN_EXECUTED_TRADES` → `Insufficient`. CI: a `syn` token-level purity test (the grep is a comment-stripped pre-filter), a `check_lock` mismatch test, no golden write-if-empty fallback, and the matrix named by architecture (x86_64 + arm64). Spec §3/§4/§5/§8.4/§10.2/§10.4/§10.5/§13 updated to match.

---

## File Structure

```
process/strategy-analyzer/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── strategy/
│   │   ├── mod.rs            # #![deny(clippy::float_arithmetic)]
│   │   ├── decimal.rs        # scales, half-up rounding helper, checked ops
│   │   ├── error.rs          # StrategyError
│   │   ├── config.rs         # Config, validation, config_version hash
│   │   ├── types.rs          # Input/Output/State/Zone/Signal/TrendState/Candle/Tick
│   │   ├── step.rs           # step() dispatch + event ordering + validation
│   │   ├── indicators/{mod,atr}.rs
│   │   ├── structure.rs      # pivots, BOS, trend, valid high/low
│   │   ├── zones.rs          # creation, mitigation, lifecycle, selection
│   │   └── signals.rs        # trigger, levels, obstacle, target modes, payload
│   └── harness/
│       ├── mod.rs
│       ├── dataset.rs        # cached OHLCV + dataset hash
│       ├── fetch.rs          # KuCoin paging (fetch-data only)
│       ├── tick_source.rs    # TickSource + Ohlcv4hSynthesis
│       ├── replay.rs         # drive core; replay-equivalence helpers
│       ├── sim.rs            # position simulation
│       ├── report.rs         # ledger + stats + comparisons
│       └── lock.rs           # freeze lock + OOS one-shot
│   └── bin/backtest.rs       # [[bin]] required-features = ["harness"]
└── tests/{replay_equivalence.rs,integration_backtest.rs,fixtures/small_4h.csv}
```

---

## Task 1: Crate scaffold, features, and workspace wiring

**Files:**
- Create: `process/strategy-analyzer/Cargo.toml`
- Create: `process/strategy-analyzer/src/lib.rs`
- Create: `process/strategy-analyzer/src/strategy/mod.rs`
- Create: `process/strategy-analyzer/src/harness/mod.rs`
- Create: `process/strategy-analyzer/src/bin/backtest.rs`
- Modify: `Cargo.toml` (root `[workspace].members`)
- Modify: `Makefile:1` (`PACKAGES`)

- [ ] **Step 1: Create the crate files**

`src/lib.rs`:
```rust
pub mod strategy;

#[cfg(feature = "harness")]
pub mod harness;
```

`src/strategy/mod.rs`:
```rust
#![deny(clippy::float_arithmetic)]

pub const STATE_SCHEMA_VERSION: u32 = 1;
```

`src/harness/mod.rs`:
```rust
//! Backtest/replay harness. Depends on `strategy`; `strategy` never depends on this.
```

`src/bin/backtest.rs`:
```rust
fn main() {
    println!("strategy-analyzer backtest harness (stub)");
}
```

`Cargo.toml`:
```toml
[package]
name = "strategy-analyzer"
version = "0.1.0"
edition = "2021"

[features]
default = []
harness = ["dep:clap", "dep:ureq", "dep:csv", "dep:sha2"]

[dependencies]
rust_decimal = { version = "1", features = ["serde-with-str"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
sha2 = { version = "0.10", optional = true }
clap = { version = "4", features = ["derive"], optional = true }
ureq = { version = "2", optional = true }
csv = { version = "1", optional = true }

[[bin]]
name = "backtest"
path = "src/bin/backtest.rs"
required-features = ["harness"]
```

Add `"process/strategy-analyzer"` to root `Cargo.toml` `[workspace].members`.

Change `Makefile:1` to:
```make
PACKAGES = api-gateway share deposit trade-engine strategy-analyzer
```

- [ ] **Step 2: Run both builds to verify feature gating**

Run: `cargo build -p strategy-analyzer --no-default-features`
Expected: builds pure core only; `backtest` bin skipped; no `clap`/`ureq` compiled.

Run: `cargo build -p strategy-analyzer --features harness`
Expected: builds including the `backtest` bin.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer Cargo.toml Makefile
git commit -m "feat(strategy-analyzer): scaffold pure core + harness feature crate"
```

---

## Task 2: `decimal.rs` — scales, half-up rounding, checked ops

**Files:**
- Create: `process/strategy-analyzer/src/strategy/decimal.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/decimal.rs`:
```rust
use rust_decimal::{Decimal, RoundingStrategy};

pub const WORKING_SCALE: u32 = 12;
pub const RATIO_SCALE: u32 = 4;

/// Half-up rounding. rust_decimal defaults to half-even, so request explicitly.
pub fn round_to(value: Decimal, scale: u32) -> Decimal {
    value.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

/// Round a level to the symbol's price scale (half-up).
pub fn round_price(value: Decimal, price_scale: u32) -> Decimal {
    round_to(value, price_scale)
}

/// Round a ratio for serialization/reporting only (toward zero).
pub fn round_ratio_for_output(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(RATIO_SCALE, RoundingStrategy::ToZero)
}

/// Round an intermediate indicator value to the working scale.
pub fn work(value: Decimal) -> Decimal {
    round_to(value, WORKING_SCALE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    #[test]
    fn half_up_not_half_even() {
        assert_eq!(round_to(dec("2.5"), 0), Decimal::from(3));
        assert_eq!(round_to(dec("3.5"), 0), Decimal::from(4));
    }

    #[test]
    fn price_scale_respected() {
        assert_eq!(round_price(dec("1.23456"), 2), dec("1.23"));
    }

    #[test]
    fn ratio_output_rounds_toward_zero() {
        // 2.99996 -> 2.9999, so a 3.0 floor is not passed by rounding.
        assert_eq!(round_ratio_for_output(dec("2.99996")), dec("2.9999"));
    }
}
```

Add `pub mod decimal;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features decimal::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/decimal.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): decimal rounding policy"
```

---

## Task 3: `error.rs` — `StrategyError`

**Files:**
- Create: `process/strategy-analyzer/src/strategy/error.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing test + implementation**

Create `src/strategy/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error("candle out of order at {got_ms} (expected > {prev_ms})")]
    OutOfOrderCandle { got_ms: i64, prev_ms: i64 },
    #[error("duplicate candle at {0}")]
    DuplicateCandle(i64),
    #[error("gapped candles: expected {expected_ms}, got {got_ms}")]
    GappedCandle { expected_ms: i64, got_ms: i64 },
    #[error("malformed candle at {at_ms}: {why}")]
    MalformedCandle { at_ms: i64, why: &'static str },
    #[error("malformed tick: price {price}")]
    MalformedTick { price: String },
    #[error("tick out of order at {got_ms} (last {last_ms})")]
    OutOfOrderTick { got_ms: i64, last_ms: i64 },
    #[error("decimal overflow")]
    DecimalOverflow,
    #[error("division by zero")]
    DivideByZero,
    #[error("invalid config: {0}")]
    ConfigInvalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_are_equatable_and_displayable() {
        let e = StrategyError::MalformedTick { price: "-1".into() };
        assert_eq!(e.clone(), e);
        assert!(e.to_string().contains("malformed tick"));
    }
}
```

Add `pub mod error;` to `strategy/mod.rs`.

Note: `AtrZero` is intentionally **not** an error (handled in-band, see Task 16).

- [ ] **Step 2: Run the test**

Run: `cargo test -p strategy-analyzer --no-default-features error::`
Expected: 1 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/error.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): typed StrategyError"
```

---

## Task 4: `config.rs` — config, validation, `config_version` hash

**Files:**
- Create: `process/strategy-analyzer/src/strategy/config.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/config.rs`:
```rust
use crate::strategy::error::StrategyError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode { FixedR, Structural }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub atr_period: u32,
    pub pivot_left: u32,
    pub pivot_right: u32,
    pub bos_min_distance_atr: Decimal,
    pub zone_search_lookback: u32,
    pub impulse_body_threshold_atr: Decimal,
    pub zone_expiry_candles: u32,
    pub stop_buffer_atr: Decimal,
    pub min_zone_height_atr: Decimal,
    pub max_zone_height_atr: Decimal,
    pub mitigation_grace_candles: u32,
    pub target_mode: TargetMode,
    pub target_r: Decimal,
    pub min_gross_rr: Decimal,
    pub structural_buffer_atr: Decimal,
    pub allow_no_obstacle: bool,
    pub engine_net_floor: Decimal,
    pub working_scale: u32,
    pub ratio_serialization_scale: u32,
    /// Per-symbol price scale (dp).
    pub price_scale: BTreeMap<String, u32>,
}

impl Config {
    pub fn default_for(symbols: &[&str]) -> Self {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        Config {
            atr_period: 14, pivot_left: 2, pivot_right: 2,
            bos_min_distance_atr: d("0.1"), zone_search_lookback: 10,
            impulse_body_threshold_atr: d("1.5"), zone_expiry_candles: 30,
            stop_buffer_atr: d("0.1"), min_zone_height_atr: d("0.5"),
            max_zone_height_atr: d("3.0"), mitigation_grace_candles: 1,
            target_mode: TargetMode::FixedR, target_r: d("3.0"),
            min_gross_rr: d("3.0"), structural_buffer_atr: d("0.1"),
            allow_no_obstacle: true, engine_net_floor: d("2.5"),
            working_scale: 12, ratio_serialization_scale: 4,
            price_scale: symbols.iter().map(|s| (s.to_string(), 4)).collect(),
        }
    }

    pub fn validate(&self) -> Result<(), StrategyError> {
        let bad = |m: &str| StrategyError::ConfigInvalid(m.to_string());
        if self.atr_period == 0 { return Err(bad("atr_period must be >= 1")); }
        if self.pivot_left == 0 || self.pivot_right == 0 { return Err(bad("pivot windows must be >= 1")); }
        if self.min_zone_height_atr > self.max_zone_height_atr { return Err(bad("min_zone_height > max_zone_height")); }
        if self.bos_min_distance_atr <= Decimal::ZERO { return Err(bad("bos_min_distance must be positive")); }
        if self.impulse_body_threshold_atr <= Decimal::ZERO { return Err(bad("impulse threshold must be positive")); }
        if self.target_r <= self.engine_net_floor { return Err(bad("target_r must exceed engine net floor (cushion)")); }
        if self.min_gross_rr <= self.engine_net_floor { return Err(bad("min_gross_rr must exceed engine net floor (cushion)")); }
        if self.price_scale.is_empty() { return Err(bad("price_scale required per symbol")); }
        Ok(())
    }

    /// Deterministic canonical JSON used for `config_version`.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("config is serializable")
    }
}

/// `config_version` = sha256 hex of the canonical config. Hashing lives at the edge.
pub fn config_version(canonical_json: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(canonical_json.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default_for(&["XRP-USDT"]).validate().is_ok());
    }

    #[test]
    fn target_r_below_floor_rejected() {
        let mut c = Config::default_for(&["XRP-USDT"]);
        c.target_r = dec("2.0");
        assert!(matches!(c.validate(), Err(StrategyError::ConfigInvalid(_))));
    }

    #[test]
    fn min_gross_rr_below_floor_rejected() {
        let mut c = Config::default_for(&["XRP-USDT"]);
        c.min_gross_rr = dec("2.5");
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_version_hashes_json_stably() {
        let c = Config::default_for(&["XRP-USDT"]);
        let j = c.canonical_json();
        assert_eq!(config_version(&j), config_version(&j));
        assert_ne!(config_version(&j), config_version("{}"));
    }
}
```

Note: `sha2` is a harness feature dep but `config_version` uses it. Make `sha2` **non-optional** (move it out of the `harness` feature in `Cargo.toml`) so the core-only build compiles. Update the `[features]` line and `[dependencies]` accordingly:

```toml
harness = ["dep:clap", "dep:ureq", "dep:csv"]
...
sha2 = "0.10"
```

Add `pub mod config;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features config::`
Expected: 4 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/config.rs process/strategy-analyzer/src/strategy/mod.rs process/strategy-analyzer/Cargo.toml
git commit -m "feat(strategy-analyzer): config, validation, config_version hash"
```

---

## Task 5: `types.rs` — inputs, outputs, state, zone, signal

**Files:**
- Create: `process/strategy-analyzer/src/strategy/types.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/types.rs`:
```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub type Ms = i64;

/// Bounded rolling window of closed candles kept in state.
pub const WINDOW_CANDLES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candle { pub open_time_ms: Ms, pub open: Decimal, pub high: Decimal, pub low: Decimal, pub close: Decimal }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick { pub time_ms: Ms, pub price: Decimal, pub is_bar_open: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction { Long, Short }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendState { Unknown, Bullish, Bearish }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneKind { Demand, Supply }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneState { Active, Consumed, Invalidated, Expired }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumeReason {
    SignalPublished, MissedTouch, EngineAccepted,
    EngineRejectedStrategy, EngineRejectedTransient, ExpiredUndelivered,
}

/// Why a zone became terminal without a signal. Separate from `ConsumeReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalReason { TrendFlip, CloseBeyondEdge, Expired }

/// A zone event carries either a terminal reason or a consumption reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneEventReason { Terminal(TerminalReason), Consumed(ConsumeReason) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub kind: ZoneKind,
    pub low: Decimal,
    pub high: Decimal,
    pub atr: Decimal,                 // ATR at activation
    pub breakout_time_ms: Ms,
    pub zone_candle_time_ms: Ms,
    pub activated_at_index: u64,      // absolute candle index at activation
    pub state: ZoneState,
    pub terminal_reason: Option<TerminalReason>,
    pub consume_reason: Option<ConsumeReason>,
    pub attempt_made: bool,
    pub window_high: Decimal,         // max high from breakout candle through latest closed candle
    pub window_low: Decimal,          // min low over the same window (includes the breakout candle)
}

impl Zone {
    /// Phase 0: every non-Active state is terminal.
    pub fn is_terminal(&self) -> bool { !matches!(self.state, ZoneState::Active) }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    pub schema_version: u32,
    pub event_type: String,
    pub signal_id: String,
    pub strategy: String,
    pub strategy_version: String,
    pub config_version: String,
    pub symbol: String,
    pub market: String,
    pub direction: Direction,
    pub timeframe: String,
    pub trend_state: TrendState,
    pub zone_id: String,
    pub zone_low: Decimal,
    pub zone_high: Decimal,
    pub zone_height_atr: Decimal,
    pub atr: Decimal,
    pub trigger_price: Decimal,
    pub reference_entry_price: Decimal,
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub target_mode: String,
    pub target_r: Decimal,
    pub min_gross_rr: Decimal,
    pub gross_rr: Decimal,
    pub structure_obstacle_price: Option<Decimal>,
    pub obstacle_rr: Option<Decimal>,
    pub triggered_at_ms: Ms,
    pub candle_timestamp_ms: Ms,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pivot { pub index: u64, pub time_ms: Ms, pub price: Decimal, pub broken: bool }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyState {
    pub schema_version: u32,
    pub symbol: String,
    /// Bounded rolling window of the most recent closed candles (capacity WINDOW_CANDLES).
    pub window: VecDeque<Candle>,
    /// Absolute count of closed candles seen (window positions are derived from this).
    pub candle_count: u64,
    pub prev_close: Option<Decimal>,
    /// Running Wilder ATR through the last processed candle.
    pub atr: Option<Decimal>,
    pub atr_seed_sum: Decimal,
    pub atr_seen: u32,
    pub trend: TrendState,
    /// Absolute pivot indices for the valid low/high (spec §6.3).
    pub valid_low_idx: Option<u64>,
    pub valid_high_idx: Option<u64>,
    /// Pivot.index is ABSOLUTE (== candle_count - 1 - right at confirmation time).
    pub swing_highs: Vec<Pivot>,
    pub swing_lows: Vec<Pivot>,
    pub zones: Vec<Zone>,
    pub zones_awaiting_first_tick: Vec<String>,
    pub last_candle_time_ms: Option<Ms>,
    pub last_tick_time_ms: Option<Ms>,
    pub last_price: Option<Decimal>,
    pub missed_touch_count: u64,
}

impl StrategyState {
    pub fn new(symbol: &str, schema_version: u32) -> Self {
        StrategyState {
            schema_version, symbol: symbol.to_string(), window: VecDeque::new(),
            candle_count: 0, prev_close: None, atr: None, atr_seed_sum: Decimal::ZERO,
            atr_seen: 0, trend: TrendState::Unknown, valid_low_idx: None, valid_high_idx: None,
            swing_highs: Vec::new(), swing_lows: Vec::new(), zones: Vec::new(),
            zones_awaiting_first_tick: Vec::new(),
            last_candle_time_ms: None, last_tick_time_ms: None, last_price: None,
            missed_touch_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input { ClosedCandle(Candle), PriceTick(Tick) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneEventKind { Created, Invalidated, Expired, Consumed }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneEvent { pub zone_id: String, pub kind: ZoneEventKind, pub reason: Option<ZoneEventReason> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Output {
    ZoneEvent(ZoneEvent),
    EntrySignal(Signal),
    StateChange { reason: &'static str },
}
```

Tests (append to the file):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> Zone {
        Zone { id: "z".into(), kind: ZoneKind::Demand, low: Decimal::ZERO, high: Decimal::ONE,
            atr: Decimal::ONE, breakout_time_ms: 0, zone_candle_time_ms: 0,
            activated_at_index: 0, state: ZoneState::Active,
            terminal_reason: None, consume_reason: None, attempt_made: false,
            window_high: Decimal::ONE, window_low: Decimal::ZERO }
    }

    #[test]
    fn zone_terminal_rule() {
        let mut z = zone();
        assert!(!z.is_terminal());
        z.state = ZoneState::Consumed; assert!(z.is_terminal());
        z.state = ZoneState::Invalidated; assert!(z.is_terminal());
        z.state = ZoneState::Expired; assert!(z.is_terminal());
    }

    #[test]
    fn state_round_trips_json() {
        let s = StrategyState::new("XRP-USDT", 1);
        let j = serde_json::to_string(&s).unwrap();
        let back: StrategyState = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }
}
```

Add `pub mod types;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features types::`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/types.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): core types, state, zone, signal"
```

---

## Task 6: `step.rs` — dispatch, event ordering, validation

**Files:**
- Create: `process/strategy-analyzer/src/strategy/step.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/step.rs`:
```rust
use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::types::*;
use rust_decimal::Decimal;

pub(crate) const TIMEFRAME_MS: i64 = 4 * 60 * 60 * 1000;

/// Pure step. `state` is borrowed; on error the caller's state is untouched
/// (validate-then-commit: a new state is produced only on success).
pub fn step(
    state: &StrategyState,
    input: Input,
    cfg: &Config,
) -> Result<(StrategyState, Vec<Output>), StrategyError> {
    let mut next = state.clone();
    let outputs = match input {
        Input::PriceTick(t) => on_tick(&mut next, t)?,
        Input::ClosedCandle(c) => on_candle(&mut next, c, cfg)?,
    };
    Ok((next, outputs))
}

fn on_tick(state: &mut StrategyState, t: Tick) -> Result<Vec<Output>, StrategyError> {
    if t.price <= Decimal::ZERO {
        return Err(StrategyError::MalformedTick { price: t.price.to_string() });
    }
    let last = state.last_tick_time_ms.or(state.last_candle_time_ms);
    if let Some(prev) = last {
        if t.time_ms < prev {
            return Err(StrategyError::OutOfOrderTick { got_ms: t.time_ms, last_ms: prev });
        }
    }
    state.last_tick_time_ms = Some(t.time_ms);
    state.last_price = Some(t.price);
    Ok(vec![]) // touch detection added in Task 15
}

fn on_candle(
    state: &mut StrategyState,
    c: Candle,
    _cfg: &Config,
) -> Result<Vec<Output>, StrategyError> {
    if c.high < c.low || c.high <= Decimal::ZERO || c.low <= Decimal::ZERO {
        return Err(StrategyError::MalformedCandle { at_ms: c.open_time_ms, why: "bad range/price" });
    }
    if let Some(prev) = state.last_candle_time_ms {
        if c.open_time_ms == prev { return Err(StrategyError::DuplicateCandle(c.open_time_ms)); }
        if c.open_time_ms < prev { return Err(StrategyError::OutOfOrderCandle { got_ms: c.open_time_ms, prev_ms: prev }); }
        if c.open_time_ms != prev + TIMEFRAME_MS {
            return Err(StrategyError::GappedCandle { expected_ms: prev + TIMEFRAME_MS, got_ms: c.open_time_ms });
        }
    }
    state.window.push_back(c);
    if state.window.len() > crate::strategy::types::WINDOW_CANDLES { state.window.pop_front(); }
    state.candle_count += 1;
    state.last_candle_time_ms = Some(c.open_time_ms);
    Ok(vec![]) // structure/ATR wired in Task 16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle(ms: i64) -> Candle {
        Candle { open_time_ms: ms, open: dec("1.0"), high: dec("1.2"), low: dec("0.9"), close: dec("1.1") }
    }

    #[test]
    fn accepts_contiguous_candles() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let (s, _) = step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS)), &cfg).unwrap();
        assert_eq!(s.window.len(), 2);
    }

    #[test]
    fn rejects_duplicate_and_out_of_order() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(step(&s, Input::ClosedCandle(candle(0)), &cfg), Err(StrategyError::DuplicateCandle(_))));
        assert!(matches!(step(&s, Input::ClosedCandle(candle(-5)), &cfg), Err(StrategyError::OutOfOrderCandle { .. })));
    }

    #[test]
    fn rejects_gap() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS * 2)), &cfg), Err(StrategyError::GappedCandle { .. })));
    }

    #[test]
    fn error_leaves_state_unchanged() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let snapshot = s.clone();
        let _ = step(&s, Input::ClosedCandle(candle(0)), &cfg);
        assert_eq!(s, snapshot);
    }

    #[test]
    fn rejects_malformed_and_out_of_order_tick() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 1, price: dec("-1"), is_bar_open: false }), &cfg), Err(StrategyError::MalformedTick { .. })));
        let (s, _) = step(&s, Input::PriceTick(Tick { time_ms: 10, price: dec("1.0"), is_bar_open: true }), &cfg).unwrap();
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 5, price: dec("1.0"), is_bar_open: false }), &cfg), Err(StrategyError::OutOfOrderTick { .. })));
    }
}
```

Add `pub mod step;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features step::`
Expected: 5 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/step.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): step dispatch, ordering, validation"
```

---

## Task 7: `indicators/atr.rs` — Wilder ATR with in-band `ATR==0`

**Files:**
- Create: `process/strategy-analyzer/src/strategy/indicators/mod.rs`
- Create: `process/strategy-analyzer/src/strategy/indicators/atr.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

`src/strategy/indicators/mod.rs`:
```rust
pub mod atr;
```

`src/strategy/indicators/atr.rs`:
```rust
use crate::strategy::decimal::work;
use rust_decimal::Decimal;

/// True range; the first candle uses `high - low` (no previous close).
pub fn true_range(prev_close: Option<Decimal>, high: Decimal, low: Decimal) -> Decimal {
    match prev_close {
        None => high - low,
        Some(pc) => (high - low).max((high - pc).abs()).max((low - pc).abs()),
    }
}

/// Wilder ATR seeded with the simple mean of the first `period` true ranges.
/// Returns `None` until warm-up is complete.
/// Incremental Wilder ATR. State stores its fields directly (`atr`, `atr_seed_sum`, `atr_seen`).
pub struct AtrState { pub value: Option<Decimal>, pub seed_sum: Decimal, pub seen: u32, pub period: u32 }

impl AtrState {
    pub fn new(period: u32) -> Self { Self { value: None, seed_sum: Decimal::ZERO, seen: 0, period } }

    /// Feed one candle's TR inputs; returns the running ATR once warm-up completes.
    pub fn update(&mut self, prev_close: Option<Decimal>, high: Decimal, low: Decimal) -> Option<Decimal> {
        let tr = true_range(prev_close, high, low);
        self.seen += 1;
        match self.value {
            None => {
                self.seed_sum += tr;
                if self.seen == self.period {
                    self.value = Some(work(self.seed_sum / Decimal::from(self.period as u64)));
                }
            }
            Some(prev) => {
                self.value = Some(work((prev * Decimal::from(self.period as u64 - 1) + tr) / Decimal::from(self.period as u64)));
            }
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    #[test]
    fn first_candle_tr_is_high_minus_low() {
        assert_eq!(true_range(None, dec("10"), dec("8")), dec("2"));
    }

    #[test]
    fn warmup_returns_none() {
        let mut a = AtrState::new(14);
        for _ in 0..13 { assert_eq!(a.update(None, dec("2"), dec("0.5")), None); }
        assert!(a.update(None, dec("2"), dec("0.5")).is_some());
    }

    #[test]
    fn flat_series_yields_zero_atr_not_error() {
        let mut a = AtrState::new(14);
        let mut v = None;
        for _ in 0..20 { v = a.update(Some(dec("1")), dec("1"), dec("1")); }
        assert_eq!(v, Some(Decimal::ZERO));
    }

    #[test]
    fn constant_range_seeds_to_that_range() {
        let mut a = AtrState::new(14);
        let mut v = None;
        for _ in 0..20 { v = a.update(Some(dec("1")), dec("2"), dec("0.5")); }
        assert_eq!(v, Some(dec("1.5")));
    }
}
```

Add `pub mod indicators;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features atr::`
Expected: 4 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/indicators process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): Wilder ATR with warm-up and ATR==0 handling"
```

---

## Task 8: `structure.rs` — confirmed swing pivots

**Files:**
- Create: `process/strategy-analyzer/src/strategy/structure.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/structure.rs`:
```rust
use crate::strategy::types::{Candle, Pivot};

/// Strict on the right, `>=` on the left: a plateau resolves to its rightmost member,
/// and a pivot is available only after `right` candles close.
pub fn is_swing_high(c: &[Candle], i: usize, left: usize, right: usize) -> bool {
    if i < left || i + right >= c.len() { return false; }
    let h = c[i].high;
    (1..=right).all(|k| h > c[i + k].high) && (1..=left).all(|k| h >= c[i - k].high)
}

pub fn is_swing_low(c: &[Candle], i: usize, left: usize, right: usize) -> bool {
    if i < left || i + right >= c.len() { return false; }
    let l = c[i].low;
    (1..=right).all(|k| l < c[i + k].low) && (1..=left).all(|k| l <= c[i - k].low)
}

/// Pivot newly confirmed by appending the candle at the last index.
pub fn newly_confirmed_high(c: &[Candle], left: usize, right: usize) -> Option<Pivot> {
    if c.len() <= right { return None; }
    let i = c.len() - 1 - right;
    if is_swing_high(c, i, left, right) {
        Some(Pivot { index: i as u64, time_ms: c[i].open_time_ms, price: c[i].high, broken: false })
    } else { None }
}

pub fn newly_confirmed_low(c: &[Candle], left: usize, right: usize) -> Option<Pivot> {
    if c.len() <= right { return None; }
    let i = c.len() - 1 - right;
    if is_swing_low(c, i, left, right) {
        Some(Pivot { index: i as u64, time_ms: c[i].open_time_ms, price: c[i].low, broken: false })
    } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn c(high: &str, low: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(low), high: dec(high), low: dec(low), close: dec(low) }
    }

    #[test]
    fn swing_high_confirmed_only_after_right_candles_close() {
        // highs: 1, 0.5, 2, 0.5, 0.4 -> index 2 is the swing high
        let candles = vec![c("1","0.5"), c("0.5","0.2"), c("2","1.5"), c("0.5","0.2"), c("0.4","0.1")];
        assert!(newly_confirmed_high(&candles[..4], 2, 2).is_none());
        let p = newly_confirmed_high(&candles[..5], 2, 2).unwrap();
        assert_eq!(p.index, 2);
        assert_eq!(p.price, dec("2"));
    }

    #[test]
    fn plateau_resolves_to_rightmost_member() {
        // highs: 2,2,2,1,0.5 -> index 2 is the only eligible swing high (strict right)
        let candles = vec![c("2","1"), c("2","1"), c("2","1"), c("1","0.5"), c("0.5","0.2")];
        let p = newly_confirmed_high(&candles, 2, 2).unwrap();
        assert_eq!(p.index, 2);
    }

    #[test]
    fn finds_swing_low() {
        // lows: 1.0, 1.2, 0.3, 1.1, 1.0 -> index 2
        let candles = vec![c("1.5","1.0"), c("1.5","1.2"), c("1.5","0.3"), c("1.5","1.1"), c("1.5","1.0")];
        let p = newly_confirmed_low(&candles, 2, 2).unwrap();
        assert_eq!(p.index, 2);
        assert_eq!(p.price, dec("0.3"));
    }
}
```

Add `pub mod structure;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features structure::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/structure.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): confirmed swing pivots with delay + tie rule"
```

---

## Task 9: `structure.rs` — BOS detection (no look-ahead)

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/structure.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Append to `structure.rs`:
```rust
use crate::strategy::config::Config;
use crate::strategy::types::{StrategyState, TrendState};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BosKind { Bullish, Bearish }

/// Detect a BOS on the just-closed candle using the §6.3 eligibility table.
/// `atr_before` must be the ATR through the *previous* candle (no look-ahead).
/// Returns the broken kind and the absolute index of the broken level.
pub fn detect_bos(state: &StrategyState, atr_before: Decimal, cfg: &Config) -> Option<(BosKind, u64)> {
    let close = state.window.back()?.close;
    let dist = cfg.bos_min_distance_atr * atr_before;
    let recent_high = state.swing_highs.iter().filter(|p| !p.broken).max_by_key(|p| p.index);
    let recent_low = state.swing_lows.iter().filter(|p| !p.broken).max_by_key(|p| p.index);
    let high_price = |idx: u64| state.swing_highs.iter().find(|p| p.index == idx).map(|p| p.price);
    let low_price = |idx: u64| state.swing_lows.iter().find(|p| p.index == idx).map(|p| p.price);

    // Bullish candidate: in Bearish only valid_high (reversal); otherwise the most recent unbroken high.
    let bull = match state.trend {
        TrendState::Bearish => state.valid_high_idx.and_then(|i| high_price(i).map(|p| (p, i))),
        _ => recent_high.map(|h| (h.price, h.index)),
    };
    if let Some((price, idx)) = bull {
        if close >= price + dist { return Some((BosKind::Bullish, idx)); }
    }
    // Bearish candidate: in Bullish only valid_low (reversal); otherwise the most recent unbroken low.
    let bear = match state.trend {
        TrendState::Bullish => state.valid_low_idx.and_then(|i| low_price(i).map(|p| (p, i))),
        _ => recent_low.map(|l| (l.price, l.index)),
    };
    if let Some((price, idx)) = bear {
        if close <= price - dist { return Some((BosKind::Bearish, idx)); }
    }
    None
}

#[cfg(test)]
mod bos_tests {
    use super::*;
    use crate::strategy::types::{Candle, Pivot, StrategyState, TrendState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    fn state_with(close: &str, highs: Vec<(u64, &str)>, lows: Vec<(u64, &str)>, trend: TrendState, vl: Option<u64>, vh: Option<u64>) -> StrategyState {
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.window.push_back(Candle { open_time_ms: 0, open: dec(close), high: dec(close), low: dec(close), close: dec(close) });
        s.swing_highs = highs.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: dec(p), broken: false }).collect();
        s.swing_lows = lows.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: dec(p), broken: false }).collect();
        s.trend = trend; s.valid_low_idx = vl; s.valid_high_idx = vh;
        s
    }

    #[test]
    fn bullish_bos_reports_most_recent_cleared_high() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = state_with("2.6", vec![(0, "2.0"), (1, "2.3")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), Some((BosKind::Bullish, 1)));
    }

    #[test]
    fn bos_equality_boundary_fires() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // close == price + dist exactly (2.0 + 0.1)
        let s = state_with("2.1", vec![(0, "2.0")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), Some((BosKind::Bullish, 0)));
    }

    #[test]
    fn bullish_trend_does_not_reverse_on_arbitrary_low() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // In Bullish, only valid_low triggers a bearish reversal; an unbroken low must NOT fire.
        let s = state_with("1.0", vec![], vec![(0, "0.5")], TrendState::Bullish, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), None);
    }

    #[test]
    fn no_bos_when_distance_not_met() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = state_with("2.0", vec![(0, "2.0")], vec![], TrendState::Unknown, None, None);
        assert_eq!(detect_bos(&s, dec("1.0"), &cfg), None);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features bos_tests`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/structure.rs
git commit -m "feat(strategy-analyzer): BOS detection against pre-breakout ATR"
```

---

## Task 10: `structure.rs` — trend state machine + valid high/low

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/structure.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Append:
```rust
// `StrategyState` and `TrendState` are already in scope from Task 9.

/// Apply a BOS: mark EVERY cleared level broken, set trend + valid low/high per §6.3.
/// If no valid low/high can be assigned, the trend is left unchanged (state stays Unknown).
pub fn apply_bos(state: &mut StrategyState, kind: BosKind, broken_idx: u64) {
    let breakout = state.candle_count.saturating_sub(1);
    let close = state.window.back().map(|c| c.close).unwrap_or(Decimal::ZERO);
    match kind {
        BosKind::Bullish => {
            for h in state.swing_highs.iter_mut() {
                if !h.broken && h.price <= close { h.broken = true; }
            }
            let vl = state.swing_lows.iter()
                .filter(|l| l.index > broken_idx && l.index < breakout).map(|l| l.index).max()
                .or_else(|| state.swing_lows.iter().filter(|l| l.index < broken_idx).map(|l| l.index).max());
            state.valid_low_idx = vl;
            state.trend = if vl.is_some() { TrendState::Bullish } else { TrendState::Unknown };
        }
        BosKind::Bearish => {
            for l in state.swing_lows.iter_mut() {
                if !l.broken && l.price >= close { l.broken = true; }
            }
            let vh = state.swing_highs.iter()
                .filter(|h| h.index > broken_idx && h.index < breakout).map(|h| h.index).max()
                .or_else(|| state.swing_highs.iter().filter(|h| h.index < broken_idx).map(|h| h.index).max());
            state.valid_high_idx = vh;
            state.trend = if vh.is_some() { TrendState::Bearish } else { TrendState::Unknown };
        }
    }
}

#[cfg(test)]
mod trend_tests {
    use super::*;
    use crate::strategy::types::{Candle, Pivot, StrategyState, TrendState};
    use rust_decimal::Decimal;

    fn st(count: u64, close: i64, highs: Vec<(u64, i64)>, lows: Vec<(u64, i64)>) -> StrategyState {
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.candle_count = count;
        s.window.push_back(Candle { open_time_ms: 0, open: Decimal::from(close), high: Decimal::from(close), low: Decimal::from(close), close: Decimal::from(close) });
        s.swing_highs = highs.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: Decimal::from(p), broken: false }).collect();
        s.swing_lows = lows.into_iter().map(|(i, p)| Pivot { index: i, time_ms: 0, price: Decimal::from(p), broken: false }).collect();
        s
    }

    #[test]
    fn bullish_bos_marks_all_cleared_highs_and_sets_valid_low() {
        let mut s = st(6, 5, vec![(1, 2), (2, 3)], vec![(0, 1), (3, 2)]);
        apply_bos(&mut s, BosKind::Bullish, 2);
        assert_eq!(s.trend, TrendState::Bullish);
        assert!(s.swing_highs.iter().all(|h| h.broken)); // both cleared in ONE BOS
        assert_eq!(s.valid_low_idx, Some(3)); // low after the broken high, before breakout
    }

    #[test]
    fn trend_stays_unknown_when_no_valid_low() {
        let mut s = st(6, 5, vec![(1, 2)], vec![]);
        apply_bos(&mut s, BosKind::Bullish, 1);
        assert_eq!(s.trend, TrendState::Unknown);
        assert_eq!(s.valid_low_idx, None);
        assert!(s.swing_highs[0].broken); // level still marked broken
    }

    #[test]
    fn valid_low_falls_back_to_before_broken_high() {
        let mut s = st(6, 5, vec![(3, 9)], vec![(1, 5)]);
        apply_bos(&mut s, BosKind::Bullish, 3);
        assert_eq!(s.valid_low_idx, Some(1));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features trend_tests`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/structure.rs
git commit -m "feat(strategy-analyzer): trend state machine + valid high/low"
```

---

## Task 11: `zones.rs` — creation, mitigation, size limits

**Files:**
- Create: `process/strategy-analyzer/src/strategy/zones.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/zones.rs`:
```rust
use crate::strategy::config::Config;
use crate::strategy::structure::BosKind;
use crate::strategy::types::{Candle, Zone, ZoneKind, ZoneState};
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Build a zone after a BOS, from the bounded candle window. `Err(reason)` is a rejection tag.
/// `activated_at_index` is set by the caller (it owns the absolute candle counter).
pub fn create_zone(window: &VecDeque<Candle>, kind: BosKind, atr: Decimal, symbol: &str, cfg: &Config) -> Result<Zone, &'static str> {
    if atr <= Decimal::ZERO { return Err("atr_zero"); }
    let n = window.len();
    if n == 0 { return Err("no_candles"); }
    let at = |i: usize| window.get(i).unwrap();
    let breakout = n - 1;
    let lookback_start = breakout.saturating_sub(cfg.zone_search_lookback as usize);

    let mut chosen = None;
    for i in (lookback_start..breakout).rev() {
        let c = at(i);
        let opposite = match kind { BosKind::Bullish => c.close < c.open, BosKind::Bearish => c.close > c.open };
        if opposite { chosen = Some(i); break; }
    }
    let zi = chosen.ok_or("no_opposite_candle")?;

    let body = (at(breakout).close - at(breakout).open).abs();
    if body < cfg.impulse_body_threshold_atr * atr { return Err("weak_impulse"); }

    let zc = at(zi);
    let (low, high) = (zc.low, zc.high);
    let height_atr = (high - low) / atr;
    if height_atr < cfg.min_zone_height_atr { return Err("zone_too_small"); }
    if height_atr > cfg.max_zone_height_atr { return Err("zone_too_large"); }

    let start = (zi + 1 + cfg.mitigation_grace_candles as usize).min(breakout);
    for i in start..breakout {
        let c = at(i);
        let mitigated = match kind { BosKind::Bullish => c.low <= high, BosKind::Bearish => c.high >= low };
        if mitigated { return Err("mitigated"); }
    }

    let zone_kind = match kind { BosKind::Bullish => ZoneKind::Demand, BosKind::Bearish => ZoneKind::Supply };
    let kind_tag = match zone_kind { ZoneKind::Demand => "demand", ZoneKind::Supply => "supply" };
    let bc = at(breakout);
    let id = format!("{}:{}:{}:{}", symbol, bc.open_time_ms, zc.open_time_ms, kind_tag);
    Ok(Zone {
        id, kind: zone_kind, low, high, atr,
        breakout_time_ms: bc.open_time_ms,
        zone_candle_time_ms: zc.open_time_ms,
        activated_at_index: 0, // set by caller (absolute index)
        state: ZoneState::Active, terminal_reason: None, consume_reason: None, attempt_made: false,
        window_high: bc.high, window_low: bc.low, // window starts at the breakout candle
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle(o: &str, h: &str, l: &str, c: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(o), high: dec(h), low: dec(l), close: dec(c) }
    }
    fn win(v: Vec<Candle>) -> VecDeque<Candle> { v.into() }

    // index 1 is the bearish demand candle (low 0.98 / high 1.06); index 4 is the breakout.
    fn base() -> Vec<Candle> {
        vec![
            candle("1.00","1.05","0.95","1.00"),
            candle("1.02","1.06","0.98","0.99"),
            candle("1.00","1.05","0.98","1.04"),
            candle("1.04","1.10","1.07","1.09"), // low 1.07 > zone high 1.06 -> not mitigated
            candle("1.09","1.25","1.08","1.24"), // breakout, body 0.15
        ]
    }

    #[test]
    fn creates_demand_zone_with_window_starting_at_breakout() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = create_zone(&win(base()), BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap();
        assert_eq!(z.kind, ZoneKind::Demand);
        assert_eq!(z.low, dec("0.98"));
        assert_eq!(z.high, dec("1.06"));
        assert_eq!(z.window_high, dec("1.25")); // breakout candle high (included)
        assert_eq!(z.window_low, dec("1.08"));  // breakout candle low (included)
    }

    #[test]
    fn rejects_when_mitigated() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut c = base();
        c[3] = candle("1.04","1.10","1.05","1.09"); // low 1.05 <= zone high 1.06 -> mitigated
        assert_eq!(create_zone(&win(c), BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap_err(), "mitigated");
    }

    #[test]
    fn rejects_atr_zero() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(create_zone(&win(base()), BosKind::Bullish, dec("0"), "XRP-USDT", &cfg).unwrap_err(), "atr_zero");
    }

    // Breakout body is large (0.34) so the impulse gate never fires before the height bound.
    fn base_big_breakout() -> Vec<Candle> {
        vec![
            candle("1.00","1.05","0.95","1.00"),
            candle("1.02","1.06","0.98","0.99"), // bearish demand candle: h 1.06, l 0.98
            candle("1.02","1.07","1.00","1.05"),
            candle("1.05","1.12","1.08","1.11"), // low 1.08 > 1.06 -> not mitigated
            candle("1.11","1.46","1.10","1.45"), // breakout: body 0.34
        ]
    }

    #[test]
    fn rejects_zone_too_small_and_too_large() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // impulse 1.5*0.2 = 0.30 <= body 0.34 (passes); height 0.08/0.2 = 0.4 < 0.5 -> too small
        assert_eq!(create_zone(&win(base_big_breakout()), BosKind::Bullish, dec("0.2"), "XRP-USDT", &cfg).unwrap_err(), "zone_too_small");
        // impulse 1.5*0.02 = 0.03 <= body 0.34 (passes); height 0.08/0.02 = 4.0 > 3.0 -> too large
        assert_eq!(create_zone(&win(base_big_breakout()), BosKind::Bullish, dec("0.02"), "XRP-USDT", &cfg).unwrap_err(), "zone_too_large");
    }

    #[test]
    fn creates_supply_zone_on_bearish_bos() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // The most recent bullish candle before the breakout is index 3 (height 0.10 -> 1.0 ATR, in bounds).
        let v = vec![
            candle("1.00","1.05","0.95","1.00"), // 0
            candle("1.00","1.03","0.98","1.02"), // 1
            candle("1.02","1.05","1.00","1.04"), // 2
            candle("1.06","1.14","1.04","1.12"), // 3 bullish supply candle: h 1.14, l 1.04
            candle("1.12","1.13","0.93","0.94"), // 4 bearish breakout: body 0.18
        ];
        let z = create_zone(&win(v), BosKind::Bearish, dec("0.1"), "XRP-USDT", &cfg).unwrap();
        assert_eq!(z.kind, ZoneKind::Supply);
        assert_eq!(z.low, dec("1.04"));
        assert_eq!(z.high, dec("1.14"));
    }
}
```

Add `pub mod zones;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features zones::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/zones.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): zone creation, mitigation, size limits"
```

---

## Task 12: `zones.rs` — lifecycle (expire, invalidate, trend flip)

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/zones.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Append to `zones.rs`:
```rust
use crate::strategy::types::{TerminalReason, TrendState, ZoneEvent, ZoneEventKind, ZoneEventReason, ZoneState as ZS};

/// Expiry + close-beyond-edge invalidation for the just-closed candle.
pub fn update_lifecycle(zones: &mut [Zone], candle: &Candle, idx: u64, expiry_candles: u32) -> Vec<ZoneEvent> {
    let mut events = Vec::new();
    for z in zones.iter_mut() {
        if z.state != ZS::Active { continue; }
        if idx.saturating_sub(z.activated_at_index) >= expiry_candles as u64 {
            z.state = ZS::Expired; z.terminal_reason = Some(TerminalReason::Expired);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Expired, reason: Some(ZoneEventReason::Terminal(TerminalReason::Expired)) });
            continue;
        }
        let invalid = match z.kind { ZoneKind::Demand => candle.close < z.low, ZoneKind::Supply => candle.close > z.high };
        if invalid {
            z.state = ZS::Invalidated; z.terminal_reason = Some(TerminalReason::CloseBeyondEdge);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Invalidated, reason: Some(ZoneEventReason::Terminal(TerminalReason::CloseBeyondEdge)) });
        }
    }
    events
}

/// On a trend flip, invalidate every active zone in the now-wrong direction.
pub fn invalidate_on_flip(zones: &mut [Zone], trend: TrendState) -> Vec<ZoneEvent> {
    let mut events = Vec::new();
    for z in zones.iter_mut() {
        if z.state != ZS::Active { continue; }
        let wrong = match trend {
            TrendState::Bullish => z.kind == ZoneKind::Supply,
            TrendState::Bearish => z.kind == ZoneKind::Demand,
            TrendState::Unknown => false,
        };
        if wrong {
            z.state = ZS::Invalidated; z.terminal_reason = Some(TerminalReason::TrendFlip);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Invalidated, reason: Some(ZoneEventReason::Terminal(TerminalReason::TrendFlip)) });
        }
    }
    events
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::strategy::types::Candle;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn zone(kind: ZoneKind) -> Zone {
        Zone { id: "z".into(), kind, low: dec("1.0"), high: dec("1.1"), atr: dec("0.05"),
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0,
            state: ZS::Active, terminal_reason: None, consume_reason: None, attempt_made: false,
            window_high: dec("1.1"), window_low: dec("1.0") }
    }
    fn c(open: &str, high: &str, low: &str, close: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(open), high: dec(high), low: dec(low), close: dec(close) }
    }

    #[test]
    fn expiry_uses_expired_terminal_reason() {
        let mut zs = vec![zone(ZoneKind::Demand)];
        let ev = update_lifecycle(&mut zs, &c("1","1","1","1"), 30, 30);
        assert_eq!(zs[0].state, ZS::Expired);
        assert_eq!(zs[0].terminal_reason, Some(TerminalReason::Expired));
        assert_eq!(ev.len(), 1);
    }

    #[test]
    fn close_below_low_uses_close_beyond_edge() {
        let mut zs = vec![zone(ZoneKind::Demand)];
        update_lifecycle(&mut zs, &c("1.0","1.0","0.9","0.95"), 1, 30);
        assert_eq!(zs[0].state, ZS::Invalidated);
        assert_eq!(zs[0].terminal_reason, Some(TerminalReason::CloseBeyondEdge));
    }

    #[test]
    fn trend_flip_uses_trend_flip_reason() {
        let mut zs = vec![zone(ZoneKind::Demand), zone(ZoneKind::Supply)];
        invalidate_on_flip(&mut zs, TrendState::Bullish);
        assert_eq!(zs[0].state, ZS::Active);
        assert_eq!(zs[1].state, ZS::Invalidated);
        assert_eq!(zs[1].terminal_reason, Some(TerminalReason::TrendFlip));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features lifecycle_tests`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/zones.rs
git commit -m "feat(strategy-analyzer): zone lifecycle + trend-flip invalidation"
```

---

## Task 13: `zones.rs` — nearest-zone selection

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/zones.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Append:
```rust
use crate::strategy::decimal::round_to;
use crate::strategy::types::TrendState as TS;

/// Active zone nearest to `price` in the trend direction (distance to the proximal edge).
pub fn select_zone<'a>(zones: &'a [Zone], trend: TS, price: Decimal) -> Option<&'a Zone> {
    let want = match trend { TS::Bullish => ZoneKind::Demand, TS::Bearish => ZoneKind::Supply, TS::Unknown => return None };
    zones.iter()
        .filter(|z| z.state == ZS::Active && z.kind == want)
        .min_by_key(|z| {
            let d = match want { ZoneKind::Demand => (z.high - price).abs(), ZoneKind::Supply => (z.low - price).abs() };
            round_to(d, 8) // deterministic ordering key
        })
}

#[cfg(test)]
mod select_tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn zone(id: &str, low: &str, high: &str) -> Zone {
        Zone { id: id.into(), kind: ZoneKind::Demand, low: dec(low), high: dec(high), atr: dec("0.1"),
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0, state: ZS::Active,
            terminal_reason: None, consume_reason: None, attempt_made: false,
            window_high: dec(high), window_low: dec(low) }
    }

    #[test]
    fn picks_nearest_demand_zone() {
        let zs = vec![zone("far","0.5","0.6"), zone("near","1.0","1.05")];
        assert_eq!(select_zone(&zs, TS::Bullish, dec("1.2")).unwrap().id, "near");
    }

    #[test]
    fn unknown_trend_selects_none() {
        let zs = vec![zone("a","1.0","1.1")];
        assert!(select_zone(&zs, TS::Unknown, dec("1.2")).is_none());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features select_tests`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/zones.rs
git commit -m "feat(strategy-analyzer): nearest-zone selection"
```

---

## Task 14: `signals.rs` — levels, obstacle, target modes

**Files:**
- Create: `process/strategy-analyzer/src/strategy/signals.rs`
- Modify: `process/strategy-analyzer/src/strategy/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/strategy/signals.rs`:
```rust
use crate::strategy::config::{Config, TargetMode};
use crate::strategy::decimal::round_price;
use crate::strategy::types::{Direction, Pivot, Zone, ZoneKind};
use rust_decimal::Decimal;

pub struct Levels {
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub gross_rr: Decimal,
    pub obstacle: Option<Decimal>,
    pub obstacle_rr: Option<Decimal>,
}

pub fn obstacle_for_long(trigger: Decimal, swing_highs: &[Pivot], active_supply: &[Zone], window_high: Decimal) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(swing_highs.iter().filter(|p| !p.broken && p.price > trigger).map(|p| p.price));
    c.extend(active_supply.iter().filter(|z| z.low > trigger).map(|z| z.low));
    if window_high > trigger { c.push(window_high); } // window includes the breakout candle
    c.into_iter().min()
}

pub fn obstacle_for_short(trigger: Decimal, swing_lows: &[Pivot], active_demand: &[Zone], window_low: Decimal) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(swing_lows.iter().filter(|p| !p.broken && p.price < trigger).map(|p| p.price));
    c.extend(active_demand.iter().filter(|z| z.high < trigger).map(|z| z.high));
    if window_low < trigger { c.push(window_low); }
    c.into_iter().max()
}

/// Rounded levels + the mode's gate. `None` = veto.
pub fn compute_levels(
    direction: Direction,
    zone: &Zone,
    trigger: Decimal,
    obstacle: Option<Decimal>,
    price_scale: u32,
    cfg: &Config,
) -> Option<Levels> {
    let t = round_price(trigger, price_scale);
    let buffer = cfg.stop_buffer_atr * zone.atr;
    let (stop_raw, risk) = match direction {
        Direction::Long => (zone.low - buffer, t - (zone.low - buffer)),
        Direction::Short => (zone.high + buffer, (zone.high + buffer) - t),
    };
    if risk <= Decimal::ZERO { return None; }
    let stop = round_price(stop_raw, price_scale);
    let risk = match direction { Direction::Long => t - stop, Direction::Short => stop - t };
    if risk <= Decimal::ZERO { return None; }

    let obstacle_rounded = obstacle.map(|o| round_price(o, price_scale));
    let obstacle_rr = obstacle_rounded.map(|o| match direction {
        Direction::Long => (o - t) / risk,
        Direction::Short => (t - o) / risk,
    });

    let (tp, gross_rr) = match cfg.target_mode {
        TargetMode::FixedR => {
            // Gate = obstacle veto only (no tautological gross gate).
            if let Some(rr) = obstacle_rr {
                if rr < cfg.target_r { return None; }
            } else if !cfg.allow_no_obstacle {
                return None;
            }
            let tp = match direction { Direction::Long => t + cfg.target_r * risk, Direction::Short => t - cfg.target_r * risk };
            (round_price(tp, price_scale), cfg.target_r)
        }
        TargetMode::Structural => {
            let o = match obstacle_rounded {
                Some(o) => o,
                None => {
                    if !cfg.allow_no_obstacle { return None; }
                    let tp = match direction { Direction::Long => t + cfg.target_r * risk, Direction::Short => t - cfg.target_r * risk };
                    return Some(Levels { stop_loss: stop, take_profit: round_price(tp, price_scale), gross_rr: cfg.target_r, obstacle: None, obstacle_rr: None });
                }
            };
            let buf = round_price(cfg.structural_buffer_atr * zone.atr, price_scale);
            let tp = match direction { Direction::Long => o - buf, Direction::Short => o + buf };
            let tp = round_price(tp, price_scale);
            let gross = match direction { Direction::Long => (tp - t) / risk, Direction::Short => (t - tp) / risk };
            if gross < cfg.min_gross_rr { return None; }
            (tp, gross)
        }
    };
    // Recompute gross from the rounded levels that go into the payload.
    let gross = match direction { Direction::Long => (tp - t) / risk, Direction::Short => (t - tp) / risk };
    Some(Levels { stop_loss: stop, take_profit: tp, gross_rr: gross, obstacle: obstacle_rounded, obstacle_rr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::ZoneState;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn zone(kind: ZoneKind, low: &str, high: &str) -> Zone {
        Zone { id: "z".into(), kind, low: dec(low), high: dec(high), atr: dec("0.1"),
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0,
            state: ZoneState::Active, terminal_reason: None, consume_reason: None, attempt_made: false,
            window_high: dec(high), window_low: dec(low) }
    }

    #[test]
    fn long_levels_and_obstacle_veto() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let ok = compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.20")), 2, &cfg).unwrap();
        assert_eq!(ok.stop_loss, dec("1.95"));
        assert_eq!(ok.take_profit, dec("2.15"));
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.05")), 2, &cfg).is_none());
    }

    #[test]
    fn short_sign_is_correct() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Supply, "1.90", "1.94");
        let s = compute_levels(Direction::Short, &z, dec("1.90"), Some(dec("1.70")), 2, &cfg).unwrap();
        assert_eq!(s.stop_loss, dec("1.95"));
        assert_eq!(s.take_profit, dec("1.75"));
    }

    #[test]
    fn coarse_price_scale_gate_uses_rounded_levels() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.960", "2.000");
        // trigger 2.001 rounds to 2.00 at scale 2; stop 1.95; risk 0.05; TP = 2.00 + 3*0.05 = 2.15
        let l = compute_levels(Direction::Long, &z, dec("2.001"), Some(dec("2.500")), 2, &cfg).unwrap();
        assert_eq!(l.stop_loss, dec("1.95"));
        assert_eq!(l.take_profit, dec("2.15"));
        assert_eq!(l.gross_rr, dec("3.0")); // derived from the ROUNDED levels, against a known value
    }

    #[test]
    fn structural_mode_targets_the_obstacle() {
        let cfg = Config { target_mode: TargetMode::Structural, ..Config::default_for(&["XRP-USDT"]) };
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        // obstacle 2.50, buffer = 0.1*atr(0.1) = 0.01 -> TP = 2.49; risk 0.05 -> gross 9.8
        let l = compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.50")), 2, &cfg).unwrap();
        assert_eq!(l.take_profit, dec("2.49"));
        assert_eq!(l.gross_rr, dec("9.8"));
    }

    #[test]
    fn no_obstacle_respects_allow_no_obstacle() {
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let allow = Config { allow_no_obstacle: true, ..Config::default_for(&["XRP-USDT"]) };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &allow).is_some());
        let deny = Config { allow_no_obstacle: false, ..Config::default_for(&["XRP-USDT"]) };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &deny).is_none());
    }
}
```

Add `pub mod signals;` to `strategy/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features signals::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/signals.rs process/strategy-analyzer/src/strategy/mod.rs
git commit -m "feat(strategy-analyzer): levels, obstacle check, target modes"
```

---

## Task 15: `signals.rs` — first-touch trigger + `missed_touch`, wired into `step`

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/types.rs` (add one state field)
- Modify: `process/strategy-analyzer/src/strategy/signals.rs`
- Modify: `process/strategy-analyzer/src/strategy/step.rs`

- [ ] **Step 1: Confirm first-observation tracking exists**

`StrategyState.zones_awaiting_first_tick` is already defined in Task 5 and initialized in `new()`. No change is needed here; it is used below.

- [ ] **Step 2: Write the failing tests + implementation**

Append to `signals.rs`:
```rust
use crate::strategy::types::*;
use crate::strategy::zones::select_zone;

pub fn is_first_touch(direction: Direction, zone: &Zone, prev_price: Decimal, price: Decimal) -> bool {
    match direction {
        Direction::Long => prev_price > zone.high && price <= zone.high,
        Direction::Short => prev_price < zone.low && price >= zone.low,
    }
}

pub fn is_missed_touch(direction: Direction, zone: &Zone, price: Decimal) -> bool {
    match direction { Direction::Long => price <= zone.high, Direction::Short => price >= zone.low }
}

fn direction_for(trend: TrendState) -> Option<Direction> {
    match trend { TrendState::Bullish => Some(Direction::Long), TrendState::Bearish => Some(Direction::Short), TrendState::Unknown => None }
}

/// Entry attempt on a tick. Mutates `state`, returns outputs.
/// `tick_time_ms` is the touching tick's timestamp (becomes `triggered_at_ms`).
pub fn try_entry(state: &mut StrategyState, prev_price: Option<Decimal>, tick_time_ms: Ms, price: Decimal, cfg: &Config) -> Vec<Output> {
    let mut out = Vec::new();
    let dir = match direction_for(state.trend) { Some(d) => d, None => return out };
    let atr = match state.atr { Some(a) if a > Decimal::ZERO => a, _ => return out };

    // Select the nearest active zone in the trend direction (immutably first).
    let zone_snapshot = match select_zone(&state.zones, state.trend, price) { Some(z) => z.clone(), None => return out };

    // Missed touch: the first observed price after activation is already inside/beyond.
    if state.zones_awaiting_first_tick.iter().any(|id| id == &zone_snapshot.id) {
        state.zones_awaiting_first_tick.retain(|id| id != &zone_snapshot.id);
        if is_missed_touch(dir, &zone_snapshot, price) {
            if let Some(z) = state.zones.iter_mut().find(|z| z.id == zone_snapshot.id) {
                z.state = ZoneState::Consumed; z.consume_reason = Some(ConsumeReason::MissedTouch);
            }
            state.missed_touch_count += 1;
            out.push(Output::ZoneEvent(ZoneEvent { zone_id: zone_snapshot.id.clone(), kind: ZoneEventKind::Consumed, reason: Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch)) }));
            return out;
        }
    }

    let prev = match prev_price { Some(p) => p, None => return out };
    if !is_first_touch(dir, &zone_snapshot, prev, price) { return out; }

    // Obstacle: swing levels + active opposite zones + the zone's window extremes (includes breakout candle).
    let (swing_highs, swing_lows) = (&state.swing_highs, &state.swing_lows);
    let (active_supply, active_demand): (Vec<Zone>, Vec<Zone>) = (
        state.zones.iter().filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Supply).cloned().collect(),
        state.zones.iter().filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Demand).cloned().collect(),
    );
    let obstacle = match dir {
        Direction::Long => obstacle_for_long(price, swing_highs, &active_supply, zone_snapshot.window_high),
        Direction::Short => obstacle_for_short(price, swing_lows, &active_demand, zone_snapshot.window_low),
    };
    let price_scale = cfg.price_scale.get(&state.symbol).copied().unwrap_or(4);
    let levels = match compute_levels(dir, &zone_snapshot, price, obstacle, price_scale, cfg) {
        Some(l) => l, None => return out, // veto; zone stays Active
    };
    let zone_height_atr = (zone_snapshot.high - zone_snapshot.low) / atr;
    let trigger = crate::strategy::decimal::round_price(price, price_scale);
    let signal = Signal {
        schema_version: 1, event_type: "strategy.entry_signal".to_string(),
        signal_id: format!("{}:0", zone_snapshot.id),
        strategy: "supply_demand_4h".to_string(), strategy_version: "0.1.0".to_string(),
        config_version: crate::strategy::config::config_version(&cfg.canonical_json()),
        symbol: state.symbol.clone(), market: "margin".to_string(), direction: dir,
        timeframe: "4hour".to_string(), trend_state: state.trend,
        zone_id: zone_snapshot.id.clone(), zone_low: zone_snapshot.low, zone_high: zone_snapshot.high,
        zone_height_atr, atr, trigger_price: trigger, reference_entry_price: trigger,
        stop_loss: levels.stop_loss, take_profit: levels.take_profit,
        target_mode: match cfg.target_mode { TargetMode::FixedR => "fixed_r", TargetMode::Structural => "structural" }.to_string(),
        target_r: cfg.target_r, min_gross_rr: cfg.min_gross_rr, gross_rr: levels.gross_rr,
        structure_obstacle_price: levels.obstacle, obstacle_rr: levels.obstacle_rr,
        triggered_at_ms: tick_time_ms,
        candle_timestamp_ms: state.last_candle_time_ms.unwrap_or(0),
    };
    if let Some(z) = state.zones.iter_mut().find(|z| z.id == zone_snapshot.id) {
        z.state = ZoneState::Consumed; z.consume_reason = Some(ConsumeReason::SignalPublished); z.attempt_made = true;
    }
    out.push(Output::EntrySignal(signal));
    out
}
```

Modify `step.rs::on_tick` to call it:
```rust
    let outputs = crate::strategy::signals::try_entry(state, state.last_price, t.time_ms, t.price, cfg);
    state.last_tick_time_ms = Some(t.time_ms);
    state.last_price = Some(t.price);
    Ok(outputs)
```
(Change `on_tick` to also take `cfg`, and pass `cfg` from `step`.)

Tests (append to `signals.rs`):
```rust
#[cfg(test)]
mod touch_tests {
    use super::*;
    use crate::strategy::types::{Pivot, ZoneState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn demand() -> Zone {
        Zone { id:"z".into(), kind: ZoneKind::Demand, low: dec("1.96"), high: dec("2.00"), atr: dec("0.1"),
            breakout_time_ms:0, zone_candle_time_ms:0, activated_at_index:0, state: ZoneState::Active,
            terminal_reason:None, consume_reason:None, attempt_made:false,
            window_high: dec("2.00"), window_low: dec("1.96") }
    }

    #[test]
    fn first_touch_requires_crossing_from_above() {
        assert!(is_first_touch(Direction::Long, &demand(), dec("2.01"), dec("2.00")));
        assert!(!is_first_touch(Direction::Long, &demand(), dec("2.01"), dec("2.02")));
    }

    #[test]
    fn emits_signal_on_first_touch_with_tick_time() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.zones = vec![demand()];
        s.swing_highs = vec![Pivot { index:0, time_ms:0, price: dec("2.30"), broken:false }];
        let out = try_entry(&mut s, Some(dec("2.05")), 12345, dec("2.00"), &cfg);
        let sig = out.iter().find_map(|o| if let Output::EntrySignal(sig) = o { Some(sig) } else { None }).unwrap();
        assert_eq!(sig.triggered_at_ms, 12345); // the TOUCHING tick's time, not the previous one
        assert!(s.zones[0].state == ZoneState::Consumed);
    }

    #[test]
    fn missed_touch_on_first_observation() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.zones = vec![demand()];
        s.zones_awaiting_first_tick = vec!["z".into()];
        let out = try_entry(&mut s, Some(dec("2.10")), 1, dec("1.98"), &cfg);
        assert!(out.iter().any(|o| matches!(o, Output::ZoneEvent(_))));
        assert_eq!(s.missed_touch_count, 1);
    }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features signals::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add process/strategy-analyzer/src/strategy
git commit -m "feat(strategy-analyzer): first-touch trigger + missed-touch handling"
```

---

## Task 16: Wire structure + ATR + zones into `on_candle`; payload serialization

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/step.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Rewrite `on_candle` to be incremental (bounded window, running ATR). Replace its body with:

```rust
fn on_candle(state: &mut StrategyState, c: Candle, cfg: &Config) -> Result<Vec<Output>, StrategyError> {
    if c.high < c.low || c.high <= Decimal::ZERO || c.low <= Decimal::ZERO {
        return Err(StrategyError::MalformedCandle { at_ms: c.open_time_ms, why: "bad range/price" });
    }
    if let Some(prev) = state.last_candle_time_ms {
        if c.open_time_ms == prev { return Err(StrategyError::DuplicateCandle(c.open_time_ms)); }
        if c.open_time_ms < prev { return Err(StrategyError::OutOfOrderCandle { got_ms: c.open_time_ms, prev_ms: prev }); }
        if c.open_time_ms != prev + TIMEFRAME_MS {
            return Err(StrategyError::GappedCandle { expected_ms: prev + TIMEFRAME_MS, got_ms: c.open_time_ms });
        }
    }
    let mut out = Vec::new();

    // ATR through the PREVIOUS candle is the no-look-ahead value used for BOS/zone `A`.
    let atr_before = state.atr;

    // Record into the bounded window and advance counters.
    state.window.push_back(c);
    if state.window.len() > crate::strategy::types::WINDOW_CANDLES { state.window.pop_front(); }
    state.candle_count += 1;
    state.last_candle_time_ms = Some(c.open_time_ms);

    // Update window extremes for every live zone (window = breakout candle through latest close).
    for z in state.zones.iter_mut() {
        if z.state == ZoneState::Active {
            if c.high > z.window_high { z.window_high = c.high; }
            if c.low < z.window_low { z.window_low = c.low; }
        }
    }

    // Running ATR (incremental; fields live on the state).
    let mut atr_state = crate::strategy::indicators::atr::AtrState { value: state.atr, seed_sum: state.atr_seed_sum, seen: state.atr_seen, period: cfg.atr_period };
    let atr_now = atr_state.update(state.prev_close, c.high, c.low);
    state.atr = atr_state.value;
    state.atr_seed_sum = atr_state.seed_sum;
    state.atr_seen = atr_state.seen;

    let atr_is_zero = atr_now == Some(Decimal::ZERO);
    if atr_is_zero {
        // In-band: ATR == 0 skips ONLY BOS + zone creation. Pivots and lifecycle still run.
        out.push(Output::StateChange { reason: "atr_zero" });
    }

    // Pivots (absolute index = candle_count - 1 - right).
    let window: Vec<Candle> = state.window.iter().copied().collect();
    let (left, right) = (cfg.pivot_left as usize, cfg.pivot_right as usize);
    if let Some(mut p) = newly_confirmed_high(&window, left, right) {
        p.index = state.candle_count - 1 - right as u64;
        state.swing_highs.push(p);
    }
    if let Some(mut p) = newly_confirmed_low(&window, left, right) {
        p.index = state.candle_count - 1 - right as u64;
        state.swing_lows.push(p);
    }

    // BOS + zones (skipped only when ATR == 0).
    if !atr_is_zero {
        if let Some(atr_b) = atr_before {
            if let Some((kind, broken_idx)) = detect_bos(state, atr_b, cfg) {
                let before = state.trend;
                apply_bos(state, kind, broken_idx);
                if state.trend != before {
                    out.extend(invalidate_on_flip(&mut state.zones, state.trend));
                }
                match create_zone(&state.window, kind, atr_b, &state.symbol.clone(), cfg) {
                    Ok(mut z) => {
                        z.activated_at_index = state.candle_count - 1;
                        state.zones_awaiting_first_tick.push(z.id.clone());
                        out.push(Output::ZoneEvent(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Created, reason: None }));
                        state.zones.push(z);
                    }
                    Err(tag) => out.push(Output::StateChange { reason: tag }),
                }
            }
        }
    }

    // Lifecycle (expiry + close-beyond-edge) for the just-closed candle.
    out.extend(update_lifecycle(&mut state.zones, &c, state.candle_count - 1, cfg.zone_expiry_candles));

    // Bound state growth so per-tick cloning stays small at 1m volume: drop terminal zones
    // and broken pivots (never needed again), then clear any valid index whose pivot was pruned.
    state.zones.retain(|z| !z.is_terminal());
    state.swing_highs.retain(|p| !p.broken);
    state.swing_lows.retain(|p| !p.broken);
    if state.valid_low_idx.map_or(false, |i| !state.swing_lows.iter().any(|p| p.index == i)) { state.valid_low_idx = None; }
    if state.valid_high_idx.map_or(false, |i| !state.swing_highs.iter().any(|p| p.index == i)) { state.valid_high_idx = None; }

    state.prev_close = Some(c.close);
    Ok(out)
}
```

Add to `step.rs` imports: `use crate::strategy::indicators::atr::AtrState;` (or fully-qualify); `use crate::strategy::structure::{apply_bos, detect_bos, newly_confirmed_high, newly_confirmed_low, BosKind};`; `use crate::strategy::zones::{create_zone, invalidate_on_flip, update_lifecycle};`; and `ZoneState`, `TrendState`, `ZoneEvent`, `ZoneEventKind` from `types`.

Tests to add to `step.rs`:
```rust
#[test]
fn atr_zero_is_in_band_not_error() {
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut s = StrategyState::new("XRP-USDT", 1);
    let flat = Candle { open_time_ms: 0, open: Decimal::ONE, high: Decimal::ONE, low: Decimal::ONE, close: Decimal::ONE };
    for i in 0..14 {
        let (ns, outs) = step(&s, Input::ClosedCandle(Candle { open_time_ms: i * TIMEFRAME_MS, ..flat }), &cfg).unwrap();
        s = ns;
        if outs.iter().any(|o| matches!(o, Output::StateChange { reason: "atr_zero" })) {
            // Candle still fully processed: counter and window advanced (no early return).
            assert_eq!(s.candle_count, (i + 1) as u64);
            assert_eq!(s.window.len(), (i + 1) as usize);
            return;
        }
    }
    panic!("expected atr_zero state change");
}

#[test]
fn signal_serializes_with_string_decimals() {
    use crate::strategy::types::{Direction, Signal, TrendState};
    let sig = Signal {
        schema_version: 1, event_type: "strategy.entry_signal".into(), signal_id: "z:0".into(),
        strategy: "supply_demand_4h".into(), strategy_version: "0.1.0".into(), config_version: "abc".into(),
        symbol: "XRP-USDT".into(), market: "margin".into(), direction: Direction::Long,
        timeframe: "4hour".into(), trend_state: TrendState::Bullish, zone_id: "z".into(),
        zone_low: dec("1.96"), zone_high: dec("2.00"), zone_height_atr: dec("0.4"), atr: dec("0.1"),
        trigger_price: dec("2.00"), reference_entry_price: dec("2.00"),
        stop_loss: dec("1.95"), take_profit: dec("2.15"), target_mode: "fixed_r".into(),
        target_r: dec("3.0"), min_gross_rr: dec("3.0"), gross_rr: dec("3.0"),
        structure_obstacle_price: Some(dec("2.30")), obstacle_rr: Some(dec("6.0")),
        triggered_at_ms: 1, candle_timestamp_ms: 0,
    };
    let j = serde_json::to_string(&sig).unwrap();
    assert!(j.contains("\"stop_loss\":\"1.95\""), "decimals must serialize as strings: {j}");
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --no-default-features step::`
Expected: all pass. If `serde_json` emits a different string form, adjust the assertion to confirm **strings, not JSON numbers** (that is the requirement).

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/strategy/step.rs
git commit -m "feat(strategy-analyzer): wire structure/ATR/zones into step; payload serialization"
```

---

## Task 17: `harness/dataset.rs` — cached OHLCV + dataset hash

**Files:**
- Create: `process/strategy-analyzer/tests/fixtures/small_4h.csv`
- Create: `process/strategy-analyzer/src/harness/dataset.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `tests/fixtures/small_4h.csv`:
```csv
open_time_ms,open,high,low,close
0,1.00,1.05,0.95,1.00
14400000,1.02,1.06,0.98,0.99
28800000,1.00,1.05,0.98,1.04
43200000,1.04,1.10,1.07,1.09
57600000,1.09,1.25,1.08,1.24
```

Create `src/harness/dataset.rs`:
```rust
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use std::str::FromStr;

/// How to handle missing intervals. KuCoin omits intervals with no trades, so `Skip`
/// is the default (a gap must not abort a backtest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPolicy { Reject, Skip, ForwardFill }

pub fn load_csv(path: &str) -> Result<Vec<Candle>, String> {
    // Forward-fill by default: KuCoin omits intervals with no trades, and the core rejects
    // gaps, so a default `Reject`/`Skip` would abort a backtest at the first empty interval.
    load_csv_with_policy(path, GapPolicy::ForwardFill, 4 * 60 * 60 * 1000)
}

pub fn load_csv_with_policy(path: &str, policy: GapPolicy, expected_interval_ms: i64) -> Result<Vec<Candle>, String> {
    let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    for rec in rdr.records() {
        let r = rec.map_err(|e| e.to_string())?;
        raw.push(Candle {
            open_time_ms: r[0].parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
            open: Decimal::from_str(&r[1]).map_err(|e| e.to_string())?,
            high: Decimal::from_str(&r[2]).map_err(|e| e.to_string())?,
            low: Decimal::from_str(&r[3]).map_err(|e| e.to_string())?,
            close: Decimal::from_str(&r[4]).map_err(|e| e.to_string())?,
        });
    }
    raw.sort_by_key(|c| c.open_time_ms);
    raw.dedup_by_key(|c| c.open_time_ms);
    let mut out: Vec<Candle> = Vec::with_capacity(raw.len());
    for c in raw {
        if let Some(prev) = out.last().map(|p| p.open_time_ms) {
            let mut t = prev;
            while t + expected_interval_ms < c.open_time_ms {
                t += expected_interval_ms;
                match policy {
                    GapPolicy::Reject => return Err(format!("gap before {}", c.open_time_ms)),
                    GapPolicy::Skip => {}
                    GapPolicy::ForwardFill => {
                        let last = out.last().unwrap().close;
                        out.push(Candle { open_time_ms: t, open: last, high: last, low: last, close: last });
                    }
                }
            }
        }
        out.push(c);
    }
    Ok(out)
}

/// Stable dataset identity: sha256 over the raw file bytes.
pub fn dataset_hash(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_fixture() {
        assert_eq!(load_csv("tests/fixtures/small_4h.csv").unwrap().len(), 5);
    }

    #[test]
    fn hash_is_stable() {
        let a = dataset_hash("tests/fixtures/small_4h.csv").unwrap();
        let b = dataset_hash("tests/fixtures/small_4h.csv").unwrap();
        assert_eq!(a, b);
    }
}
```

Add `pub mod dataset;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness dataset::`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/tests process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): cached OHLCV loader + dataset hash"
```

---

## Task 18: `harness/tick_source.rs` — TickSource, path policies, 4H synthesis

**Files:**
- Create: `process/strategy-analyzer/src/harness/tick_source.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/harness/tick_source.rs`:
```rust
use crate::strategy::types::{Candle, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicy { LowFirst, HighFirst }

/// Ordered intrabar path for a candle under a global policy. Deterministic (no RNG).
pub fn synth_ticks(c: &Candle, policy: PathPolicy) -> Vec<Tick> {
    let (a, b) = match policy { PathPolicy::LowFirst => (c.low, c.high), PathPolicy::HighFirst => (c.high, c.low) };
    let out = vec![
        Tick { time_ms: c.open_time_ms, price: c.open, is_bar_open: true },
        Tick { time_ms: c.open_time_ms + 1, price: a, is_bar_open: false },
        Tick { time_ms: c.open_time_ms + 2, price: b, is_bar_open: false },
        Tick { time_ms: c.open_time_ms + 3, price: c.close, is_bar_open: false },
    ];
    out
}

pub trait TickSource {
    fn policy(&self) -> PathPolicy;
    /// Yield (ticks, closed_candle) per candle; the caller delivers ticks first.
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)>;
}

pub struct Ohlcv4hSynthesis { candles: Vec<Candle>, idx: usize, policy: PathPolicy }

impl Ohlcv4hSynthesis {
    pub fn new(candles: Vec<Candle>, policy: PathPolicy) -> Self { Self { candles, idx: 0, policy } }
}

impl TickSource for Ohlcv4hSynthesis {
    fn policy(&self) -> PathPolicy { self.policy }
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)> {
        let c = self.candles.get(self.idx)?.clone();
        self.idx += 1;
        Some((synth_ticks(&c, self.policy), c))
    }
}

/// Aggregate a contiguous run of 1m candles into one 4H candle.
pub fn aggregate_4h(minutes: &[Candle]) -> Candle {
    let first = &minutes[0];
    let (mut high, mut low) = (first.high, first.low);
    for m in minutes { if m.high > high { high = m.high; } if m.low < low { low = m.low; } }
    Candle { open_time_ms: first.open_time_ms, open: first.open, high, low, close: minutes.last().unwrap().close }
}

/// 1m source (Phase 0): each 1m candle becomes a path; every 240 minutes are emitted as one
/// 4H `ClosedCandle` (ticks first, §4 ordering). Boundaries align to the 4H epoch grid; the
/// caller validates the aggregates against the fetched 4H series before trusting a 1m run.
pub struct OneMinuteExpansion { minutes: Vec<Candle>, idx: usize, policy: PathPolicy }
impl OneMinuteExpansion {
    pub fn new(minutes: Vec<Candle>, policy: PathPolicy) -> Self { Self { minutes, idx: 0, policy } }
}
impl TickSource for OneMinuteExpansion {
    fn policy(&self) -> PathPolicy { self.policy }
    fn next_candle(&mut self) -> Option<(Vec<Tick>, Candle)> {
        let end = (self.idx + 240).min(self.minutes.len());
        if self.idx >= end { return None; }
        let slice = &self.minutes[self.idx..end];
        let ticks: Vec<Tick> = slice.iter().flat_map(|m| synth_ticks(m, self.policy)).collect();
        let agg = aggregate_4h(slice);
        self.idx = end;
        Some((ticks, agg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle() -> Candle {
        Candle { open_time_ms: 0, open: dec("1.0"), high: dec("1.3"), low: dec("0.9"), close: dec("1.1") }
    }

    #[test]
    fn low_first_visits_low_before_high() {
        let t = synth_ticks(&candle(), PathPolicy::LowFirst);
        assert!(t.iter().position(|x| x.price == dec("0.9")).unwrap() < t.iter().position(|x| x.price == dec("1.3")).unwrap());
    }

    #[test]
    fn high_first_visits_high_before_low() {
        let t = synth_ticks(&candle(), PathPolicy::HighFirst);
        assert!(t.iter().position(|x| x.price == dec("1.3")).unwrap() < t.iter().position(|x| x.price == dec("0.9")).unwrap());
    }

    #[test]
    fn source_streams_ticks_before_candle() {
        let mut src = Ohlcv4hSynthesis::new(vec![candle()], PathPolicy::LowFirst);
        let (ticks, c) = src.next_candle().unwrap();
        assert!(!ticks.is_empty());
        assert_eq!(c.high, dec("1.3"));
        assert!(src.next_candle().is_none());
    }
}
```

Add `pub mod tick_source;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness tick_source::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): TickSource + 4H synthesis + path policies"
```

---

## Task 19: `harness/replay.rs` — drive the core; replay equivalence

**Files:**
- Create: `process/strategy-analyzer/src/harness/replay.rs`
- Create: `process/strategy-analyzer/tests/replay_equivalence.rs`
- Create: `process/strategy-analyzer/tests/golden/outs.hash` (empty placeholder, filled by first run)
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/harness/replay.rs`:
```rust
use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::step::step;
use crate::strategy::types::{Input, Output, StrategyState};

pub fn replay(mut state: StrategyState, events: &[Input], cfg: &Config)
    -> Result<(StrategyState, Vec<Output>), StrategyError>
{
    let mut all = Vec::new();
    for ev in events {
        let (ns, mut outs) = step(&state, ev.clone(), cfg)?;
        state = ns;
        all.append(&mut outs);
    }
    Ok((state, all))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::{Candle, Tick};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    fn events() -> Vec<Input> {
        let mut e = Vec::new();
        for i in 0..20i64 {
            let c = Candle { open_time_ms: i * 4*3600*1000, open: dec("1.0"), high: dec("1.1"), low: dec("0.9"), close: dec("1.0") };
            e.push(Input::PriceTick(Tick { time_ms: c.open_time_ms, price: c.open, is_bar_open: true }));
            e.push(Input::ClosedCandle(c));
        }
        e
    }

    #[test]
    fn deterministic_same_outputs() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let (_, a) = replay(StrategyState::new("XRP-USDT", 1), &events(), &cfg).unwrap();
        let (_, b) = replay(StrategyState::new("XRP-USDT", 1), &events(), &cfg).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn snapshot_resume_matches_uninterrupted() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let ev = events();
        let (full, _) = replay(StrategyState::new("XRP-USDT", 1), &ev, &cfg).unwrap();
        let (mid, _) = replay(StrategyState::new("XRP-USDT", 1), &ev[..15], &cfg).unwrap();
        let json = serde_json::to_string(&mid).unwrap();
        let resumed: StrategyState = serde_json::from_str(&json).unwrap();
        let (tail, _) = replay(resumed, &ev[15..], &cfg).unwrap();
        assert_eq!(full, tail);
    }
}
```

Create `tests/replay_equivalence.rs`:
```rust
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Candle, Input, StrategyState, Output};
use rust_decimal::Decimal;
use std::str::FromStr;

#[test]
fn golden_hash_is_stable() {
    let dec = |s: &str| Decimal::from_str(s).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut s = StrategyState::new("XRP-USDT", 1);
    let mut outs: Vec<Output> = Vec::new();
    for i in 0..30i64 {
        let c = Candle { open_time_ms: i*4*3600*1000, open: dec("1"), high: dec("1.2"), low: dec("0.8"), close: dec("1.1") };
        let (ns, o) = step(&s, Input::ClosedCandle(c), &cfg).unwrap();
        s = ns; outs.extend(o);
    }
    let digest = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(serde_json::to_string(&outs).unwrap().as_bytes());
        format!("{:x}", h.finalize())
    };
    let path = "tests/golden/outs.hash";
    let golden = std::fs::read_to_string(path).unwrap_or_default();
    if golden.trim().is_empty() {
        std::fs::create_dir_all("tests/golden").unwrap();
        std::fs::write(path, &digest).unwrap();
    } else {
        assert_eq!(golden.trim(), digest, "cross-platform golden hash mismatch");
    }
}
```

Run the test once to generate `tests/golden/outs.hash`, then **commit the non-empty file** (so CI legs compare against it instead of each writing its own and passing vacuously). Generate it against the signal-producing fixture (Task 27); if Task 27 has not run yet, regenerate and re-commit after it lands.

Add `pub mod replay;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness replay`
Expected: 2 unit + 1 integration pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness process/strategy-analyzer/tests
git commit -m "feat(strategy-analyzer): replay driver + equivalence/golden tests"
```

---

## Task 20: `harness/sim.rs` — position simulation

**Files:**
- Create: `process/strategy-analyzer/src/harness/sim.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/harness/sim.rs`:
```rust
use crate::strategy::types::{Direction, Signal, Tick};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimParams {
    pub fee_bps: Decimal,
    pub entry_slip_bps: Decimal,
    pub stop_slip_bps: Decimal,      // worse than entry slip (stops hit in fast moves)
    pub borrow_bps_per_day: Decimal,
    pub latency_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimOutcome { TakeProfit(Decimal), StopLoss(Decimal), Unresolved }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimResult { pub outcome: SimOutcome, pub entry_ms: i64, pub exit_ms: i64 }

/// Simulate one signal across the FULL tick stream from entry (the caller concatenates
/// ticks across subsequent candles until resolution). Take-profit fills AT its limit price;
/// the stop fills at the stop price, or the tick price if gapped past it, plus slippage.
pub fn simulate(sig: &Signal, ticks: &[Tick], params: SimParams) -> SimResult {
    let risk = match sig.direction { Direction::Long => sig.trigger_price - sig.stop_loss, Direction::Short => sig.stop_loss - sig.trigger_price };
    let idx = params.latency_ticks as usize;
    if risk <= Decimal::ZERO || idx >= ticks.len() {
        let ms = ticks.first().map(|t| t.time_ms).unwrap_or(0);
        return SimResult { outcome: SimOutcome::Unresolved, entry_ms: ms, exit_ms: ms };
    }
    let raw = ticks[idx].price;
    let slip = raw * params.entry_slip_bps / Decimal::from(10_000);
    let entry = match sig.direction { Direction::Long => raw + slip, Direction::Short => raw - slip };
    let entry_ms = ticks[idx].time_ms;

    for t in &ticks[idx..] {
        let (hit_stop, hit_tp) = match sig.direction {
            Direction::Long => (t.price <= sig.stop_loss, t.price >= sig.take_profit),
            Direction::Short => (t.price >= sig.stop_loss, t.price <= sig.take_profit),
        };
        let days = Decimal::from((t.time_ms - entry_ms).max(0)) / Decimal::from(86_400_000);
        let cost = entry * (params.fee_bps * Decimal::from(2) + params.borrow_bps_per_day * days) / Decimal::from(10_000);
        if hit_stop {
            // Normal mid-bar touch fills AT the stop; only a tick that is a candle open already
            // beyond the stop (a gap) fills at the tick price.
            let gapped = t.is_bar_open && match sig.direction { Direction::Long => t.price < sig.stop_loss, Direction::Short => t.price > sig.stop_loss };
            let base = if gapped { t.price } else { sig.stop_loss };
            let s = base * params.stop_slip_bps / Decimal::from(10_000);
            let fill = match sig.direction { Direction::Long => base - s, Direction::Short => base + s };
            let r = match sig.direction { Direction::Long => (entry - fill - cost) / risk, Direction::Short => (fill - entry - cost) / risk };
            return SimResult { outcome: SimOutcome::StopLoss(r), entry_ms, exit_ms: t.time_ms };
        }
        if hit_tp {
            let fill = sig.take_profit; // limit price, not the tick extreme
            let r = match sig.direction { Direction::Long => (fill - entry - cost) / risk, Direction::Short => (entry - fill - cost) / risk };
            return SimResult { outcome: SimOutcome::TakeProfit(r), entry_ms, exit_ms: t.time_ms };
        }
    }
    SimResult { outcome: SimOutcome::Unresolved, entry_ms, exit_ms: ticks.last().map(|t| t.time_ms).unwrap_or(entry_ms) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::{Direction, Signal, TrendState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn long_sig() -> Signal {
        Signal {
            schema_version:1, event_type:"strategy.entry_signal".into(), signal_id:"z:0".into(),
            strategy:"supply_demand_4h".into(), strategy_version:"0.1.0".into(), config_version:"x".into(),
            symbol:"XRP-USDT".into(), market:"margin".into(), direction:Direction::Long,
            timeframe:"4hour".into(), trend_state:TrendState::Bullish, zone_id:"z".into(),
            zone_low:dec("1.96"), zone_high:dec("2.00"), zone_height_atr:dec("0.4"), atr:dec("0.1"),
            trigger_price:dec("2.00"), reference_entry_price:dec("2.00"),
            stop_loss:dec("1.95"), take_profit:dec("2.15"), target_mode:"fixed_r".into(),
            target_r:dec("3.0"), min_gross_rr:dec("3.0"), gross_rr:dec("3.0"),
            structure_obstacle_price:Some(dec("2.5")), obstacle_rr:Some(dec("10")),
            triggered_at_ms:0, candle_timestamp_ms:0,
        }
    }
    fn p() -> SimParams { SimParams { fee_bps:dec("5"), entry_slip_bps:dec("5"), stop_slip_bps:dec("15"), borrow_bps_per_day:dec("2"), latency_ticks:1 } }

    #[test]
    fn take_profit_fills_at_limit_not_tick_extreme() {
        // TP is 2.15; the last tick is 2.20 (the synthetic extreme). The fill must be AT 2.15.
        let t = vec![
            Tick{time_ms:0,price:dec("2.00"),is_bar_open:true},
            Tick{time_ms:1,price:dec("2.00"),is_bar_open:false},
            Tick{time_ms:2,price:dec("2.20"),is_bar_open:false},
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::TakeProfit(x) => assert!(x > dec("2.9") && x <= dec("3.0"), "clean TP ~3R, got {x}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn midpath_stop_fills_at_stop_price() {
        // The 1.90 tick is NOT a candle open: a normal mid-bar touch fills AT the 1.95 stop (~ -1R).
        let t = vec![
            Tick{time_ms:0,price:dec("2.00"),is_bar_open:true},
            Tick{time_ms:1,price:dec("2.00"),is_bar_open:false},
            Tick{time_ms:2,price:dec("1.90"),is_bar_open:false},
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::StopLoss(x) => assert!(x < dec("-1.0") && x > dec("-1.1"), "mid-bar stop ~-1R, got {x}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn gapped_stop_fills_at_open() {
        // The next candle OPENS at 1.90, already beyond the 1.95 stop -> fill at 1.90 (~ -2R).
        let t = vec![
            Tick{time_ms:0,price:dec("2.00"),is_bar_open:true},
            Tick{time_ms:1,price:dec("2.00"),is_bar_open:false},
            Tick{time_ms:2,price:dec("1.90"),is_bar_open:true},
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::StopLoss(x) => assert!(x < dec("-1.5"), "gap stop ~-2R, got {x}"),
            other => panic!("{other:?}"),
        }
    }
}
```

Add `pub mod sim;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness sim::`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): position simulation with latency, stop slippage, costs"
```

---

## Task 21: `harness/report.rs` — ledger, stats, comparisons, calibration, bootstrap

**Files:**
- Create: `process/strategy-analyzer/src/harness/report.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/harness/report.rs`:
```rust
use crate::strategy::types::Direction;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRecord { pub direction: Direction, pub r: Decimal, pub pair: String, pub entry_time_ms: i64, pub exit_time_ms: i64 }

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub mean_r: f64,
    pub win_rate: f64,
    pub by_pair: BTreeMap<String, f64>,
    pub by_direction_long: f64,
    pub by_direction_short: f64,
}

pub fn summarize(trades: &[TradeRecord]) -> Summary {
    let n = trades.len();
    let mean = |xs: &[Decimal]| if xs.is_empty() { 0.0 } else { xs.iter().map(|d| d.to_string().parse::<f64>().unwrap()).sum::<f64>() / xs.len() as f64 };
    let all: Vec<Decimal> = trades.iter().map(|t| t.r).collect();
    let wins = trades.iter().filter(|t| t.r > Decimal::ZERO).count();
    let mut by_pair: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for t in trades { by_pair.entry(t.pair.clone()).or_default().push(t.r); }
    let longs: Vec<Decimal> = trades.iter().filter(|t| t.direction == Direction::Long).map(|t| t.r).collect();
    let shorts: Vec<Decimal> = trades.iter().filter(|t| t.direction == Direction::Short).map(|t| t.r).collect();
    Summary {
        n,
        mean_r: mean(&all),
        win_rate: if n == 0 { 0.0 } else { wins as f64 / n as f64 },
        by_pair: by_pair.into_iter().map(|(k, v)| (k, mean(&v))).collect(),
        by_direction_long: mean(&longs),
        by_direction_short: mean(&shorts),
    }
}

/// Adversarial composite: longs from `low_first`, shorts from `high_first`.
pub fn adversarial_composite(low_first: &[TradeRecord], high_first: &[TradeRecord]) -> Vec<TradeRecord> {
    let mut out: Vec<TradeRecord> = low_first.iter().filter(|t| t.direction == Direction::Long).cloned().collect();
    out.extend(high_first.iter().filter(|t| t.direction == Direction::Short).cloned());
    out
}

/// Flag when realised cost exceeds 0.5R; references the analyzer's floor.
pub fn cushion_warning(mean_cost_r: f64) -> Option<&'static str> {
    if mean_cost_r > 0.5 { Some("cost > 0.5R: the analyzer's floor (target_r in fixed_r, min_gross_rr in structural) is too low") } else { None }
}

/// Deterministic tiny LCG (no `rand`). `state = state * 6364136223846793005 + 1442695040888963407`.
pub(crate) struct Lcg(u64);
impl Lcg {
    pub(crate) fn new(seed: u64) -> Self { Lcg(seed) }
    pub(crate) fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
    pub(crate) fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

pub const BOOTSTRAP_BLOCK_MS: i64 = 14 * 86_400_000; // 2 weeks (pre-registered block length)

/// Calendar-block bootstrap: assign each trade to a calendar block by `entry_time_ms`,
/// resample whole blocks with replacement, return (mean, 5th-percentile lower bound).
pub fn time_blocked_bootstrap(trades: &[TradeRecord], block_ms: i64, resamples: usize, seed: u64) -> (f64, f64) {
    if trades.is_empty() { return (0.0, 0.0); }
    let vals = |r: Decimal| r.to_string().parse::<f64>().unwrap();
    let mean0 = trades.iter().map(|t| vals(t.r)).sum::<f64>() / trades.len() as f64;
    let t0 = trades.iter().map(|t| t.entry_time_ms).min().unwrap();
    let mut blocks: std::collections::BTreeMap<i64, Vec<f64>> = std::collections::BTreeMap::new();
    for t in trades { blocks.entry((t.entry_time_ms - t0) / block_ms).or_default().push(vals(t.r)); }
    let block_list: Vec<Vec<f64>> = blocks.into_values().collect();
    let n = block_list.len();
    let mut rng = Lcg(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut acc = 0.0; let mut cnt = 0usize;
        for _ in 0..n {
            let b = rng.below(n);
            for v in &block_list[b] { acc += v; cnt += 1; }
        }
        means.push(acc / cnt as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (mean0, means[(means.len() as f64 * 0.05) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn t(r: &str, p: &str) -> TradeRecord { TradeRecord { direction: Direction::Long, r: Decimal::from_str(r).unwrap(), pair: p.into(), entry_time_ms: 0, exit_time_ms: 0 } }

    #[test]
    fn summary_counts_and_splits() {
        let s = summarize(&[t("3.0","A"), t("-1.0","A"), t("3.0","B")]);
        assert_eq!(s.n, 3);
        assert!(s.mean_r > 1.6 && s.mean_r < 1.7);
        assert_eq!(s.by_pair.len(), 2);
    }

    #[test]
    fn bootstrap_is_deterministic_and_calendar_blocked() {
        let data: Vec<_> = (0..200).map(|i| TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(if i % 3 == 0 { "-1.0" } else { "3.0" }).unwrap(),
            pair: "A".into(), entry_time_ms: i as i64 * 86_400_000, exit_time_ms: i as i64 * 86_400_000,
        }).collect();
        assert_eq!(time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42), time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42));
    }

    #[test]
    fn cushion_warning_references_floor() {
        assert!(cushion_warning(0.6).unwrap().contains("floor"));
    }
}
```

Add `pub mod report;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness report::`
Expected: 3 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): reporting, splits, seeded bootstrap, cushion calibration"
```

---

## Task 22: `harness/lock.rs` — freeze lock + one-shot OOS

**Files:**
- Create: `process/strategy-analyzer/src/harness/lock.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Create `src/harness/lock.rs`:
```rust
use sha2::{Digest, Sha256};
use std::path::Path;

/// lock = hash(config_version + backtest params)
pub fn lock_hash(config_version: &str, backtest_params_json: &str) -> String {
    let mut h = Sha256::new();
    h.update(config_version.as_bytes());
    h.update(b"|");
    h.update(backtest_params_json.as_bytes());
    format!("{:x}", h.finalize())
}

/// Refuse to start OOS without a matching lock.
pub fn check_lock(lock_path: &str, expected: &str) -> Result<(), String> {
    match std::fs::read_to_string(lock_path) {
        Ok(v) if v.trim() == expected => Ok(()),
        Ok(v) => Err(format!("lock mismatch: expected {expected}, found {}", v.trim())),
        Err(_) => Err(format!("no lock file at {lock_path}; in-sample exploration only")),
    }
}

pub fn write_lock(lock_path: &str, value: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(lock_path).parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(lock_path, value)
}

/// True if an OOS run on this data has already been consumed (one-shot guard).
pub fn oos_already_consumed(consumed_path: &str) -> bool {
    Path::new(consumed_path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_depends_on_both_inputs() {
        assert_ne!(lock_hash("cfg1", "{\"fee\":5}"), lock_hash("cfg1", "{\"fee\":6}"));
        assert_ne!(lock_hash("cfg1", "{\"fee\":5}"), lock_hash("cfg2", "{\"fee\":5}"));
    }

    #[test]
    fn missing_lock_is_error() {
        assert!(check_lock("/nonexistent/lock", "x").is_err());
    }
}
```

Add `pub mod lock;` to `harness/mod.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p strategy-analyzer --features harness lock::`
Expected: 2 pass.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): freeze lock + one-shot OOS guard"
```

---

## Task 23: `harness/fetch.rs` + CLI skeleton

**Files:**
- Create: `process/strategy-analyzer/src/harness/fetch.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`
- Modify: `process/strategy-analyzer/src/bin/backtest.rs`

- [ ] **Step 1: Write the failing test (paging planner) + implementation**

Create `src/harness/fetch.rs`:
```rust
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_respect_1500_cap() {
        let p = page_windows(0, 3000 * 60, 60);
        assert_eq!(p, vec![(0, 1500 * 60), (1500 * 60, 3000 * 60)]);
    }
}
```

Add `pub mod fetch;` to `harness/mod.rs`.

Replace `src/bin/backtest.rs`:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    /// Fetch OHLCV and cache to a file. Never used by `backtest`.
    FetchData {
        #[arg(long)] symbol: String,
        #[arg(long, default_value = "4hour")] interval: String,
        #[arg(long)] start_s: i64,
        #[arg(long)] end_s: i64,
        #[arg(long)] out: String,
    },
    /// Run the backtest offline against a cached dataset.
    Backtest {
        #[arg(long)] data: String,
        #[arg(long, default_value = "both")] policy: String,
        #[arg(long, default_value = "both")] mode: String,
        #[arg(long)] lock: Option<String>,
        #[arg(long, default_value_t = false)] oos: bool,
        #[arg(long, default_value_t = false)] oos_consumed: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::FetchData { .. } => eprintln!("fetch-data: network call added in Task 24; paging planner is tested"),
        Cmd::Backtest { data, .. } => {
            let candles = strategy_analyzer::harness::dataset::load_csv(&data)?;
            println!("loaded {} candles from {}", candles.len(), data);
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Run the test and the CLI**

Run: `cargo test -p strategy-analyzer --features harness fetch::`
Expected: 1 pass.
Run: `cargo run -p strategy-analyzer --features harness --bin backtest -- backtest --data tests/fixtures/small_4h.csv`
Expected: prints `loaded 5 candles from ...`.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/src/harness/fetch.rs process/strategy-analyzer/src/harness/mod.rs process/strategy-analyzer/src/bin/backtest.rs
git commit -m "feat(strategy-analyzer): fetch paging planner + backtest CLI skeleton"
```

---

## Task 24: Full backtest run + real `fetch-data`

**Files:**
- Modify: `process/strategy-analyzer/src/bin/backtest.rs`
- Modify: `process/strategy-analyzer/src/harness/fetch.rs`
- Create: `process/strategy-analyzer/tests/integration_backtest.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/integration_backtest.rs`:
```rust
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::{Config, TargetMode};
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

#[test]
fn both_policies_run_and_match_signal_counts() {
    let candles = load_csv("tests/fixtures/small_4h.csv").unwrap();
    let mut counts = Vec::new();
    for policy in [PathPolicy::LowFirst, PathPolicy::HighFirst] {
        let cfg = Config { target_mode: TargetMode::FixedR, ..Config::default_for(&["XRP-USDT"]) };
        let mut st = StrategyState::new("XRP-USDT", 1);
        let mut src = Ohlcv4hSynthesis::new(candles.clone(), policy);
        let mut signals = 0usize;
        while let Some((ticks, candle)) = src.next_candle() {
            for t in ticks { let (ns, _) = step(&st, Input::PriceTick(t), &cfg).unwrap(); st = ns; }
            let (ns, outs) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap(); st = ns;
            signals += outs.iter().filter(|o| matches!(o, Output::EntrySignal(_))).count();
        }
        counts.push(signals);
    }
    assert_eq!(counts[0], counts[1]);
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p strategy-analyzer --features harness --test integration_backtest`
Expected: pass.

- [ ] **Step 3: Implement the real `fetch-data` (ureq, throttled)**

In `src/harness/fetch.rs`, add a network function used **only** by the `FetchData` subcommand:
```rust
/// Fetch one page from KuCoin and return raw CSV rows `time,open,close,high,low,volume`.
/// Throttle between calls in the caller. Keep I/O out of the core.
pub fn fetch_page(base_url: &str, symbol: &str, interval: &str, start_s: i64, end_s: i64)
    -> Result<Vec<[String; 7]>, String>
{
    let url = format!("{base_url}/api/v1/market/candles?symbol={symbol}&type={interval}&startAt={start_s}&endAt={end_s}");
    let body: serde_json::Value = ureq::get(&url).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?;
    if body.get("code").and_then(|c| c.as_str()) != Some("200000") {
        return Err(format!("KuCoin error: {}", body.get("msg").and_then(|m| m.as_str()).unwrap_or("unknown")));
    }
    let rows = body.get("data").and_then(|d| d.as_array()).ok_or("no data")?;
    Ok(rows.iter().map(|r| {
        let a = r.as_array().unwrap();
        std::array::from_fn(|i| a[i].as_str().unwrap_or_default().to_string())
    }).collect())
}
```
Wire `FetchData` in `bin/backtest.rs` to: page via `page_windows`; call `fetch_page` with a small `std::thread::sleep` between pages and **retry with exponential backoff on HTTP 429 / rate-limit responses**; then (a) convert KuCoin **seconds to milliseconds** (`* 1000`), (b) **sort rows oldest-first**, (c) **de-duplicate across page boundaries by open time**, (d) write the cache CSV (`open_time_ms,open,high,low,close`; KuCoin column order is `time,open,close,high,low,volume,turnover`), and finally print `dataset_hash(out)`.

- [ ] **Step 4: Implement `harness/run.rs` (orchestration — coded, not prose)**

Create `process/strategy-analyzer/src/harness/run.rs`:

```rust
use crate::harness::report::TradeRecord;
use crate::harness::sim::{simulate, SimOutcome, SimParams};
use crate::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy};
use crate::strategy::config::Config;
use crate::strategy::step::step;
use crate::strategy::types::{Candle, ConsumeReason, Input, Output, Signal, StrategyState, Tick, ZoneEvent, ZoneEventReason};
use rust_decimal::Decimal;

pub struct RunResult {
    pub trades: Vec<TradeRecord>,
    pub signals: usize,
    pub skipped_open_position: usize,
    pub missed_touch: u64,
}

/// Run one (policy) over a dataset. Signals are collected in pass 1; pass 2 simulates them
/// in time order, opening at most one position at a time (later signals while a position is
/// open are counted as `skipped_open_position`, mirroring the engine's caps).
pub fn run(symbol: &str, candles: &[Candle], policy: PathPolicy, cfg: &Config, params: SimParams) -> RunResult {
    let mut state = StrategyState::new(symbol, crate::strategy::STATE_SCHEMA_VERSION);
    let mut src = Ohlcv4hSynthesis::new(candles.to_vec(), policy);
    let mut flat_ticks: Vec<Tick> = Vec::new();
    let mut signals: Vec<(Signal, usize)> = Vec::new();
    let mut missed_touch = 0u64;

    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let idx = flat_ticks.len();
            let (ns, outs) = step(&state, Input::PriceTick(t), cfg).expect("core tick step");
            state = ns;
            flat_ticks.push(t);
            for o in outs {
                if let Output::EntrySignal(s) = o { signals.push((s, idx)); }
            }
        }
        let (ns, outs) = step(&state, Input::ClosedCandle(candle), cfg).expect("core candle step");
        state = ns;
        missed_touch += outs.iter().filter(|o| matches!(
            o,
            Output::ZoneEvent(ZoneEvent { reason: Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch)), .. })
        )).count() as u64;
    }

    let n_signals = signals.len();
    let mut trades = Vec::new();
    let mut skipped_open_position = 0usize;
    let mut open_until_ms: Option<i64> = None;

    for (sig, idx) in signals {
        let entry_ms = flat_ticks.get(idx + params.latency_ticks as usize)
            .map(|t| t.time_ms)
            .unwrap_or(sig.triggered_at_ms);
        if open_until_ms.map_or(false, |x| entry_ms < x) {
            skipped_open_position += 1; // a position is still open; the engine would block this
            continue;
        }
        let res = simulate(&sig, &flat_ticks[idx..], params);
        match res.outcome {
            SimOutcome::TakeProfit(r) | SimOutcome::StopLoss(r) => {
                open_until_ms = Some(res.exit_ms);
                trades.push(TradeRecord { direction: sig.direction, r, pair: symbol.to_string(), entry_time_ms: res.entry_ms, exit_time_ms: res.exit_ms });
            }
            SimOutcome::Unresolved => {
                // Position never resolved within the data: counts as a trade of 0R, keeps the slot open to the end.
                open_until_ms = Some(i64::MAX);
                trades.push(TradeRecord { direction: sig.direction, r: Decimal::ZERO, pair: symbol.to_string(), entry_time_ms: res.entry_ms, exit_time_ms: res.exit_ms });
            }
        }
    }

    RunResult { trades, signals: n_signals, skipped_open_position, missed_touch }
}
```

Then the `Backtest` arm runs `run(...)` for each `(policy in {LowFirst, HighFirst}) × (mode in {FixedR, Structural})`, prints the §8.5 header, and reports: mode × path side-by-side, long/short + per-pair splits, the adversarial composite, cushion calibration, and the bootstrap bound. Diagnostics (Tasks 29–31) run **on 4H synthesis only**, within the pre-registered wall-clock budget. If `--oos`, enforce `lock::check_lock`; on `--oos-consumed`, stamp non-confirmatory and record the prior run.

- [ ] **Step 5: Run the CLI end to end**

Run: `cargo run -p strategy-analyzer --features harness --bin backtest -- backtest --data tests/fixtures/small_4h.csv`
Expected: prints the report header + short trade table without error.

- [ ] **Step 6: Commit**

```bash
git add process/strategy-analyzer/src/bin/backtest.rs process/strategy-analyzer/src/harness/fetch.rs process/strategy-analyzer/tests/integration_backtest.rs
git commit -m "feat(strategy-analyzer): full backtest run + fetch-data (policies x modes, lock, report)"
```

---

## Task 25: CI wiring (float grep, HashMap grep, core-only build, matrix)

**Files:**
- Modify: `Makefile` (add `strategy-check`; call it from `ci`)
- Modify: `.github/workflows/ci.yml` (add purity checks + a 2-entry matrix)

- [ ] **Step 1: Add the Makefile target**

```make
strategy-check:
	@echo "checking strategy core purity..."
	@! grep -RInE '\b(f32|f64)\b' process/strategy-analyzer/src/strategy --include='*.rs' | grep -vE '//' || (echo "float type in core" && exit 1)
	@! grep -RInE '\b(HashMap|HashSet)\b' process/strategy-analyzer/src/strategy --include='*.rs' | grep -vE '//' || (echo "hash-map in core" && exit 1)
	cargo build -p strategy-analyzer --no-default-features
	cargo build -p strategy-analyzer --all-features
```

The `grep -vE '//'` filters lines that contain a `//` comment, so matches in comments are ignored (the spec excludes comments; a full tokenizer is out of scope). Add `strategy-check` to the `ci` target after `test`, and to `.PHONY`.

**Lint/test the harness too:** change the Makefile `clippy` and `test` targets to pass `--all-features` for `strategy-analyzer` (so `harness/` is linted and tested, not just the core), e.g. `cargo clippy -p strategy-analyzer --all-features --no-deps -- -D warnings` and `cargo test -p strategy-analyzer --all-features`.

- [ ] **Step 2: Add the CI matrix and steps**

In `.github/workflows/ci.yml`, convert `backend-test` to a matrix with two entries (`os: [ubuntu-latest, ubuntu-24.04-arm]`) and add steps: `make strategy-check`; and on one leg `cargo test -p strategy-analyzer --features harness --test replay_equivalence` (the cross-platform golden hash).

- [ ] **Step 3: Verify locally**

Run: `make strategy-check`
Expected: passes (no float/hash-map in core; core-only build succeeds).

- [ ] **Step 4: Commit**

```bash
git add Makefile .github/workflows/ci.yml
git commit -m "chore(strategy-analyzer): CI purity checks + core-only build + platform matrix"
```

---

## Task 26: Crate README + spec status

**Files:**
- Create: `process/strategy-analyzer/README.md`
- Modify: `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md`

- [ ] **Step 1: Write the crate README**

Cover: what the crate is; the `harness → strategy` boundary; core-only vs harness builds; `fetch-data` / `backtest` usage; the numeric/determinism policy; and the open TO-CONFIRM item (§6.3 valid high/low).

- [ ] **Step 2: Verify docs build**

Run: `cargo doc -p strategy-analyzer --no-deps --no-default-features`
Expected: builds with no broken intra-doc links.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/README.md docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md
git commit -m "docs(strategy-analyzer): crate README + spec implementation status"
```

---

## Task 27: Signal-producing fixture + reuse in replay/golden/integration

**Files:**
- Create: `process/strategy-analyzer/tests/fixtures/signal_4h.csv`
- Create: `process/strategy-analyzer/tests/signal_fixture.rs`

- [ ] **Step 1: Hand-derive a series that yields exactly one signal**

Do **not** tune to a passing implementation — derive it from the spec, with the working in comments, and have a human check it before accepting the golden hash.

Derivation (defaults: `atr_period` 14, `pivot_left/right` 2, `bos_min_distance` 0.1·ATR, `impulse_body` 1.5·ATR, `zone_search_lookback` 10, `zone_expiry` 30, `target_r` 3.0, `allow_no_obstacle` true; price scale 1 dp):

- Candles 0–12 flat (`o=c=100.0, h=100.5, l=99.5`): each true range is 1.0; the ATR seed (simple mean of the first 14 TRs, candles 0–13) is **1.0357**, and the 14-candle warm-up completes at index 13. (Candle 15's 5.0 range pushes ATR up: ATR15 = 1.3165.)
- **Swing low at 13** (`l=99.0`): right lows 14 (`100.0`) and 15 (`100.5`) are higher; left lows 11/12 (`99.5`) are ≥ 99.0. Confirmed at candle 15.
- **Swing high at 15** (`h=105.5`): right highs 16 (`105.2`) and 17 (`104.5`) are lower; left highs 13/14 (`100.5`/`101`) are ≤ 105.5. Confirmed at candle 17.
- **Demand candle at 19** (`o=104.5, h=105.0, l=104.0, c=104.2`, bearish): the most recent bearish candle before the breakout. Zone = `[104.0, 105.0]`, height 1.0 → 1.0 ATR (inside [0.5, 3.0]). Not mitigated (no candles strictly between 19 and the breakout).
- **Breakout candle at 20** (`o=105.0, h=109.0, l=104.9, c=108.0`): with the pre-breakout ATR (**ATR19 = 1.2929**), the impulse threshold is 1.5·ATR = 1.9394 (body 3.0 passes) and the BOS level is 105.5 + 0.1·ATR = 105.6293 (close 108.0 passes). `apply_bos` marks the 105.5 high broken; valid low = the 99.0 low at index 13. (Candle 18's low 103.2 is **not** a swing low — it is above candle 17's 103.0 — so there is no swing low between 15 and 20.) → trend Bullish.
- **Touch at candle 21** (`o=107.5, h=108.0, l=104.5, c=105.5`): `low_first` path is O(107.5) → L(104.5): previous 107.5 > zone_high 105.0, current 104.5 ≤ 105.0 → first touch. **The stop uses `zone.atr` — the pre-breakout ATR stored at BOS creation (ATR19 = 1.2929)**, per "no look-ahead, ATR before the breakout"; it does NOT use the latest closed ATR (ATR20 = 1.5434, which would give stop 103.8 / risk 0.7 / rr 6.43). So stop = 104.0 − 0.1·1.2929 = 103.8707 → rounds half-up to **103.9**; risk = **0.6**; TP = **106.3**. Obstacle = the breakout high **109.0** (the unconfirmed-extreme window includes the breakout candle, patched §7.5; no unbroken swing high or supply zone sits above 104.5) → obstacle_rr = (109.0 − 104.5)/0.6 = **7.5** ≥ 3.0 → **signal emitted**. Under `high_first` the path is O(107.5) → H(108.0) → L(104.5); the first crossing is again L(104.5), so both policies yield the same signal. Exactly one signal.

`tests/fixtures/signal_4h.csv`:
```csv
open_time_ms,open,high,low,close
0,100.0,100.5,99.5,100.0
14400000,100.0,100.5,99.5,100.0
28800000,100.0,100.5,99.5,100.0
43200000,100.0,100.5,99.5,100.0
57600000,100.0,100.5,99.5,100.0
72000000,100.0,100.5,99.5,100.0
86400000,100.0,100.5,99.5,100.0
100800000,100.0,100.5,99.5,100.0
115200000,100.0,100.5,99.5,100.0
129600000,100.0,100.5,99.5,100.0
144000000,100.0,100.5,99.5,100.0
158400000,100.0,100.5,99.5,100.0
172800000,100.0,100.5,99.5,100.0
187200000,100.0,100.5,99.0,100.0
201600000,100.0,101.0,100.0,101.0
216000000,101.0,105.5,100.5,105.0
230400000,105.0,105.2,104.0,104.2
244800000,104.2,104.5,103.0,103.5
259200000,103.5,104.0,103.2,103.6
273600000,104.5,105.0,104.0,104.2
288000000,105.0,109.0,104.9,108.0
302400000,107.5,108.0,104.5,105.5
```

Test (assert exactly one signal):
```rust
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{Input, Output, StrategyState};

#[test]
fn signal_fixture_yields_exactly_one_signal() {
    let candles = load_csv("tests/fixtures/signal_4h.csv").unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, PathPolicy::LowFirst);
    let mut n = 0usize;
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks { let (ns, o) = step(&st, Input::PriceTick(t), &cfg).unwrap(); st = ns; n += o.iter().filter(|o| matches!(o, Output::EntrySignal(_))).count(); }
        let (ns, _) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap(); st = ns;
    }
    assert_eq!(n, 1, "hand-derived fixture must produce exactly one signal");
}
```

**Negative variants** (same head, candles 0–19; `open_time_ms = i * 14400000`). Hand-derived; each is a full CSV. Assert the signal count with the same loop (`assert_eq!(n, expected)`):

- `signal_second_touch_4h.csv` — happy path plus one more touch: append `316800000,105.0,106.0,104.2,104.8`. The zone is already `Consumed`, so the second touch yields no new signal → **1**.
- `signal_mitigated_4h.csv` — a candle between the zone candle and the breakout trades back into the zone, while no *earlier* candle BOSes (so the breakout is at 22 and the grace candle skips index 20, leaving the dip at 21):
  `19:104.5,105.0,104.0,104.2, 20:105.0,105.5,104.8,105.4, 21:104.2,105.2,104.0,105.0, 22:104.5,109.0,104.4,108.0`.
  Candles 20 (close 105.4) and 21 (close 105.0) stay below the BOS level (105.63), so no BOS precedes 22; candle 21 is bullish (so the demand candle is still 19). **Candle 20 also enters the zone** — its low 104.8 ≤ zone_high 105.0 — but it is skipped as the single grace candle: with `mitigation_grace_candles = 1` the mitigation scan starts at `zi + 1 + 1 = 21`, i.e. it runs over `[21, 22)` only. Candle 21 (low 104.0 ≤ zone_high 105.0) is inside that window → `mitigated` at the breakout on 22 → **0**. (So the derivation is correct not because candle 20 stays out of the zone, but because the grace window deliberately ignores it. Do NOT use a close ≥ 105.63 on candle 20 — that BOSes early and rejects the zone as `weak_impulse` instead, so the `mitigated` path is never exercised.)
- `signal_obstacle_too_close_4h.csv` — a lower breakout high keeps the obstacle within 3R:
  `19:104.5,105.0,104.0,104.2, 20:104.0,106.2,103.9,106.0, 21:106.0,106.1,104.9,105.0`.
  Candle 20 body 2.0 passes the impulse gate and close 106.0 passes the BOS level (105.63); the touch at 21 triggers 104.9; obstacle = window_high 106.2 → rr = (106.2 − 104.9)/risk < 3 → veto → **0**.

> Send all three derivations to the human reviewer alongside the happy path before locking the golden hash.

> If the derived series does not produce exactly one signal, fix the **series** (report the discrepancy), do not weaken the assertion. Send the fixture and this derivation to the human reviewer before locking the golden hash.

- [ ] **Step 2: Reuse the fixture**

Change the replay-equivalence test (Task 19) and the integration test (Task 24) to use `signal_4h.csv` (not `small_4h.csv`), so those tests exercise a real signal path.

- [ ] **Step 3: Commit**

```bash
git add process/strategy-analyzer/tests
git commit -m "test(strategy-analyzer): signal-producing fixture reused across harness tests"
```

---

## Task 28: Edge-case accuracy tests

**Files:**
- Modify: `process/strategy-analyzer/src/strategy/signals.rs` (tests only)
- Modify: `process/strategy-analyzer/src/strategy/structure.rs` (tests only)

- [ ] **Step 1: Add the missing tests** (Task 14 already adds structural/no-obstacle; Task 9 adds the BOS equality boundary). Add the remaining:

```rust
#[test]
fn supply_path_short_signal_end_to_end() {
    // A strengthening down-move forms a supply zone; a tick crosses zone_low from below.
    // Assert one Short EntrySignal with stop above zone_high and TP below trigger.
}

#[test]
fn zone_size_bounds_reject_exactly_at_the_boundary() {
    // height/atr == min -> accepted; just below -> rejected; == max -> accepted; just above -> rejected.
}
```
Fill these with concrete values following the patterns in Tasks 11/14 (demand-zone construction mirrored for supply; `compute_levels` for the short).

- [ ] **Step 2: Run and commit**

Run: `cargo test -p strategy-analyzer --no-default-features signals:: structure::`
Expected: all pass.

```bash
git add process/strategy-analyzer/src/strategy/signals.rs process/strategy-analyzer/src/strategy/structure.rs
git commit -m "test(strategy-analyzer): supply path + zone size boundaries"
```

---

## Task 29: Verdict, regime tagging, concentration, cost stress

**Files:**
- Create: `process/strategy-analyzer/src/harness/verdict.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

```rust
use crate::harness::report::{Summary, TradeRecord, time_blocked_bootstrap, BOOTSTRAP_BLOCK_MS};
use rust_decimal::Decimal;

/// Regime by the 200-day average: `true` = uptrend. `closes` are daily-ish closes.
pub fn is_uptrend(closes: &[Decimal]) -> bool {
    if closes.len() < 200 { return false; }
    let mean200: Decimal = closes[closes.len() - 200..].iter().copied().sum::<Decimal>() / Decimal::from(200);
    closes.last().map(|c| *c >= mean200).unwrap_or(false)
}

#[derive(Debug, PartialEq)]
pub struct Verdict { pub n: usize, pub mean_r: f64, pub lower_95: f64, pub passes: bool, pub inconclusive_single_regime: bool, pub concentrated: bool, pub cost_stress_ok: bool }

/// Evaluate the pre-registered §10.4 bar. Gates: n >= 200 (executed), mean net R > 0,
/// bootstrap lower bound > 0 (adversarial composite), not single-regime, no pair > 50% of total R,
/// and mean still positive at 1.5x costs.
pub fn verdict(composite: &[TradeRecord], one_regime_only: bool, total_r: Decimal, max_pair_r: Decimal, mean_r_at_1_5x_cost: f64) -> Verdict {
    let (mean_r, lower_95) = time_blocked_bootstrap(composite, BOOTSTRAP_BLOCK_MS, 10_000, 12345);
    let concentrated = total_r > Decimal::ZERO && (max_pair_r / total_r) > Decimal::new(5, 1); // > 50%
    let cost_stress_ok = mean_r_at_1_5x_cost > 0.0;
    let passes = composite.len() >= 200 && mean_r > 0.0 && lower_95 > 0.0 && !one_regime_only && !concentrated && cost_stress_ok;
    Verdict { n: composite.len(), mean_r, lower_95, passes, inconclusive_single_regime: one_regime_only, concentrated, cost_stress_ok }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Direction;
    use std::str::FromStr;
    fn rec(r: &str) -> TradeRecord { TradeRecord { direction: Direction::Long, r: Decimal::from_str(r).unwrap(), pair: "A".into(), entry_time_ms: 0, exit_time_ms: 0 } }

    #[test]
    fn healthy_sample_passes() {
        let data: Vec<_> = (0..220).map(|i| if i % 4 == 0 { rec("-1.0") } else { rec("3.0") }).collect();
        let total: Decimal = data.iter().map(|t| t.r).sum();
        let v = verdict(&data, false, total, total / Decimal::from(2), 0.5);
        assert!(v.passes, "{v:?}");
    }

    #[test]
    fn single_regime_and_concentration_block() {
        let data: Vec<_> = (0..220).map(|i| if i % 4 == 0 { rec("-1.0") } else { rec("3.0") }).collect();
        let total: Decimal = data.iter().map(|t| t.r).sum();
        let one_regime = verdict(&data, true, total, total / Decimal::from(2), 0.5);
        assert!(!one_regime.passes); // a single regime is inconclusive
        let concentrated = verdict(&data, false, total, total, 0.5); // one pair holds 100% of R
        assert!(concentrated.concentrated);
        assert!(!concentrated.passes);
    }
}
```

- [ ] **Step 2: Run and commit**

Run: `cargo test -p strategy-analyzer --features harness verdict::`
Expected: pass.

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): verdict + regime + concentration + cost-stress gates"
```

---

## Task 30: Perturbation diagnostics (OAT + joint), 4H synthesis only

**Files:**
- Create: `process/strategy-analyzer/src/harness/perturb.rs`
- Modify: `process/strategy-analyzer/src/harness/mod.rs`

- [ ] **Step 1: Write the failing tests + implementation**

```rust
use crate::harness::report::Lcg;
use rust_decimal::Decimal;

/// One-at-a-time ±20% perturbation of a single parameter, passed through `eval`.
/// Returns the fraction of cells (each ±20%) whose expectancy > 0.
pub fn oat_positive_fraction(base: Decimal, eval: &dyn Fn(Decimal) -> Decimal) -> f64 {
    let mut ok = 0; let mut total = 0;
    for f in [Decimal::new(8, 1), Decimal::new(12, 1)] { // 0.8 and 1.2
        if eval(base * f) > Decimal::ZERO { ok += 1; }
        total += 1;
    }
    ok as f64 / total as f64
}

/// Seeded joint perturbation: each parameter uniform in ±20%; returns the fraction of
/// `draws` with positive expectancy.
pub fn joint_positive_fraction(base: &[Decimal], draws: usize, seed: u64, eval: &dyn Fn(&[Decimal]) -> Decimal) -> f64 {
    let mut rng = Lcg::new(seed);
    let mut ok = 0;
    for _ in 0..draws {
        let v: Vec<Decimal> = base.iter().map(|b| {
            // uniform in [-0.2, 0.2]: map u64 fraction to a per-mille offset
            let off = (rng.below(401) as i64 - 200); // -200..=200 -> -0.2..=0.2
            *b * (Decimal::ONE + Decimal::from(off) / Decimal::from(1000))
        }).collect();
        if eval(&v) > Decimal::ZERO { ok += 1; }
    }
    ok as f64 / draws as f64
}

// reuse Lcg: make it `pub(crate)` in report.rs.
```
Tests: `oat_positive_fraction` returns 1.0 when eval is always positive; `joint_positive_fraction` is deterministic for a fixed seed.

- [ ] **Step 2: Run and commit**

Run: `cargo test -p strategy-analyzer --features harness perturb::`
Expected: pass.

```bash
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): seeded OAT + joint perturbation diagnostics"
```

Note: the CLI runs these **on 4H synthesis only**, within the pre-registered wall-clock budget.

---

## Task 31: 1m calibration report

**Files:**
- Modify: `process/strategy-analyzer/src/harness/report.rs`

- [ ] **Step 1: Write the failing test + implementation**

```rust
/// Given per-candle comparisons of the assumed vs. observed intrabar high/low order,
/// return the fraction of candles where the assumption was wrong.
pub fn calibration_wrong_fraction(assumed_low_first: &[bool], observed_low_first: &[bool]) -> f64 {
    if assumed_low_first.is_empty() { return 0.0; }
    let wrong = assumed_low_first.iter().zip(observed_low_first).filter(|(a, o)| a != o).count();
    wrong as f64 / assumed_low_first.len() as f64
}
```
Test: identical → 0.0; all different → 1.0. Print this in the report header when 1m data is present.

- [ ] **Step 2: Run and commit**

```bash
cargo test -p strategy-analyzer --features harness report::calibration
git add process/strategy-analyzer/src/harness
git commit -m "feat(strategy-analyzer): 1m intrabar calibration report"
```

---

## Review gates

- **After Task 16 (core complete):** stop and run a human/code review of `strategy/` before starting the harness. Confirm: no float/HashMap in core, replay determinism, signal payload correctness, and the §6.3 tre/valid-high-low behavior.
- **After Task 23 (harness CLI):** stop and review the harness before wiring the final backtest/verdict.

---

## Self-Review

**Spec coverage:**
- §1 layout/features → Task 1; §2 numeric policy → Tasks 2, 4, 16; §3 config/validation → Task 4; §4 I/O/errors/ordering/per-symbol → Tasks 5, 6, 15, 16; §5 structure → Tasks 8–10; §6 zones/lifecycle → Tasks 11–13; §7 signals/levels/obstacle/modes → Tasks 14–15; §8 harness (sources, paging, policies, sim, reporting, datasets, fetch) → Tasks 17–24, 28; §9 payload → Tasks 5, 16; §10 tests/bar/verdict → Tasks 19, 21, 22, 27, 29–31; §11–13 → logs/README + deferred list.
- **Second-review fixes** are reflected in the Revision note above and in Tasks 5, 7, 9–17, 19–21, 24, 25, 27–31.

**Second-review coverage:**
- BOS-repeats + §6.3 table → Tasks 9–10 (marks every cleared level; trend stays Unknown without a valid low/high) and Task 9's equality-boundary + no-reversal tests.
- `triggered_at_ms` → Task 15 (takes `tick_time_ms`; test asserts it).
- Unconfirmed-extreme obstacle → Task 15 (`zone.window_high`/`window_low`, incremented in Task 16, seeded at the breakout candle in Task 11).
- Separate terminal reasons → Tasks 5, 12 (`TerminalReason` + `ZoneEventReason`).
- `ATR == 0` scoping → Task 16 (skips only BOS/zone creation; test asserts the candle is fully processed).
- Sim fill model → Task 20 (TP at limit, stop at stop-or-gap-tick; multi-candle tick stream; `entry_ms`/`exit_ms`).
- Performance (state shape) → Task 5 (`VecDeque` window + `candle_count`), Task 7 (`AtrState`); `step` cloning is now O(window).
- Bootstrap calendar blocks + timestamps → Task 21.
- 1m aggregation, dataset gaps, fetch hygiene, CI `--all-features` + comment-excluding grep → Tasks 8.1, 17, 24, 25.
- Verification gaps (signal fixture, committed golden hash, edge-case/verdict/perturbation/calibration) → Tasks 27–31.

**Placeholder scan:** no TBD/TODO. Tasks 24 (Steps 3–4), 27 (fixture tuning), 28 (test bodies to fill following given patterns), 29 (one test to rewrite cleanly) and 30–31 are described with concrete patterns rather than fully coded — each builds on code-complete units from earlier tasks. Everything they call is code-complete.

**Type consistency:** `Zone` (id/kind/low/high/atr/breakout_time_ms/zone_candle_time_ms/activated_at_index/state/terminal_reason/consume_reason/attempt_made/window_high/window_low) is identical across Tasks 5, 11–15. `StrategyState` (window/candle_count/prev_close/atr/atr_seed_sum/atr_seen/trend/valid_low_idx/valid_high_idx/swing_*/zones/zones_awaiting_first_tick/…) is defined once in Task 5 and used consistently. `detect_bos(&state, atr_before, cfg) -> Option<(BosKind, u64)>` (Task 9) matches Task 16's call; `apply_bos(state, kind, broken_idx)` (Task 10) matches. `try_entry(state, prev_price, tick_time_ms, price, cfg)` (Task 15) matches Task 16's `on_tick`. `simulate(&Signal, &[Tick], SimParams) -> SimResult` (Task 20) and `TradeRecord { …, entry_time_ms, exit_time_ms }` (Task 21) match Tasks 29–31.

**Known follow-ups (intentional):** the real HTTP call is isolated in Task 24 so tests never hit the network; the `OneMinuteExpansion` source and deep-1m acquisition remain a post-Phase-0 add-on (the `TickSource` trait accommodates it); Task 27's fixture may need value tuning to satisfy all gates.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-10-17-strategy-analyzer-phase0.md`. Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session via `executing-plans`, batch execution with checkpoints.

**Which approach?**

