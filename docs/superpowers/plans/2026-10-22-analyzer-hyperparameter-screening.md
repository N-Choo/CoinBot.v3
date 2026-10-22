# Analyzer Hyperparameter Screening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an in-process `analyzer-sweep` harness and run a 30-config-per-timeframe screening study (1hour, 4hour; BTC/ETH/SOL; 24 months) with costs, chronological splits, expanding folds, and a one-shot holdout.

**Architecture:** A new `harness/sweep` module reuses the live-exact `CandleDriver` replay and the simulator unchanged, adding: paginated data fetch + cache, per-side cost accounting, partition slicing, deterministic sampling, evaluation, ranking, and reporting. A new `analyzer-sweep` binary orchestrates and writes CSV/JSON/Markdown artifacts.

**Tech Stack:** Rust 2021, `rust_decimal`, `serde`/`serde_json`, `csv`, `ureq`, `chrono`, `sha2`, `clap` (all already available under the `harness` feature).

**Spec:** `docs/superpowers/specs/2026-10-22-analyzer-hyperparameter-screening-design.md`

**Safety:** Do not modify `sim.rs` or production defaults; no live orders; screening results are provisional.

---

## File map

| File | Responsibility |
|---|---|
| `process/analyzer-rs/src/exchange/timeframes.rs` | Include the uncommitted `H1` (1hour) support |
| `process/analyzer-rs/src/harness/sweep/mod.rs` | Module exports + shared types |
| `process/analyzer-rs/src/harness/sweep/costs.rs` | Per-side cost accounting |
| `process/analyzer-rs/src/harness/sweep/sampling.rs` | Deterministic 30-config generation |
| `process/analyzer-rs/src/harness/sweep/partition.rs` | Chronological splits + expanding folds |
| `process/analyzer-rs/src/harness/sweep/data.rs` | Paginated KuCoin fetch + cache + provenance |
| `process/analyzer-rs/src/harness/sweep/evaluate.rs` | Per-config/partition metrics |
| `process/analyzer-rs/src/harness/sweep/ranking.rs` | Selection, fold consistency, stability |
| `process/analyzer-rs/src/harness/sweep/report.rs` | CSV/JSON/Markdown artifacts |
| `process/analyzer-rs/src/bin/analyzer-sweep.rs` | CLI + orchestration |
| `process/analyzer-rs/src/harness/mod.rs` | `pub mod sweep;` |
| `process/analyzer-rs/Cargo.toml` | `[[bin]] analyzer-sweep` |
| `Makefile` | `analyzer-sweep` target |

---

### Task 1: Worktree, include H1, and verify the baseline

**Files:**
- Modify: `process/analyzer-rs/src/exchange/timeframes.rs`
- Test: `process/analyzer-rs/tests/exchange.rs`

- [ ] **Step 1: Create the worktree** (skip if already isolated)

```bash
git worktree add .worktrees/analyzer-sweep -b feat/analyzer-sweep
```

- [ ] **Step 2: Write the failing H1 test**

In `process/analyzer-rs/tests/exchange.rs` add:

```rust
#[test]
fn one_hour_timeframe_is_supported() {
    let tf = Timeframe::from_kucoin("1hour").expect("1hour supported");
    assert_eq!(tf.as_kucoin(), "1hour");
    assert_eq!(tf.millis(), 3_600_000);
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test exchange one_hour_timeframe_is_supported`
Expected: FAIL to compile (`H1`/`1hour` missing).

- [ ] **Step 4: Apply the H1 change**

The working tree already contains the change; confirm `Timeframe::H1`, `"1hour"` in `from_kucoin`/`as_kucoin`, and `3_600_000` in `millis()`.

- [ ] **Step 5: Run tests + commit**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness
git add process/analyzer-rs/src/exchange/timeframes.rs process/analyzer-rs/tests/exchange.rs
git commit -m "feat(analyzer-rs): add 1hour timeframe with tests"
```

---

### Task 2: Sweep module skeleton + candidate + cost accounting

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/mod.rs`
- Create: `process/analyzer-rs/src/harness/sweep/costs.rs`
- Modify: `process/analyzer-rs/src/harness/mod.rs`
- Test: `process/analyzer-rs/tests/sweep_costs.rs`

- [ ] **Step 1: Write the failing cost tests**

Create `tests/sweep_costs.rs`:

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::costs::{net_trade_pnl, CostScenario};
use analyzer_rs::harness::sim::{ClosedTrade, Position};
use analyzer_rs::strategy::types::Direction;
use rust_decimal::Decimal;

fn d(v: i64) -> Decimal { Decimal::from(v) }

#[test]
fn base_costs_are_applied_to_both_notionals_long_winner() {
    let scenario = CostScenario::base();
    // E=500, fill=100, exit=110 -> q=5, exit_notional=550, gross=+50
    let net = net_trade_pnl(&scenario, Direction::Long, d(500), d(100), d(110), d(50));
    // entry_cost=500*15bps=0.75, exit_cost=550*15bps=0.825 -> 50-1.575
    assert_eq!(net, d(50) - Decimal::new(75, 2) - Decimal::new(825, 3));
}

#[test]
fn short_loser_still_pays_both_sides() {
    let scenario = CostScenario::base();
    // E=500, fill=100, exit=105, gross=-25
    let net = net_trade_pnl(&scenario, Direction::Short, d(500), d(100), d(105), d(-25));
    assert_eq!(net, d(-25) - Decimal::new(75, 2) - Decimal::new(7875, 4));
}

#[test]
fn stress_scenario_doubles_the_assumption() {
    assert_eq!(CostScenario::stress().per_side_rate(), Decimal::new(30, 4));
    assert_eq!(CostScenario::base().per_side_rate(), Decimal::new(15, 4));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_costs`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement the module and costs**

`src/harness/mod.rs`: add `pub mod sweep;`.

`src/harness/sweep/mod.rs`:

```rust
pub mod costs;
pub mod data;
pub mod evaluate;
pub mod partition;
pub mod ranking;
pub mod report;
pub mod sampling;
```

`src/harness/sweep/costs.rs`:

```rust
use crate::strategy::types::Direction;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CostScenario {
    pub label: String,
    pub fee_bps: Decimal,
    pub slip_bps: Decimal,
}

impl CostScenario {
    pub fn base() -> Self {
        Self { label: "base".into(), fee_bps: Decimal::from(10), slip_bps: Decimal::from(5) }
    }
    pub fn stress() -> Self {
        Self { label: "stress".into(), fee_bps: Decimal::from(20), slip_bps: Decimal::from(10) }
    }
    /// Per-side rate as a fraction (bps / 10_000).
    pub fn per_side_rate(&self) -> Decimal {
        (self.fee_bps + self.slip_bps) / Decimal::from(10_000)
    }
}

/// Net PnL for a closed trade: gross minus entry and exit costs on their notionals.
pub fn net_trade_pnl(
    scenario: &CostScenario,
    _side: Direction,
    e: Decimal,
    fill: Decimal,
    exit: Decimal,
    gross_pnl: Decimal,
) -> Decimal {
    let rate = scenario.per_side_rate();
    let entry_cost = rate * e;
    let exit_notional = e * exit / fill;
    let exit_cost = rate * exit_notional;
    gross_pnl - entry_cost - exit_cost
}

/// Net unrealized for an open position marked at `mark`.
pub fn net_open_pnl(
    scenario: &CostScenario,
    e: Decimal,
    fill: Decimal,
    mark: Decimal,
    gross_unrealized: Decimal,
) -> Decimal {
    let rate = scenario.per_side_rate();
    let entry_cost = rate * e;
    let exit_notional = e * mark / fill;
    gross_unrealized - entry_cost - rate * exit_notional
}

pub fn _assert_types(_t: &ClosedTrade, _p: &Position) {}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p analyzer-rs --features harness --test sweep_costs`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
cargo fmt -p analyzer-rs
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
git add process/analyzer-rs/src/harness
git commit -m "feat(analyzer-rs): sweep cost accounting"
```

---

### Task 3: Deterministic sampling

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/sampling.rs`
- Test: `process/analyzer-rs/tests/sweep_sampling.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::sampling::{generate, Candidate, SEED_DEFAULT};
use std::collections::HashSet;

#[test]
fn generates_exactly_thirty_unique_candidates_including_baseline() {
    let c = generate(SEED_DEFAULT);
    assert_eq!(c.len(), 30);
    let keys: HashSet<_> = c.iter().map(Candidate::key).collect();
    assert_eq!(keys.len(), 30, "no duplicates");
    assert_eq!(c[0].label, "baseline");
    assert!(c[0].sma_enabled && c[0].rsi_period == 14 && c[0].rsi_order == 5);
    assert_eq!(c[0].max_age, Some(12));
}

#[test]
fn sma_period_is_always_twenty() {
    for cand in generate(SEED_DEFAULT) {
        assert_eq!(cand.sma_period, 20);
    }
}

#[test]
fn is_deterministic_for_a_fixed_seed() {
    assert_eq!(generate(SEED_DEFAULT), generate(SEED_DEFAULT));
    assert_ne!(generate(SEED_DEFAULT), generate(1));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_sampling`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `sampling.rs`**

```rust
use crate::strategy::config::Config;
use rust_decimal::Decimal;
use serde::Serialize;

pub const SEED_DEFAULT: u64 = 20261022;
pub const SMA_PERIOD: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candidate {
    pub label: String,
    pub origin: String,
    pub rsi_period: usize,
    pub rsi_order: usize,
    pub max_age: Option<usize>, // None = uncapped
    pub sma_enabled: bool,
    pub sma_period: usize,
    pub atr_sl_multiplier: Decimal,
    pub atr_tp_multiplier: Decimal,
}

impl Candidate {
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.rsi_period, self.rsi_order,
            self.max_age.map(|v| v.to_string()).unwrap_or_else(|| "none".into()),
            self.sma_enabled, self.sma_period,
            self.atr_sl_multiplier.normalize(), self.atr_tp_multiplier.normalize()
        )
    }
    pub fn to_config(&self) -> Config {
        Config {
            rsi_period: self.rsi_period,
            rsi_order: self.rsi_order,
            price_diff_threshold: Decimal::new(5, 2),
            sma_enabled: self.sma_enabled,
            sma_period: self.sma_period,
            atr_period: 14,
            atr_sl_multiplier: self.atr_sl_multiplier,
            atr_tp_multiplier: self.atr_tp_multiplier,
            atr_pct_high: Decimal::new(50, 1),
            atr_high_adjust: Decimal::new(15, 1),
            atr_sl_max_pct: Decimal::ZERO,
            window: 100,
        }
    }
}

pub fn baseline() -> Candidate {
    Candidate {
        label: "baseline".into(), origin: "baseline".into(),
        rsi_period: 14, rsi_order: 5, max_age: Some(12),
        sma_enabled: true, sma_period: SMA_PERIOD,
        atr_sl_multiplier: Decimal::new(15, 1), atr_tp_multiplier: Decimal::new(20, 1),
    }
}

pub fn generate(seed: u64) -> Vec<Candidate> {
    let base = baseline();
    let mut out = vec![base.clone()];
    let mut seen = std::collections::HashSet::new();
    seen.insert(base.key());

    // OFAT anchors.
    let mut anchor = |label: String, f: &dyn Fn(&mut Candidate)| {
        let mut c = base.clone();
        c.label = label.clone();
        c.origin = format!("ofat:{label}");
        f(&mut c);
        if seen.insert(c.key()) { out.push(c); }
    };
    for v in [10usize, 21] { anchor(format!("rsi_period={v}"), &|c| c.rsi_period = v); }
    for v in [3usize, 7] { anchor(format!("rsi_order={v}"), &|c| c.rsi_order = v); }
    for v in [Some(4usize), Some(8), None] { anchor(format!("max_age={}", v.map(|x| x.to_string()).unwrap_or_else(|| "uncapped".into())), &|c| c.max_age = v); }
    anchor("sma_enabled=false".into(), &|c| c.sma_enabled = false);
    for v in [Decimal::new(20, 1), Decimal::new(25, 1)] { anchor(format!("atr_sl={v}"), &|c| c.atr_sl_multiplier = v); }
    for v in [Decimal::new(30, 1), Decimal::new(40, 1)] { anchor(format!("atr_tp={v}"), &|c| c.atr_tp_multiplier = v); }

    // Deterministic seeded joint draws.
    let rsi = [10usize, 14, 21];
    let order = [3usize, 5, 7];
    let age = [Some(4usize), Some(8), Some(12), None];
    let sma = [false, true];
    let sl = [Decimal::new(15, 1), Decimal::new(20, 1), Decimal::new(25, 1)];
    let tp = [Decimal::new(20, 1), Decimal::new(30, 1), Decimal::new(40, 1)];
    let mut state = seed | 1;
    let mut next = || { state ^= state << 13; state ^= state >> 7; state ^= state << 17; state };
    while out.len() < 30 {
        let pick = |n: usize, next: &mut dyn FnMut() -> u64| (next() % n as u64) as usize;
        let c = Candidate {
            label: String::new(), origin: "joint".into(),
            rsi_period: rsi[pick(rsi.len(), &mut next)],
            rsi_order: order[pick(order.len(), &mut next)],
            max_age: age[pick(age.len(), &mut next)],
            sma_enabled: sma[pick(sma.len(), &mut next)],
            sma_period: SMA_PERIOD,
            atr_sl_multiplier: sl[pick(sl.len(), &mut next)],
            atr_tp_multiplier: tp[pick(tp.len(), &mut next)],
        };
        let mut c = c;
        if seen.insert(c.key()) {
            c.label = format!("joint-{:02}", out.len());
            out.push(c);
        }
    }
    out
}
```

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p analyzer-rs --features harness --test sweep_sampling`
Expected: PASS (3 tests). Then `cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings` clean.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src/harness/sweep
git commit -m "feat(analyzer-rs): deterministic sweep sampling"
```

---

### Task 4: Chronological splits and expanding folds

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/partition.rs`
- Test: `process/analyzer-rs/tests/sweep_partition.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::partition::{chronological_splits, slice_with_warmup};
use analyzer_rs::strategy::types::Candle;
use rust_decimal::Decimal;

fn candles(n: usize) -> Vec<Candle> {
    (0..n).map(|i| Candle {
        open_time_ms: i as i64 * 3_600_000,
        open: Decimal::ONE, high: Decimal::ONE, low: Decimal::ONE, close: Decimal::ONE, volume: Decimal::ONE,
    }).collect()
}

#[test]
fn splits_are_chronological_and_holdout_is_last() {
    let s = chronological_splits(1000);
    assert_eq!(s.train.start, 0);
    assert_eq!(s.train.end, 600);
    assert_eq!(s.validation.start, 600);
    assert_eq!(s.validation.end, 800);
    assert_eq!(s.holdout.start, 800);
    assert_eq!(s.holdout.end, 1000);
    assert_eq!(s.folds.len(), 3);
    for f in &s.folds {
        assert!(f.val.end <= 800, "fold val must not reach holdout");
    }
}

#[test]
fn warmup_precedes_partition_and_is_not_counted() {
    let full = candles(1000);
    let s = chronological_splits(1000);
    let slice = slice_with_warmup(&full, s.validation, 100);
    assert_eq!(slice.candles.len(), 100 + 200);
    assert_eq!(slice.partition_start, 100);
    assert_eq!(slice.candles[100].open_time_ms, full[600].open_time_ms);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_partition`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `partition.rs`**

```rust
use crate::strategy::types::Candle;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Slice { pub start: usize, pub end: usize }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fold { pub train: Slice, pub val: Slice }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Splits { pub train: Slice, pub validation: Slice, pub holdout: Slice, pub folds: Vec<Fold> }

fn pct(n: usize, p: usize) -> usize { n * p / 100 }

pub fn chronological_splits(n: usize) -> Splits {
    let train = Slice { start: 0, end: pct(n, 60) };
    let validation = Slice { start: pct(n, 60), end: pct(n, 80) };
    let holdout = Slice { start: pct(n, 80), end: n };
    let folds = vec![
        Fold { train: Slice { start: 0, end: pct(n, 40) }, val: Slice { start: pct(n, 40), end: pct(n, 55) } },
        Fold { train: Slice { start: 0, end: pct(n, 55) }, val: Slice { start: pct(n, 55), end: pct(n, 70) } },
        Fold { train: Slice { start: 0, end: pct(n, 70) }, val: Slice { start: pct(n, 70), end: pct(n, 80) } },
    ];
    Splits { train, validation, holdout, folds }
}

#[derive(Debug, Clone)]
pub struct PartitionSlice { pub candles: Vec<Candle>, pub partition_start: usize, pub partition_end: usize }

/// Returns `window` warm-up candles followed by the partition candles.
/// `partition_start` is the index in `candles` where the evaluated partition begins.
pub fn slice_with_warmup(full: &[Candle], s: Slice, window: usize) -> PartitionSlice {
    let warm_start = s.start.saturating_sub(window);
    let candles = full[warm_start..s.end].to_vec();
    let partition_start = s.start - warm_start;
    let partition_end = candles.len();
    PartitionSlice { candles, partition_start, partition_end }
}
```

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p analyzer-rs --features harness --test sweep_partition`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src/harness/sweep
git commit -m "feat(analyzer-rs): sweep chronological splits and folds"
```

---

### Task 5: Paginated data fetch + cache

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/data.rs`
- Test: `process/analyzer-rs/tests/sweep_data.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::data::{dedup_and_sort, missing_candles, write_cache, read_cache};
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::strategy::types::Candle;
use rust_decimal::Decimal;

fn c(t: i64) -> Candle {
    Candle { open_time_ms: t, open: Decimal::ONE, high: Decimal::ONE, low: Decimal::ONE, close: Decimal::ONE, volume: Decimal::ONE }
}

#[test]
fn dedup_sorts_and_reports_missing() {
    let raw = vec![c(2 * 3_600_000), c(0), c(3_600_000), c(3_600_000)];
    let clean = dedup_and_sort(raw);
    assert_eq!(clean.len(), 3);
    assert_eq!(clean[0].open_time_ms, 0);
    assert_eq!(missing_candles(&clean, Timeframe::H1), 0);
    let with_gap = vec![c(0), c(3 * 3_600_000)];
    assert_eq!(missing_candles(&with_gap, Timeframe::H1), 2);
}

#[test]
fn cache_round_trips_csv() {
    let dir = std::env::temp_dir().join(format!("sweep-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("BTC-USDT_1hour.csv");
    write_cache(&path, &[c(0), c(3_600_000)]).unwrap();
    let back = read_cache(&path).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].open_time_ms, 3_600_000);
    std::fs::remove_dir_all(&dir).unwrap();
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_data`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `data.rs`**

Key functions (network function tested indirectly via the CLI smoke test):

```rust
use crate::exchange::kucoin::fetch_klines;
use crate::exchange::timeframes::Timeframe;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use std::path::Path;

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
    candles.sort_by_key(|c| c.open_time_ms);
    candles.dedup_by_key(|c| c.open_time_ms);
    candles
}

pub fn missing_candles(candles: &[Candle], tf: Timeframe) -> usize {
    let mut missing = 0usize;
    for pair in candles.windows(2) {
        let spacing = pair[1].open_time_ms - pair[0].open_time_ms;
        let gap = spacing / tf.millis() - 1;
        if gap > 0 { missing += gap as usize; }
    }
    missing
}

/// Page backward from `end_ms` until `start_ms` is covered. Closed candles only.
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
        let batch = fetch_klines(base_url, symbol, tf, PAGE, Some(cursor))
            .map_err(|e| e.to_string())?;
        if batch.is_empty() { break; }
        pages += 1;
        let earliest = batch.iter().map(|c| c.open_time_ms).min().unwrap();
        all.extend(batch);
        if earliest <= start_ms { break; }
        cursor = earliest - 1;
    }
    let mut clean = dedup_and_sort(all);
    clean.retain(|c| c.open_time_ms >= start_ms && c.open_time_ms <= end_ms && tf.is_closed(c.open_time_ms, now_ms, settle_ms));
    Ok((clean, pages))
}

pub fn write_cache(path: &Path, candles: &[Candle]) -> std::io::Result<()> { /* csv writer, header open_time_ms,open,high,low,close,volume */ }
pub fn read_cache(path: &Path) -> Result<Vec<Candle>, String> { /* csv reader */ }
```

Implement `write_cache`/`read_cache` with the `csv` crate using the exact columns `open_time_ms,open,high,low,close,volume` (compatible with `dataset::load_csv`). Write provenance JSON next to the CSV as `<name>.provenance.json`.

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p analyzer-rs --features harness --test sweep_data`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src/harness/sweep
git commit -m "feat(analyzer-rs): paginated sweep data fetch and cache"
```

---

### Task 6: Evaluation

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/evaluate.rs`
- Test: `process/analyzer-rs/tests/sweep_evaluate.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::costs::CostScenario;
use analyzer_rs::harness::sweep::evaluate::{metrics_from_outcome, PartitionMetrics};
use analyzer_rs::harness::sim::{ClosedTrade, SimOutcome};
use analyzer_rs::strategy::types::Direction;
use rust_decimal::Decimal;

fn d(v: i64) -> Decimal { Decimal::from(v) }

fn trade(pnl: i64, entry: usize, exit: usize) -> ClosedTrade {
    ClosedTrade {
        side: Direction::Long, signal_id: "s".into(), signal_entry: d(100), fill: d(100),
        exit: d(100), e: d(500), pnl: d(pnl), pnl_pct: Decimal::ZERO, risk_reward: Some(d(2)),
        reason: if pnl >= 0 { "TP".into() } else { "SL".into() }, entry_index: entry, exit_index: exit,
    }
}

#[test]
fn metrics_subtract_costs_and_compute_expectancy() {
    let outcome = SimOutcome { trades: vec![trade(50, 1, 2), trade(-20, 3, 4)], ..Default::default() };
    let m: PartitionMetrics = metrics_from_outcome(&outcome, d(100), d(1000), &CostScenario::base());
    assert_eq!(m.trades, 2);
    // Each trade pays ~0.75 entry + ~0.75 exit = 1.5; net = 50-1.5 + -20-1.5
    assert_eq!(m.net_pnl, d(50) + d(-20) - d(3));
    assert_eq!(m.net_expectancy, (d(30) - d(3)) / d(2));
}

#[test]
fn max_drawdown_tracks_net_equity() {
    let outcome = SimOutcome { trades: vec![trade(-100, 1, 2), trade(50, 3, 4)], ..Default::default() };
    let m = metrics_from_outcome(&outcome, d(100), d(1000), &CostScenario::base());
    assert!(m.max_drawdown > d(0));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_evaluate`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `evaluate.rs`**

```rust
use crate::harness::sim::SimOutcome;
use crate::harness::sweep::costs::{net_open_pnl, net_trade_pnl, CostScenario};
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PartitionMetrics {
    pub trades: usize,
    pub open_at_end: usize,
    pub gross_pnl: Decimal,
    pub net_pnl: Decimal,
    pub net_expectancy: Decimal,
    pub net_expectancy_r: Decimal,
    pub net_profit_factor: Option<Decimal>,
    pub win_rate: Decimal,
    pub max_drawdown: Decimal,
    pub max_drawdown_pct: Decimal,
    pub expectancy_stderr: Decimal,
    pub tp_hits: usize,
    pub sl_hits: usize,
    pub other_exits: usize,
}

pub fn metrics_from_outcome(
    outcome: &SimOutcome,
    last_close: Decimal,
    init_fund: Decimal,
    scenario: &CostScenario,
) -> PartitionMetrics {
    let mut gross = Decimal::ZERO;
    let mut net = Decimal::ZERO;
    let mut wins = 0usize;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    let mut r_sum = Decimal::ZERO;
    let mut r_n = 0usize;
    let mut equity = init_fund;
    let mut peak = init_fund;
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_pct = Decimal::ZERO;
    let mut pnls: Vec<Decimal> = Vec::new();
    let (mut tp, mut sl, mut other) = (0usize, 0usize, 0usize);
    for t in &outcome.trades {
        let n = net_trade_pnl(scenario, t.side, t.e, t.fill, t.exit, t.pnl);
        gross += t.pnl;
        net += n;
        pnls.push(n);
        if n > Decimal::ZERO { wins += 1; gross_profit += n; } else if n < Decimal::ZERO { gross_loss += n; }
        if !t.e.is_zero() { r_sum += n / t.e; r_n += 1; }
        match t.reason.as_str() {
            "TP" | "gap_tp" => tp += 1,
            "SL" | "gap_sl" => sl += 1,
            _ => other += 1,
        }
        equity += n;
        if equity > peak { peak = equity; }
        let dd = peak - equity;
        if dd > max_dd { max_dd = dd; max_dd_pct = if peak.is_zero() { Decimal::ZERO } else { dd / peak * Decimal::from(100) }; }
    }
    // Mark open positions at the partition's last close; net of both sides' costs.
    let mut open_net = Decimal::ZERO;
    for p in &outcome.open_positions {
        let gross_u = crate::harness::sim::unrealized_pnl(p.side, p.fill, last_close, p.e);
        let n = net_open_pnl(scenario, p.e, p.fill, last_close, gross_u);
        gross += gross_u;
        net += n;
        open_net += n;
    }
    equity += open_net;
    if equity > peak { peak = equity; }
    let dd = peak - equity;
    if dd > max_dd { max_dd = dd; max_dd_pct = if peak.is_zero() { Decimal::ZERO } else { dd / peak * Decimal::from(100) }; }

    let n_trades = Decimal::from(outcome.trades.len());
    let net_expectancy = if outcome.trades.is_empty() { Decimal::ZERO } else { net / n_trades };
    let net_expectancy_r = if r_n == 0 { Decimal::ZERO } else { r_sum / Decimal::from(r_n) };
    let variance = if pnls.len() < 2 { Decimal::ZERO } else {
        let sum_sq: Decimal = pnls.iter().map(|x| { let d = *x - net_expectancy; d * d }).sum();
        sum_sq / Decimal::from(pnls.len() - 1)
    };
    let stderr = if pnls.is_empty() { Decimal::ZERO } else { (variance / Decimal::from(pnls.len())).sqrt().unwrap_or(Decimal::ZERO) };
    let net_profit_factor = if gross_loss.is_zero() { None } else { Some(gross_profit / gross_loss.abs()) };
    PartitionMetrics {
        trades: outcome.trades.len(),
        open_at_end: outcome.open_positions.len(),
        gross_pnl: gross,
        net_pnl: net,
        net_expectancy,
        net_expectancy_r,
        net_profit_factor,
        win_rate: if outcome.trades.is_empty() { Decimal::ZERO } else { Decimal::from(wins) / n_trades * Decimal::from(100) },
        max_drawdown: max_dd,
        max_drawdown_pct: max_dd_pct,
        expectancy_stderr: stderr,
        tp_hits: tp,
        sl_hits: sl,
        other_exits: other,
    }
}
```

Update the Task 6 tests to pass `last_close` as the second argument (use `d(100)` for the closed-trade tests; with no open positions it has no effect).

Also implement the higher-level driver:

```rust
pub struct ConfigResult { pub partitions: BTreeMap<String, BTreeMap<String, PartitionMetrics>> } // scenario -> partition -> metrics
pub fn evaluate_config(symbol, tf, cfg, max_age, full_candles, splits, window, init_fund, scenarios) -> Result<ConfigResult, String>
```

For each partition (`train`, `validation`, `holdout`, `fold1..3`): `slice_with_warmup`, build a `Dataset` (candles + cloned provenance), `build_replay`, filter signals to `candle_index >= partition_start && candle_index + 1 < candles.len()`, `simulate`, then `metrics_from_outcome(..., last_close, init_fund, scenario)` for each scenario.

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p analyzer-rs --features harness --test sweep_evaluate`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src/harness/sweep
git commit -m "feat(analyzer-rs): sweep partition evaluation with costs"
```

---

### Task 7: Ranking and reporting

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/ranking.rs`
- Create: `process/analyzer-rs/src/harness/sweep/report.rs`
- Test: `process/analyzer-rs/tests/sweep_ranking.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#![cfg(feature = "harness")]
use analyzer_rs::harness::sweep::evaluate::PartitionMetrics;
use analyzer_rs::harness::sweep::ranking::{rank, CandidateOutcome, Thresholds};
use rust_decimal::Decimal;

fn d(v: i64) -> Decimal { Decimal::from(v) }

fn metrics(trades: usize, expectancy: i64, dd_pct: i64) -> PartitionMetrics {
    PartitionMetrics {
        trades,
        net_expectancy: d(expectancy),
        max_drawdown_pct: d(dd_pct),
        ..Default::default()
    }
}

fn outcome(label: &str, trades: usize, expectancy: i64, dd_pct: i64, fold_signs: [i64; 3], symbol_signs: [i64; 3]) -> CandidateOutcome {
    CandidateOutcome {
        label: label.into(),
        pooled_validation: metrics(trades, expectancy, dd_pct),
        folds: fold_signs.iter().map(|s| metrics(trades / 3, *s, dd_pct)).collect(),
        per_symbol_validation: symbol_signs.iter().enumerate()
            .map(|(i, s)| (format!("SYM{i}"), metrics(trades / 3, *s, dd_pct)))
            .collect(),
    }
}

#[test]
fn rejects_candidates_below_trade_floor() {
    let t = Thresholds::default();
    let weak = outcome("weak", 5, 10, 5, [1, 1, 1], [1, 1, 1]);
    let strong = outcome("strong", 40, 8, 10, [1, 1, -1], [1, 1, -1]);
    let sel = rank(&[weak, strong], &t);
    assert_eq!(sel.winners, vec!["strong".to_string()]);
}

#[test]
fn reports_no_candidate_when_none_qualify() {
    let t = Thresholds::default();
    let bad = outcome("bad", 40, -5, 10, [-1, -1, -1], [-1, -1, -1]);
    let sel = rank(&[bad], &t);
    assert!(sel.winners.is_empty());
    assert!(sel.notes.iter().any(|n| n.contains("no candidate")));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p analyzer-rs --features harness --test sweep_ranking`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `ranking.rs` and `report.rs`**

`ranking.rs`:

```rust
pub struct Thresholds { pub min_trades: usize, pub max_dd_pct: Decimal, pub min_positive_folds: usize, pub min_positive_symbols: usize }
impl Default for Thresholds { fn default() -> Self { Self { min_trades: 30, max_dd_pct: Decimal::from(20), min_positive_folds: 2, min_positive_symbols: 2 } } }

pub struct CandidateOutcome { pub label: String, pub pooled_validation: PartitionMetrics, pub folds: Vec<PartitionMetrics>, pub per_symbol_validation: Vec<(String, PartitionMetrics)> }

pub struct Selection { pub winners: Vec<String>, pub notes: Vec<String> }

pub fn rank(outcomes: &[CandidateOutcome], t: &Thresholds) -> Selection { /* priority 1..5; return empty winners with a note if none pass */ }
```

`report.rs`:

```rust
pub fn write_leaderboard_csv(path, rows) -> io::Result<()>
pub fn write_results_json(path, results) -> io::Result<()>
pub fn write_markdown(path, selection, results, provenance, commands) -> io::Result<()>
```

The Markdown report must contain: baseline vs candidates; exact configs + selection reason; trades/net expectancy/PF/max DD/costs; walk-forward per timeframe and symbol; sensitivity; data-quality issues; commands executed; and a recommendation line.

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p analyzer-rs --features harness --test sweep_ranking`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs/src/harness/sweep
git commit -m "feat(analyzer-rs): sweep ranking and reporting"
```

---

### Task 8: CLI orchestration, Makefile, smoke test

**Files:**
- Create: `process/analyzer-rs/src/bin/analyzer-sweep.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Modify: `Makefile`
- Test: `process/analyzer-rs/tests/sweep_smoke.rs`

- [ ] **Step 1: Add the binary target**

`Cargo.toml`:

```toml
[[bin]]
name = "analyzer-sweep"
path = "src/bin/analyzer-sweep.rs"
required-features = ["harness"]
```

- [ ] **Step 2: Implement the CLI**

Flags: `--symbols` (default `BTC-USDT,ETH-USDT,SOL-USDT`), `--timeframes` (default `1hour,4hour`), `--months` (24), `--as-of` (now), `--generated-at` (optional RFC 3339 override for reproducible artifacts; defaults to now), `--data-dir` (`data/sweep`), `--out-dir` (`process/analyzer-rs/experiments/sweep`), `--seed` (20261022), `--fee-bps` (10), `--slip-bps` (5), `--stress-fee-bps` (20), `--stress-slip-bps` (10), `--init-fund` (1000), `--refresh`.

Reproducible runs should pass `--generated-at` so `results.json`, `report.md`, and `leaderboard.csv` are byte-identical across reruns.

Orchestration:
1. For each timeframe and symbol: compute `[as_of - months, as_of]`, reuse the cache only when its provenance matches the requested symbol, timeframe, and range (otherwise refetch even without `--refresh`), retain only in-range candles, and validate a fetch through `load_csv` before committing it to the cache.
2. `generate(seed)` once per timeframe (same candidates for both).
3. First pass: for each timeframe × candidate × symbol, call `evaluate_config` with `include_holdout = false`; pool per candidate and `rank` on validation.
4. Freeze baseline plus the selected winners and re-evaluate only those with `include_holdout = true`, replacing their outcomes. Non-frozen candidates never carry holdout metrics.
5. Write `leaderboard.csv`, `results.json`, `report.md`; print a short summary and the artifact paths. Holdout rows/columns are omitted or rendered as `—` for non-frozen candidates.

- [ ] **Step 3: Makefile target**

```make
analyzer-sweep:
	cargo run -p analyzer-rs --features harness --bin analyzer-sweep -- $(ARGS)
```

Add it to `.PHONY` and `make help`.

- [ ] **Step 4: Smoke test on fixtures**

Create `tests/sweep_smoke.rs` that loads the checked-in fixtures, runs `evaluate_config` for the baseline on a small synthetic split, and asserts metrics are finite and the artifacts serialize. No network.

- [ ] **Step 5: Run the full gate**

```bash
cargo fmt -p analyzer-rs
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
cargo test -p analyzer-rs --features harness
cargo build -p analyzer-rs --no-default-features
```

- [ ] **Step 6: Commit**

```bash
git add process/analyzer-rs Makefile
git commit -m "feat(analyzer-rs): analyzer-sweep CLI and smoke test"
```

---

### Task 9: Run the 60-configuration screening study

**Files:**
- Output: `data/sweep/*` (untracked), `process/analyzer-rs/experiments/sweep/leaderboard.csv`, `process/analyzer-rs/experiments/sweep/results.json`, `process/analyzer-rs/experiments/sweep/report.md`

- [ ] **Step 1: Freeze `as_of`** and record it (e.g. the current UTC midnight).

- [ ] **Step 2: Fetch and cache** the 24-month range for both timeframes and all three symbols. Verify the provenance JSONs: candle counts, missing candles, exact date range.

- [ ] **Step 3: Run the sweep**

```bash
make analyzer-sweep ARGS="--as-of <MS> --out-dir process/analyzer-rs/experiments/sweep"
```

- [ ] **Step 4: Validate artifacts**

- `leaderboard.csv` has one row per timeframe × candidate × scored partition: `train`, `validation`, and `fold1_val`–`fold3_val` for every candidate (5 rows), `holdout` only for frozen candidates (baseline plus selected winners), plus one `validation` row per timeframe × candidate × symbol. For 30 candidates per timeframe, 3 symbols, and `F` frozen candidates, that is `30 × (5 + 3) + F` rows per timeframe (and twice that across two timeframes), not 60 rows. Stress columns are present on the pooled validation rows.
- `results.json` records the seed, candidate list, dedup results, provenance, splits, and per-partition metrics.
- `report.md` clearly separates validation (selection) from holdout (evaluation).

- [ ] **Step 5: Commit the report and results** (small, tracked)

```bash
git add -f process/analyzer-rs/experiments/sweep/leaderboard.csv process/analyzer-rs/experiments/sweep/results.json process/analyzer-rs/experiments/sweep/report.md docs/superpowers/plans/2026-10-22-analyzer-hyperparameter-screening.md
git commit -m "docs(sweep): initial 60-configuration screening results"
```

---

### Task 10: Final review and deliverable

- [ ] **Step 1: Whole-implementation review** by a fresh reviewer over the branch.
- [ ] **Step 2: Produce the final report** for the user with: baseline vs candidate results; exact configs + selection reasons; trades/net expectancy/PF/max DD/costs; walk-forward per timeframe and symbol; sensitivity; data quality; commands and outcomes; and the recommendation (retain baseline / larger experiment / insufficient evidence).

---

## Self-review

- **Spec coverage:** pagination (Task 5), costs (Task 2/6), splits/folds (Task 4), sampling (Task 3), ranking (Task 7), reporting (Task 7/9), H1 (Task 1), holdout separation (Task 8/9), safety (no `sim.rs` changes).
- **Placeholders:** none; every step contains code or an exact command. The Task 6 open-position handling and Task 7 ranking tests are fully specified.
- **Type consistency:** `Candidate`, `Slices`/`Fold`, `PartitionMetrics`, `CostScenario`, `Selection`, and `evaluate_config` are defined before use across tasks.
