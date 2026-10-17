# Rust Strategy Analyzer — Phase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `process/strategy-analyzer`, a pure deterministic Rust core implementing the supply-and-demand 4H strategy, plus an offline backtest/replay harness that runs the core identically to how it will run live.

**Architecture:** The crate splits into `strategy/` (pure, I/O-free, `Decimal`-only, no float arithmetic, no wall-clock, no `HashMap` iteration) and `harness/` (feature-gated; owns data fetching, tick generation, position simulation, and reporting). `harness → strategy` is a one-way dependency. The core exposes `step(&StrategyState, Input, &Config) -> Result<(StrategyState, Vec<Output>), StrategyError>` with validate-then-commit semantics.

**Tech Stack:** Rust 1.98.1, `rust_decimal` (+serde), `serde`/`serde_json`, `thiserror`, `sha2`; harness-only: `clap`, `ureq`, `csv`. Spec: `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md`.

**Conventions:** TDD per task (write failing test → run → implement → run → commit). Never use `f32`/`f64` in `strategy/`. Run `cargo test -p strategy-analyzer` from the repo root. Commit prefixes: `feat:`/`test:`/`chore:`/`docs:`.

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

pub type Ms = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candle { pub open_time_ms: Ms, pub open: Decimal, pub high: Decimal, pub low: Decimal, pub close: Decimal }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick { pub time_ms: Ms, pub price: Decimal }

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub kind: ZoneKind,
    pub low: Decimal,
    pub high: Decimal,
    pub atr: Decimal,                 // ATR at activation
    pub breakout_time_ms: Ms,
    pub zone_candle_time_ms: Ms,
    pub activated_at_index: u64,
    pub state: ZoneState,
    pub reason: Option<ConsumeReason>,
    pub attempt_made: bool,
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
    pub candles: Vec<Candle>,
    pub atr: Option<Decimal>,
    pub trend: TrendState,
    pub valid_low: Option<Decimal>,
    pub valid_high: Option<Decimal>,
    pub swing_highs: Vec<Pivot>,
    pub swing_lows: Vec<Pivot>,
    pub zones: Vec<Zone>,
    pub last_candle_time_ms: Option<Ms>,
    pub last_tick_time_ms: Option<Ms>,
    pub last_price: Option<Decimal>,
    pub missed_touch_count: u64,
}

impl StrategyState {
    pub fn new(symbol: &str, schema_version: u32) -> Self {
        StrategyState {
            schema_version, symbol: symbol.to_string(), candles: Vec::new(), atr: None,
            trend: TrendState::Unknown, valid_low: None, valid_high: None,
            swing_highs: Vec::new(), swing_lows: Vec::new(), zones: Vec::new(),
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
pub struct ZoneEvent { pub zone_id: String, pub kind: ZoneEventKind, pub reason: Option<ConsumeReason> }

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
            activated_at_index: 0, state: ZoneState::Active, reason: None, attempt_made: false }
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
    state.last_candle_time_ms = Some(c.open_time_ms);
    state.candles.push(c);
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
        assert_eq!(s.candles.len(), 2);
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
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 1, price: dec("-1") }), &cfg), Err(StrategyError::MalformedTick { .. })));
        let (s, _) = step(&s, Input::PriceTick(Tick { time_ms: 10, price: dec("1.0") }), &cfg).unwrap();
        assert!(matches!(step(&s, Input::PriceTick(Tick { time_ms: 5, price: dec("1.0") }), &cfg), Err(StrategyError::OutOfOrderTick { .. })));
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
pub fn wilder_atr(series: &[(Decimal, Decimal, Decimal)], period: u32) -> Option<Decimal> {
    let p = period as usize;
    if series.len() < p || p == 0 { return None; }
    let mut trs = Vec::with_capacity(series.len());
    for (i, (_, h, l)) in series.iter().enumerate() {
        let pc = if i == 0 { None } else { Some(series[i - 1].0) };
        trs.push(true_range(pc, *h, *l));
    }
    let seed: Decimal = trs[..p].iter().copied().sum::<Decimal>() / Decimal::from(p as u64);
    let mut atr = work(seed);
    for tr in trs.iter().skip(p) {
        atr = work((atr * Decimal::from(p as u64 - 1) + *tr) / Decimal::from(p as u64));
    }
    Some(atr)
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
        let data: Vec<_> = (0..13).map(|_| (dec("1"), dec("2"), dec("0.5"))).collect();
        assert_eq!(wilder_atr(&data, 14), None);
    }

    #[test]
    fn flat_series_yields_zero_atr_not_error() {
        let data: Vec<_> = (0..20).map(|_| (dec("1"), dec("1"), dec("1"))).collect();
        assert_eq!(wilder_atr(&data, 14), Some(Decimal::ZERO));
    }

    #[test]
    fn constant_range_seeds_to_that_range() {
        let data: Vec<_> = (0..20).map(|_| (dec("1"), dec("2"), dec("0.5"))).collect();
        assert_eq!(wilder_atr(&data, 14), Some(dec("1.5")));
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
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BosKind { Bullish, Bearish }

/// Detect a BOS on the just-closed candle. `atr_before` must come from data
/// strictly before that candle (caller guarantees no look-ahead).
pub fn detect_bos(
    candles: &[Candle],
    swing_highs: &[Pivot],
    swing_lows: &[Pivot],
    atr_before: Decimal,
    cfg: &Config,
) -> Option<(BosKind, usize)> {
    let last = candles.len().checked_sub(1)?;
    let close = candles[last].close;
    let dist = cfg.bos_min_distance_atr * atr_before;
    for (i, hi) in swing_highs.iter().enumerate() {
        if !hi.broken && close >= hi.price + dist { return Some((BosKind::Bullish, i)); }
    }
    for (i, lo) in swing_lows.iter().enumerate() {
        if !lo.broken && close <= lo.price - dist { return Some((BosKind::Bearish, i)); }
    }
    None
}

#[cfg(test)]
mod bos_tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

    fn up_break() -> (Vec<Candle>, Vec<Pivot>) {
        let candles = vec![
            Candle { open_time_ms: 0, open: dec("1"), high: dec("2"), low: dec("1"), close: dec("1.5") },
            Candle { open_time_ms: 4*3600*1000, open: dec("1.5"), high: dec("2.5"), low: dec("1.4"), close: dec("2.6") },
        ];
        let highs = vec![Pivot { index: 0, time_ms: 0, price: dec("2.0"), broken: false }];
        (candles, highs)
    }

    #[test]
    fn bullish_bos_when_close_exceeds_high_plus_distance() {
        let (candles, highs) = up_break();
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(detect_bos(&candles, &highs, &[], dec("1.0"), &cfg), Some((BosKind::Bullish, 0)));
    }

    #[test]
    fn no_bos_when_distance_not_met() {
        let (candles, highs) = up_break();
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(detect_bos(&candles, &highs, &[], dec("100"), &cfg), None);
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
use crate::strategy::types::{StrategyState, TrendState};

/// Apply a BOS: set trend, mark the broken pivot, set valid_low/valid_high per spec §6.3.
pub fn apply_bos(state: &mut StrategyState, kind: BosKind, pivot_idx: usize) {
    let breakout_idx = state.candles.len() as u64 - 1;
    match kind {
        BosKind::Bullish => {
            let broken_high = state.swing_highs[pivot_idx].clone();
            state.swing_highs[pivot_idx].broken = true;
            state.trend = TrendState::Bullish;
            state.valid_low = state.swing_lows.iter()
                .filter(|l| l.index > broken_high.index && l.index < breakout_idx).last().map(|l| l.price)
                .or_else(|| state.swing_lows.iter().filter(|l| l.index < broken_high.index).last().map(|l| l.price));
        }
        BosKind::Bearish => {
            let broken_low = state.swing_lows[pivot_idx].clone();
            state.swing_lows[pivot_idx].broken = true;
            state.trend = TrendState::Bearish;
            state.valid_high = state.swing_highs.iter()
                .filter(|h| h.index > broken_low.index && h.index < breakout_idx).last().map(|h| h.price)
                .or_else(|| state.swing_highs.iter().filter(|h| h.index < broken_low.index).last().map(|h| h.price));
        }
    }
}

#[cfg(test)]
mod trend_tests {
    use super::*;
    use crate::strategy::types::{Candle, Pivot, StrategyState};
    use rust_decimal::Decimal;

    fn flat_candles(n: usize) -> Vec<Candle> {
        (0..n).map(|i| Candle { open_time_ms: i as i64, open: Decimal::ONE, high: Decimal::ONE, low: Decimal::ZERO, close: Decimal::ONE }).collect()
    }

    #[test]
    fn bullish_bos_sets_trend_and_valid_low_after_broken_high() {
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.candles = flat_candles(5); // breakout_idx = 4
        s.swing_highs = vec![Pivot { index: 1, time_ms: 1, price: Decimal::from(2), broken: false }];
        s.swing_lows = vec![
            Pivot { index: 0, time_ms: 0, price: Decimal::from(1), broken: false },
            Pivot { index: 3, time_ms: 3, price: Decimal::from(2), broken: false },
        ];
        apply_bos(&mut s, BosKind::Bullish, 0);
        assert_eq!(s.trend, TrendState::Bullish);
        assert_eq!(s.valid_low, Some(Decimal::from(2))); // the low after the broken high
        assert!(s.swing_highs[0].broken);
    }

    #[test]
    fn valid_low_falls_back_to_before_broken_high() {
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.candles = flat_candles(5);
        s.swing_highs = vec![Pivot { index: 3, time_ms: 3, price: Decimal::from(9), broken: false }];
        s.swing_lows = vec![Pivot { index: 1, time_ms: 1, price: Decimal::from(5), broken: false }];
        apply_bos(&mut s, BosKind::Bullish, 0);
        assert_eq!(s.valid_low, Some(Decimal::from(5)));
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

/// Build a zone after a BOS. `Err(reason)` is a rejection tag; `Ok(zone)` otherwise.
pub fn create_zone(
    candles: &[Candle],
    kind: BosKind,
    atr: Decimal,
    symbol: &str,
    cfg: &Config,
) -> Result<Zone, &'static str> {
    if atr <= Decimal::ZERO { return Err("atr_zero"); }
    let breakout = candles.len() - 1;
    let lookback_start = breakout.saturating_sub(cfg.zone_search_lookback as usize);

    let mut chosen = None;
    for i in (lookback_start..breakout).rev() {
        let c = &candles[i];
        let opposite = match kind { BosKind::Bullish => c.close < c.open, BosKind::Bearish => c.close > c.open };
        if opposite { chosen = Some(i); break; }
    }
    let zi = chosen.ok_or("no_opposite_candle")?;

    let body = (candles[breakout].close - candles[breakout].open).abs();
    if body < cfg.impulse_body_threshold_atr * atr { return Err("weak_impulse"); }

    let zc = &candles[zi];
    let (low, high) = (zc.low, zc.high);

    let height_atr = (high - low) / atr;
    if height_atr < cfg.min_zone_height_atr { return Err("zone_too_small"); }
    if height_atr > cfg.max_zone_height_atr { return Err("zone_too_large"); }

    let start = zi + 1 + cfg.mitigation_grace_candles as usize;
    for c in &candles[start.min(breakout)..breakout] {
        let mitigated = match kind { BosKind::Bullish => c.low <= high, BosKind::Bearish => c.high >= low };
        if mitigated { return Err("mitigated"); }
    }

    let zone_kind = match kind { BosKind::Bullish => ZoneKind::Demand, BosKind::Bearish => ZoneKind::Supply };
    let kind_tag = match zone_kind { ZoneKind::Demand => "demand", ZoneKind::Supply => "supply" };
    let id = format!("{}:{}:{}:{}", symbol, candles[breakout].open_time_ms, zc.open_time_ms, kind_tag);
    Ok(Zone {
        id, kind: zone_kind, low, high, atr,
        breakout_time_ms: candles[breakout].open_time_ms,
        zone_candle_time_ms: zc.open_time_ms,
        activated_at_index: breakout as u64,
        state: ZoneState::Active, reason: None, attempt_made: false,
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
    fn creates_demand_zone() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = create_zone(&base(), BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap();
        assert_eq!(z.kind, ZoneKind::Demand);
        assert_eq!(z.low, dec("0.98"));
        assert_eq!(z.high, dec("1.06"));
        assert_eq!(z.id, format!("XRP-USDT:{}:{}:demand", base()[4].open_time_ms, base()[1].open_time_ms));
    }

    #[test]
    fn rejects_when_mitigated() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut c = base();
        c[3] = candle("1.04","1.10","1.05","1.09"); // low 1.05 <= zone high 1.06 -> mitigated
        assert_eq!(create_zone(&c, BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap_err(), "mitigated");
    }

    #[test]
    fn rejects_atr_zero() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(create_zone(&base(), BosKind::Bullish, dec("0"), "XRP-USDT", &cfg).unwrap_err(), "atr_zero");
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
use crate::strategy::types::{ConsumeReason, TrendState, ZoneEvent, ZoneEventKind, ZoneState as ZS};

/// Expiry + close-beyond-edge invalidation for the just-closed candle.
pub fn update_lifecycle(zones: &mut [Zone], candle: &Candle, idx: u64, expiry_candles: u32) -> Vec<ZoneEvent> {
    let mut events = Vec::new();
    for z in zones.iter_mut() {
        if z.state != ZS::Active { continue; }
        if idx.saturating_sub(z.activated_at_index) >= expiry_candles as u64 {
            z.state = ZS::Expired; z.reason = Some(ConsumeReason::ExpiredUndelivered);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Expired, reason: z.reason });
            continue;
        }
        let invalid = match z.kind { ZoneKind::Demand => candle.close < z.low, ZoneKind::Supply => candle.close > z.high };
        if invalid {
            z.state = ZS::Invalidated; z.reason = Some(ConsumeReason::MissedTouch);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Invalidated, reason: z.reason });
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
            z.state = ZS::Invalidated; z.reason = Some(ConsumeReason::EngineRejectedStrategy);
            events.push(ZoneEvent { zone_id: z.id.clone(), kind: ZoneEventKind::Invalidated, reason: z.reason });
        }
    }
    events
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn zone(kind: ZoneKind) -> Zone {
        Zone { id: "z".into(), kind, low: dec("1.0"), high: dec("1.1"), atr: dec("0.05"),
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0,
            state: ZS::Active, reason: None, attempt_made: false }
    }
    fn c(open: &str, high: &str, low: &str, close: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(open), high: dec(high), low: dec(low), close: dec(close) }
    }

    #[test]
    fn expires_after_window() {
        let mut zs = vec![zone(ZoneKind::Demand)];
        let ev = update_lifecycle(&mut zs, &c("1","1","1","1"), 30, 30);
        assert_eq!(zs[0].state, ZS::Expired);
        assert_eq!(ev.len(), 1);
    }

    #[test]
    fn invalidates_demand_on_close_below_low() {
        let mut zs = vec![zone(ZoneKind::Demand)];
        update_lifecycle(&mut zs, &c("1.0","1.0","0.9","0.95"), 1, 30);
        assert_eq!(zs[0].state, ZS::Invalidated);
    }

    #[test]
    fn trend_flip_invalidates_opposite_zones() {
        let mut zs = vec![zone(ZoneKind::Demand), zone(ZoneKind::Supply)];
        invalidate_on_flip(&mut zs, TrendState::Bullish);
        assert_eq!(zs[0].state, ZS::Active);
        assert_eq!(zs[1].state, ZS::Invalidated);
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
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0, state: ZS::Active, reason: None, attempt_made: false }
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
            state: ZoneState::Active, reason: None, attempt_made: false }
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
        // trigger rounds to 2.00 at scale 2; levels must be derived from the rounded values
        let l = compute_levels(Direction::Long, &z, dec("2.001"), Some(dec("2.500")), 2, &cfg).unwrap();
        let t = dec("2.00");
        let risk = t - l.stop_loss;
        assert_eq!(l.gross_rr, (l.take_profit - t) / risk);
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

- [ ] **Step 1: Add first-observation tracking**

In `types.rs`, add to `StrategyState` the field `pub zones_awaiting_first_tick: Vec<String>` and initialize it to `Vec::new()` in `new()`. (Only `new()` constructs state in tests, so nothing else breaks.)

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
pub fn try_entry(state: &mut StrategyState, prev_price: Option<Decimal>, price: Decimal, cfg: &Config) -> Vec<Output> {
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
                z.state = ZoneState::Consumed; z.reason = Some(ConsumeReason::MissedTouch);
            }
            state.missed_touch_count += 1;
            out.push(Output::ZoneEvent(ZoneEvent { zone_id: zone_snapshot.id.clone(), kind: ZoneEventKind::Consumed, reason: Some(ConsumeReason::MissedTouch) }));
            return out;
        }
    }

    let prev = match prev_price { Some(p) => p, None => return out };
    if !is_first_touch(dir, &zone_snapshot, prev, price) { return out; }

    // Obstacle from swing levels + active opposite zones + window extremes (Task 16 supplies window).
    let (swing_highs, swing_lows) = (&state.swing_highs, &state.swing_lows);
    let (active_supply, active_demand): (Vec<Zone>, Vec<Zone>) = (
        state.zones.iter().filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Supply).cloned().collect(),
        state.zones.iter().filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Demand).cloned().collect(),
    );
    let obstacle = match dir {
        Direction::Long => obstacle_for_long(price, swing_highs, &active_supply, price), // window_high filled in Task 16
        Direction::Short => obstacle_for_short(price, swing_lows, &active_demand, price),
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
        triggered_at_ms: state.last_tick_time_ms.unwrap_or(0),
        candle_timestamp_ms: state.last_candle_time_ms.unwrap_or(0),
    };
    if let Some(z) = state.zones.iter_mut().find(|z| z.id == zone_snapshot.id) {
        z.state = ZoneState::Consumed; z.reason = Some(ConsumeReason::SignalPublished); z.attempt_made = true;
    }
    out.push(Output::EntrySignal(signal));
    out
}
```

Modify `step.rs::on_tick` to call it:
```rust
    let outputs = crate::strategy::signals::try_entry(state, state.last_price, t.price, cfg);
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
            breakout_time_ms:0, zone_candle_time_ms:0, activated_at_index:0, state: ZoneState::Active, reason:None, attempt_made:false }
    }

    #[test]
    fn first_touch_requires_crossing_from_above() {
        assert!(is_first_touch(Direction::Long, &demand(), dec("2.01"), dec("2.00")));
        assert!(!is_first_touch(Direction::Long, &demand(), dec("2.01"), dec("2.02")));
    }

    #[test]
    fn emits_signal_on_first_touch() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.zones = vec![demand()];
        s.swing_highs = vec![Pivot { index:0, time_ms:0, price: dec("2.30"), broken:false }];
        let _ = try_entry(&mut s, Some(dec("2.05")), dec("2.00"), &cfg);
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
        let out = try_entry(&mut s, Some(dec("2.10")), dec("1.98"), &cfg);
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

Rewrite `on_candle` to:
1. record the candle (validation as before);
2. compute `atr_now = wilder_atr(all candles)`; if `Some(0)` emit `Output::StateChange { reason: "atr_zero" }` and return (candle still recorded);
3. detect newly confirmed pivots and push them;
4. compute `atr_before = wilder_atr(candles[..len-1])` (for BOS distance and zone `A`); if `Some`, run `detect_bos`; on a BOS: `apply_bos`, `invalidate_on_flip`, then `create_zone(... `; push the zone or emit `StateChange { reason: <rejection tag> }`;
5. for each newly created zone, push its id to `state.zones_awaiting_first_tick`;
6. run `update_lifecycle`;
7. set `state.atr = atr_now`; push any events.

Add tests to `step.rs`:
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
            assert_eq!(s.candles.len(), (i + 1) as usize); // candle still recorded
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

pub fn load_csv(path: &str) -> Result<Vec<Candle>, String> {
    let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for rec in rdr.records() {
        let r = rec.map_err(|e| e.to_string())?;
        out.push(Candle {
            open_time_ms: r[0].parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
            open: Decimal::from_str(&r[1]).map_err(|e| e.to_string())?,
            high: Decimal::from_str(&r[2]).map_err(|e| e.to_string())?,
            low: Decimal::from_str(&r[3]).map_err(|e| e.to_string())?,
            close: Decimal::from_str(&r[4]).map_err(|e| e.to_string())?,
        });
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
    let mut out = vec![
        Tick { time_ms: c.open_time_ms, price: c.open },
        Tick { time_ms: c.open_time_ms + 1, price: a },
        Tick { time_ms: c.open_time_ms + 2, price: b },
        Tick { time_ms: c.open_time_ms + 3, price: c.close },
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
            e.push(Input::PriceTick(Tick { time_ms: c.open_time_ms, price: c.open }));
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

Create `tests/golden/outs.hash` as an empty file (so the path exists; the test fills it on first run and CI then compares).

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

/// Simulate one signal along the tick path, returning a realised R multiple.
pub fn simulate(sig: &Signal, ticks: &[Tick], params: SimParams, days_held: Decimal) -> SimOutcome {
    let risk = match sig.direction { Direction::Long => sig.trigger_price - sig.stop_loss, Direction::Short => sig.stop_loss - sig.trigger_price };
    if risk <= Decimal::ZERO { return SimOutcome::StopLoss(Decimal::ZERO); }
    let idx = params.latency_ticks as usize;
    if idx >= ticks.len() { return SimOutcome::Unresolved; }
    let raw = ticks[idx].price;
    let slip = raw * params.entry_slip_bps / Decimal::from(10_000);
    let e = match sig.direction { Direction::Long => raw + slip, Direction::Short => raw - slip };
    let cost = e * (params.fee_bps * Decimal::from(2) + params.borrow_bps_per_day * days_held) / Decimal::from(10_000);

    for t in &ticks[idx..] {
        match sig.direction {
            Direction::Long => {
                if t.price <= sig.stop_loss {
                    let s = t.price * params.stop_slip_bps / Decimal::from(10_000);
                    return SimOutcome::StopLoss(-((e - t.price + s) + cost) / risk);
                }
                if t.price >= sig.take_profit {
                    return SimOutcome::TakeProfit(((t.price - e) - cost) / risk);
                }
            }
            Direction::Short => {
                if t.price >= sig.stop_loss {
                    let s = t.price * params.stop_slip_bps / Decimal::from(10_000);
                    return SimOutcome::StopLoss(-((t.price - e + s) + cost) / risk);
                }
                if t.price <= sig.take_profit {
                    return SimOutcome::TakeProfit(((e - t.price) - cost) / risk);
                }
            }
        }
    }
    SimOutcome::Unresolved
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
    fn hits_take_profit() {
        let t = vec![Tick{time_ms:0,price:dec("2.00")}, Tick{time_ms:1,price:dec("2.00")}, Tick{time_ms:2,price:dec("2.20")}];
        assert!(matches!(simulate(&long_sig(), &t, p(), dec("0")), SimOutcome::TakeProfit(_)));
    }

    #[test]
    fn hits_stop() {
        let t = vec![Tick{time_ms:0,price:dec("2.00")}, Tick{time_ms:1,price:dec("2.00")}, Tick{time_ms:2,price:dec("1.90")}];
        assert!(matches!(simulate(&long_sig(), &t, p(), dec("0")), SimOutcome::StopLoss(_)));
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
pub struct TradeRecord { pub direction: Direction, pub r: Decimal, pub pair: String }

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
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

/// Calendar-block bootstrap: resample contiguous blocks of `block` trades and take
/// the 95% lower bound (5th percentile) of the resampled means. Seeded and deterministic.
pub fn time_blocked_bootstrap(trades: &[TradeRecord], block: usize, resamples: usize, seed: u64) -> (f64, f64) {
    if trades.is_empty() { return (0.0, 0.0); }
    let vals: Vec<f64> = trades.iter().map(|t| t.r.to_string().parse::<f64>().unwrap()).collect();
    let mean0 = vals.iter().sum::<f64>() / vals.len() as f64;
    let n_blocks = (vals.len() + block - 1) / block;
    let mut rng = Lcg(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut acc = 0.0;
        let mut cnt = 0usize;
        for _ in 0..n_blocks {
            let b = rng.below(n_blocks);
            for k in 0..block {
                if let Some(v) = vals.get(b * block + k) { acc += v; cnt += 1; }
            }
        }
        means.push(acc / cnt as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lower = means[(means.len() as f64 * 0.05) as usize];
    (mean0, lower)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn t(r: &str, p: &str) -> TradeRecord { TradeRecord { direction: Direction::Long, r: Decimal::from_str(r).unwrap(), pair: p.into() } }

    #[test]
    fn summary_counts_and_splits() {
        let s = summarize(&[t("3.0","A"), t("-1.0","A"), t("3.0","B")]);
        assert_eq!(s.n, 3);
        assert!(s.mean_r > 1.6 && s.mean_r < 1.7);
        assert_eq!(s.by_pair.len(), 2);
    }

    #[test]
    fn bootstrap_is_deterministic() {
        let data: Vec<_> = (0..200).map(|i| t(if i % 3 == 0 { "-1.0" } else { "3.0" }, "A")).collect();
        assert_eq!(time_blocked_bootstrap(&data, 14, 500, 42), time_blocked_bootstrap(&data, 14, 500, 42));
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
Wire `FetchData` in `bin/backtest.rs` to page via `page_windows`, call `fetch_page` with a small `std::thread::sleep` between pages, write the cache CSV (`open_time_ms,open,high,low,close`; note KuCoin order is `time,open,close,high,low,...`), then print `dataset_hash(out)`.

- [ ] **Step 4: Implement the full `Backtest` command**

Extend the `Backtest` arm to: load the dataset; compute `config_version`; if `--oos`, enforce `lock::check_lock`, and on `--oos-consumed` write the consumed marker + stamp the report non-confirmatory; run both policies × both target modes through the core; feed signals to `sim::simulate` with pre-registered `SimParams` (skip a signal while a position is open on the symbol → count `skipped_open_position`); accumulate `TradeRecord`s; print the §8.5 header (symbols, resolution, dataset hash, `config_version`, policy, mode, caveat) and the report (mode/path side-by-side, long/short + per-pair splits, cushion calibration, bootstrap bound). Diagnostics (OAT ±20% + 200-draw joint perturbation) run **on 4H synthesis only**, within a wall-clock budget.

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
	@! grep -RInE '\b(f32|f64)\b' process/strategy-analyzer/src/strategy --include='*.rs' || (echo "float type in core" && exit 1)
	@! grep -RInE '\b(HashMap|HashSet)\b' process/strategy-analyzer/src/strategy --include='*.rs' || (echo "hash-map in core" && exit 1)
	cargo build -p strategy-analyzer --no-default-features
```

Add `strategy-check` to the `ci` target (after `test`), and to `.PHONY`.

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

## Self-Review

**Spec coverage:**
- §1 layout/features → Task 1; §2 numeric policy → Tasks 2, 4, 16; §3 config/validation → Task 4; §4 I/O/errors/ordering/per-symbol → Tasks 5, 6, 15, 16; §5 structure → Tasks 8–10; §6 zones/lifecycle → Tasks 11–13; §7 signals/levels/obstacle/modes → Tasks 14–15; §8 harness (sources, paging, policies, sim, reporting) → Tasks 17–24; §9 payload → Tasks 5, 16; §10 tests/bar/lock → Tasks 19, 21, 22, 24; §11–13 → logs/README + deferred list.
- **Gap addressed:** §8.2's real network fetch is implemented in Task 24 Step 3 (`fetch_page` via `ureq`, throttled, isolated from the core).
- **Deliberate simplifications (flagged):** the harness's `fetch_page` returns raw strings and the caller maps the KuCoin column order; the perturbation diagnostics live in Task 24 Step 4 and run on 4H synthesis only.

**Placeholder scan:** no TBD/TODO. Task 24 Steps 3–4 describe work without full code (it is integration glue over already-tested units) — implementers should build on the tested `fetch::page_windows`, `sim::simulate`, and `report` functions, not invent new logic. This is the only place a step is described rather than fully coded; everything it calls is code-complete in earlier tasks.

**Type consistency:** `Zone` (id/kind/low/high/atr/breakout_time_ms/zone_candle_time_ms/activated_at_index/state/reason/attempt_made) is identical in Tasks 5, 11–15. `StrategyState` gains exactly one field in Task 15 (`zones_awaiting_first_tick`), initialized in `new()`. `Signal` fields (Task 5) match the assertions in Tasks 16 and the fixtures in Tasks 20–21. `TargetMode`, `PathPolicy`, `BosKind`, `SimParams`, `SimOutcome`, `TradeRecord`, `Summary` are each defined once and reused. `bos_min_distance_atr`/`impulse_body_threshold_atr`/`stop_buffer_atr`/`structural_buffer_atr`/`min_zone_height_atr`/`max_zone_height_atr` names are consistent between `Config` (Task 4) and their uses (Tasks 9, 11, 14).

**Known follow-ups (intentional):** the real HTTP call is isolated in Task 24 so tests never hit the network; deep-1m acquisition and the `OneMinuteExpansion` source are a post-Phase-0 add-on (the `TickSource` trait already accommodates them).

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-10-17-strategy-analyzer-phase0.md`. Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session via `executing-plans`, batch execution with checkpoints.

**Which approach?**

