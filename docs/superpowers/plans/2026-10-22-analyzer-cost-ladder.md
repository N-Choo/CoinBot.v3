# Analyzer Cost Ladder — Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build and run a cost-ladder analysis for the frozen 4hour baseline and four winners, using the existing 24-month cache, and report the break-even per-side cost.

**Architecture:** A new `analyzer-cost-ladder` bin reuses `dataset::load_csv`, `partition::chronological_splits`, `evaluate::evaluate_config`/`pool`, and `costs::CostScenario`. It evaluates each config at 7 per-side cost levels on validation + holdout, pools across symbols, and writes CSV/JSON/Markdown.

**Spec:** `docs/superpowers/specs/2026-10-22-analyzer-profitability-research-design.md` (Experiment 1)

---

### Task 1: Cost-ladder module, binary, and run

**Files:**
- Create: `process/analyzer-rs/src/harness/sweep/cost_ladder.rs`
- Create: `process/analyzer-rs/src/bin/analyzer-cost-ladder.rs`
- Modify: `process/analyzer-rs/src/harness/sweep/mod.rs` (`pub mod cost_ladder;`)
- Modify: `process/analyzer-rs/Cargo.toml` (`[[bin]] analyzer-cost-ladder`, required-features `harness`)
- Test: `process/analyzer-rs/tests/cost_ladder.rs`

- [ ] **Step 1: Define the frozen candidates**

In `cost_ladder.rs`:

```rust
use crate::harness::sweep::sampling::{baseline, Candidate};
use rust_decimal::Decimal;

pub fn frozen_candidates() -> Vec<Candidate> {
    let mut out = vec![baseline()];
    let mut push = |label: &str, rsi: usize, order: usize, age: Option<usize>, sl: Decimal, tp: Decimal| {
        out.push(Candidate {
            label: label.into(),
            origin: "screening-4hour".into(),
            rsi_period: rsi,
            rsi_order: order,
            max_age: age,
            sma_enabled: true,
            sma_period: 20,
            atr_sl_multiplier: sl,
            atr_tp_multiplier: tp,
        });
    };
    push("joint-13", 10, 5, None, Decimal::new(25, 1), Decimal::new(40, 1));
    push("joint-24", 21, 5, Some(12), Decimal::new(20, 1), Decimal::new(40, 1));
    push("max_age=4", 14, 5, Some(4), Decimal::new(15, 1), Decimal::new(20, 1));
    push("atr_tp=4.0", 14, 5, Some(12), Decimal::new(15, 1), Decimal::new(40, 1));
    out
}
```

- [ ] **Step 2: Cost scenarios and break-even**

```rust
pub const LEVELS_BPS: [u32; 7] = [0, 25, 50, 75, 100, 150, 300]; // tenths of a bp

pub fn scenarios() -> Vec<CostScenario> {
    LEVELS_BPS.iter().map(|tenths| CostScenario {
        label: format!("{}.{}bps", tenths / 10, tenths % 10),
        fee_bps: Decimal::new(*tenths as i64, 1),
        slip_bps: Decimal::ZERO,
    }).collect()
}

/// Interpolate the per-side cost where pooled net expectancy crosses zero.
/// None = still positive at the highest level; Some(0) = not positive at 0.
pub fn break_even_bps(points: &[(Decimal, Decimal)]) -> Option<Decimal> {
    if points.is_empty() { return None; }
    if points[0].1 <= Decimal::ZERO { return Some(Decimal::ZERO); }
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if y0 > Decimal::ZERO && y1 <= Decimal::ZERO {
            return Some(x0 + (x1 - x0) * y0 / (y0 - y1));
        }
    }
    None
}
```

- [ ] **Step 3: Tests**

`tests/cost_ladder.rs`:
- `frozen_candidates` has 5 entries, baseline first, expected labels.
- `scenarios` has 7 entries with labels `0.0bps`, `2.5bps`, `5.0bps`, `7.5bps`, `10.0bps`, `15.0bps`, `30.0bps`.
- `break_even_bps`:
  - all negative at 0 → `Some(0)`.
  - crossing between 5 and 10 with y0=2, y1=-2 → `7.5`.
  - all positive → `None`.

- [ ] **Step 4: Binary**

`analyzer-cost-ladder.rs`:
- Flags: `--symbols` (default `BTC-USDT,ETH-USDT,SOL-USDT`), `--data-dir` (default `data/sweep`), `--out-dir` (default `out/cost-ladder`), `--as-of` (default `1792661878840`), `--settle-ms` (default 5000), `--init-fund` (default 1000), `--generated-at` (default `2026-10-22T00:00:00Z`).
- For each symbol: path `<data-dir>/<symbol>_4hour.csv`; `load_csv(symbol, path, Timeframe::H4, as_of, settle, 0, 100, 0)`.
- `splits = chronological_splits(dataset.candles.len())`.
- For each frozen candidate: `evaluate_config(&dataset, &candidate.to_config(), candidate.max_age, &splits, 100, init_fund, &scenarios, true)` (use the actual current signature from `evaluate.rs`; the last bool is `include_holdout`).
- Pool across symbols per `(label, scenario, partition)` with `evaluate::pool`.
- Compute per-symbol rows too.
- Write:
  - `cost_ladder.csv`: `label,cost_bps_per_side,partition,symbol_or_pooled,trades,net_expectancy,net_expectancy_r,net_profit_factor,win_rate,max_drawdown_pct,expectancy_stderr,closed_net_pnl,open_net_pnl,net_pnl`
  - `results.json`: metadata + all rows + break-even per config/partition.
  - `report.md`: validation and holdout tables, break-even table, per-symbol table at 0/5/10/15 bps, and an interpretation paragraph:
    - if baseline/winners 0 bps ≤ 0 → "no gross edge";
    - else if break-even < 10 bps/side → "gross edge exists but likely below realistic execution costs";
    - else → "gross edge may survive realistic costs; proceed to excursion/execution tests".
- Print the output paths and a short summary.

- [ ] **Step 5: Cargo target and gate**

```toml
[[bin]]
name = "analyzer-cost-ladder"
path = "src/bin/analyzer-cost-ladder.rs"
required-features = ["harness"]
```

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
cargo build -p analyzer-rs --no-default-features
```

- [ ] **Step 6: Run on the cached 4hour data**

From the worktree (cache is in the main checkout):

```bash
cargo run -q -p analyzer-rs --features harness --bin analyzer-cost-ladder -- \
  --data-dir /home/blackh-t/Documents/git/CoinBot.v3/data/sweep \
  --out-dir out/cost-ladder \
  --as-of 1792661878840 \
  --generated-at 2026-10-22T00:00:00Z
```

Verify `out/cost-ladder/report.md` and `results.json`. Commit the artifacts.

- [ ] **Step 7: Report**

Return the break-even per config/partition, the 0 bps gross expectancy, the fraction of symbols/folds positive at 5/10/15 bps, and the interpretation.

---

## Self-review

- Spec Experiment 1 coverage: configs, symbols, data, splits, cost levels, partitions, metrics, break-even, decision rule, artifacts.
- No production defaults or `sim.rs` changes.
- Outputs are additive under `out/cost-ladder/`.
