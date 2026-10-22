# Analyzer Execution Modelling — Implementation Plan (Experiment 4, focused)

> **For agentic workers:** Use superpowers:subagent-driven-development.

**Goal:** Compare market-at-next-open vs limit/maker entries on the focused 4hour set, with realistic fill rules, and decide whether execution changes can rescue the thin holdout edge.

**Architecture:** New `harness/sweep/execution.rs` with a standalone execution simulator (pending limit orders, fill rules, maker/taker costs, EOD close), a thin evaluation wrapper over the existing replay/splits, and a new `analyzer-execution` bin. No changes to `sim.rs`.

**Spec:** `docs/superpowers/specs/2026-10-22-analyzer-execution-modelling-design.md`

**Focused set:** models `market`, `limit_1c_b2`, `limit_2c_b2`, `retest_2c_b2`; candidates baseline, `joint-13`, `max_age=4`; partitions validation + holdout only.

---

### Task 1: Execution simulator + tests

**Files:**
- Create `process/analyzer-rs/src/harness/sweep/execution.rs`
- Modify `process/analyzer-rs/src/harness/sweep/mod.rs` (`pub mod execution;`)
- Test `process/analyzer-rs/tests/execution.rs`

- [ ] **Step 1: Types**

```rust
use crate::harness::sim::{
    unrealized_pnl, ClosedTrade, SimOutcome, REASON_GAP_SL, REASON_GAP_TP, REASON_SL, REASON_TP,
};
use crate::harness::replay::ReplaySignal;
use crate::harness::sweep::evaluate::{metrics_from_outcome, PartitionMetrics};
use crate::harness::sweep::costs::CostScenario;
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum EntryMode {
    Market,
    Limit { valid_candles: usize, buffer_bps: Decimal, offset_bps: Decimal },
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecModel {
    pub label: String,
    pub entry: EntryMode,
    pub maker_bps: Decimal,
    pub taker_bps: Decimal,
    pub slip_bps: Decimal,
}

impl ExecModel {
    pub fn focused() -> Vec<Self> {
        let m = |label: &str, entry: EntryMode| ExecModel {
            label: label.into(), entry,
            maker_bps: Decimal::from(2), taker_bps: Decimal::from(10), slip_bps: Decimal::from(5),
        };
        vec![
            m("market", EntryMode::Market),
            m("limit_1c_b2", EntryMode::Limit { valid_candles: 1, buffer_bps: Decimal::from(2), offset_bps: Decimal::ZERO }),
            m("limit_2c_b2", EntryMode::Limit { valid_candles: 2, buffer_bps: Decimal::from(2), offset_bps: Decimal::ZERO }),
            m("retest_2c_b2", EntryMode::Limit { valid_candles: 2, buffer_bps: Decimal::from(2), offset_bps: Decimal::from(25) }),
        ]
    }
    pub fn entry_rate(&self) -> Decimal {
        match self.entry {
            EntryMode::Market => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
            EntryMode::Limit { .. } => self.maker_bps / Decimal::from(10_000),
        }
    }
    pub fn exit_rate(&self, reason: &str) -> Decimal {
        match self.entry {
            EntryMode::Market => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
            EntryMode::Limit { .. } if reason == REASON_TP => self.maker_bps / Decimal::from(10_000),
            EntryMode::Limit { .. } => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecTrade {
    pub signal_id: String,
    pub side: Direction,
    pub signal_entry: Decimal,
    pub fill: Decimal,
    pub exit: Decimal,
    pub e: Decimal,
    pub gross_pnl: Decimal,
    pub entry_cost: Decimal,
    pub exit_cost: Decimal,
    pub net_pnl: Decimal,
    pub reason: String,
    pub entry_index: usize,
    pub exit_index: usize,
    pub risk_reward: Option<Decimal>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExecOutcome {
    pub trades: Vec<ExecTrade>,
    pub available_fund: Decimal,
    pub signals_total: usize,
    pub filled: usize,
    pub expired: usize,
    pub skipped_funds: usize,
    pub invalid_levels: usize,
}
```

- [ ] **Step 2: Simulation loop**

Implement `simulate_execution(candles: &[Candle], signals: &[ReplaySignal], init_fund: i64, window: usize, model: &ExecModel) -> ExecOutcome`.

Internal structs:
```rust
struct Pending {
    signal_id: String, side: Direction, signal_entry: Decimal,
    sl: Decimal, tp: Decimal, signal_index: usize,
}
struct Open {
    signal_id: String, side: Direction, signal_entry: Decimal,
    fill: Decimal, e: Decimal, sl: Decimal, tp: Decimal,
    entry_index: usize, entry_cost: Decimal, allow_same_candle_exit: bool,
}
```

Rules:
- `budget = init_fund / 2`; `available = init_fund`.
- For each candle `index` in `window..len`:
  1. Process pending orders (oldest first). Market fills only at `signal_index + 1` at `open`, otherwise expires. Limit fills over `signal_index+1..=signal_index+valid_candles` using `limit_fill`.
  2. Open a position if geometry is valid and `available >= budget`; reserve `budget`; `entry_cost = model.entry_rate() * budget`; `allow_same_candle_exit = matches!(entry, Market)`.
  3. Process exits for existing positions, skipping same-candle exits for limit entries. Use gap SL → gap TP → SL → TP (SL-first).
  4. Queue signals at this candle. If `index + 1 >= len`, count as expired immediately.
- After the loop: close all open positions at `candles.last().close` with reason `"eod"`, taker exit; add remaining pending to `expired`.
- PnL: `gross = E * exit / fill - E` for long, `E - E * exit / fill` for short, cap at `-E`. `exit_notional = E * exit / fill`; `exit_cost = model.exit_rate(reason) * exit_notional`; `net = gross - entry_cost - exit_cost`. `available += E + gross`.
- `signals_total = signals.len()`, `filled = trades.len()`, counts add up; `fill_rate` computed by the caller.

`limit_fill`:
```rust
fn limit_fill(side: Direction, entry: Decimal, offset_bps: Decimal, buffer_bps: Decimal, c: &Candle) -> Option<Decimal> {
    let off = offset_bps / Decimal::from(10_000);
    let buf = buffer_bps / Decimal::from(10_000);
    match side {
        Direction::Long => {
            let limit = entry * (Decimal::ONE - off);
            let threshold = limit * (Decimal::ONE - buf);
            if c.open <= threshold { Some(c.open) }
            else if c.low <= threshold { Some(limit) }
            else { None }
        }
        Direction::Short => {
            let limit = entry * (Decimal::ONE + off);
            let threshold = limit * (Decimal::ONE + buf);
            if c.open >= threshold { Some(c.open) }
            else if c.high >= threshold { Some(limit) }
            else { None }
        }
    }
}
```

`exit_for` and `risk_reward_for`: copy the existing `sim.rs` logic (SL-first, gap checks).

- [ ] **Step 3: Convert to `SimOutcome`**

```rust
impl ExecOutcome {
    pub fn to_sim_outcome(&self) -> SimOutcome {
        SimOutcome {
            trades: self.trades.iter().map(|t| ClosedTrade {
                side: t.side, signal_entry: t.signal_entry, fill: t.fill, exit: t.exit,
                e: t.e, pnl: t.net_pnl,
                pnl_pct: if t.e.is_zero() { Decimal::ZERO } else { t.net_pnl / t.e * Decimal::from(100) },
                risk_reward: t.risk_reward, reason: t.reason.clone(),
                signal_id: t.signal_id.clone(),
                entry_index: t.entry_index, exit_index: t.exit_index,
            }).collect(),
            open_positions: Vec::new(),
            available_fund: self.available_fund,
            skipped_funds: self.skipped_funds,
            unfilled_at_end: self.expired,
            invalid_levels: self.invalid_levels,
        }
    }
}
```

Metrics: call `metrics_from_outcome(&sim_outcome, last_close, last_index, replayed, init_fund, &CostScenario { label: "net".into(), fee_bps: Decimal::ZERO, slip_bps: Decimal::ZERO })`. The resulting metrics are net (because `pnl` is already net and the scenario adds zero).

- [ ] **Step 4: Tests**

`tests/execution.rs`:
- `market_model_matches_existing_simulator_gross`: synthetic candles/signals where all trades close before the end; run `sim::simulate` and `simulate_execution` with a zero-cost market model; assert same trades, fills, exits, gross PnL.
- `long_limit_fill_open_through_touch_and_expiry`: three cases.
- `short_limit_fill_symmetric`.
- `buffer_requires_trade_through`.
- `limit_entry_does_not_exit_on_the_same_candle`.
- `market_cost_is_15_bps_per_side_and_limit_entry_is_maker`: a single trade where the expected net PnL is computed by hand.
- `eod_closes_open_positions_with_taker_cost`.
- `fill_counts_add_up`: filled + expired + skipped_funds + invalid_levels == signals_total.

### Task 2: Evaluation wrapper + pooling

**Files:** `execution.rs`, tests.

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ExecPartition {
    pub metrics: PartitionMetrics,
    pub signals: usize,
    pub filled: usize,
    pub expired: usize,
    pub skipped_funds: usize,
    pub invalid_levels: usize,
    pub fill_rate: Decimal,
}

pub fn evaluate_execution_config(
    dataset: &Dataset, cfg: &Config, max_age: Option<usize>, splits: &Splits,
    window: usize, init_fund: i64, models: &[ExecModel],
) -> Result<BTreeMap<String, BTreeMap<String, ExecPartition>>, String>
```
- For `validation` and `holdout` only:
  - `slice = slice_with_warmup(&dataset.candles, split, window)`.
  - `Dataset { symbol, candles: slice.candles, provenance }`.
  - `build_replay`, filter signals to `candle_index >= window.max(slice.partition_start)`.
  - For each model: simulate, convert, metrics, fill stats.
- `pool_exec(parts: &[ExecPartition]) -> ExecPartition` using `evaluate::pool` and summed counts; `fill_rate = filled/signals`.

Tests:
- `evaluate_execution_config` on the synthetic fixture: 4 models × 2 partitions; metrics finite; fill_rate in [0,1].

### Task 3: Bin, report, run

**Files:** `process/analyzer-rs/src/bin/analyzer-execution.rs`, `Cargo.toml`, `out/execution/*`.

Flags: `--symbols` (BTC/ETH/SOL), `--data-dir` (default `data/sweep`), `--out-dir` (default `out/execution`), `--as-of` (1792661878840), `--settle-ms` (5000), `--init-fund` (1000), `--generated-at` (2026-10-22T00:00:00Z).

- Load 4h cache per symbol via `load_csv`.
- `focused_candidates()` = `cost_ladder::frozen_candidates()` filtered to `baseline`, `joint-13`, `max_age=4`.
- For each candidate × symbol: `evaluate_execution_config` with `ExecModel::focused()`.
- Pool across symbols per `(candidate, model, partition)`.
- Write:
  - `execution.csv`: `candidate,model,symbol_or_pooled,partition,signals,filled,fill_rate,expired,skipped_funds,invalid_levels,trades,net_expectancy,net_profit_factor,win_rate,max_drawdown_pct,expectancy_stderr,net_pnl`.
  - `results.json`: metadata, model definitions, rows, decision.
  - `report.md`: metadata, model definitions, validation table, holdout table, per-symbol SOL/ETH tables, decision, limitations.
- Decision:
  - For each config, find the best limit model on validation with fill_rate ≥ 50%, net_expectancy > 0, and > market.
  - It rescues only if holdout net_expectancy > 0, PF > 1, and ≥ 2/3 symbols positive.
  - Otherwise `execution does not rescue`.
  - Include the multiple-use holdout caveat.

Cargo:
```toml
[[bin]]
name = "analyzer-execution"
path = "src/bin/analyzer-execution.rs"
required-features = ["harness"]
```

Run:
```bash
cargo run -q -p analyzer-rs --features harness --bin analyzer-execution -- \
  --data-dir /home/blackh-t/Documents/git/CoinBot.v3/data/sweep \
  --out-dir out/execution \
  --as-of 1792661878840 \
  --generated-at 2026-10-22T00:00:00Z
```

Gate:
```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
cargo build -p analyzer-rs --no-default-features
```

Commit source + artifacts.

## Self-review

- Focused set: 4 models, 3 candidates, validation + holdout only.
- No `sim.rs` or production-default changes.
- Costs applied once; net PnL converted to `SimOutcome` with a zero-cost metrics scenario.
- Fill rates and expired orders reported; low-fill high-expectancy flagged.
