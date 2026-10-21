# Analyzer-RS Replay Report Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `analyzer-replay` report show TP/SL and planned R:R explicitly, precompute per-step account state, and keep KuCoin fetch as the default data source.

**Architecture:** Extend the simulator to carry `signal_id` and emit a per-step `account[]` series; extend the payload additively; upgrade the self-contained Terminal template to read that data without recomputing account math or inferring exits.

**Tech Stack:** Rust 2021 (`rust_decimal`, `serde_json`), the existing harness/replay/sim/report modules, and the self-contained `templates/replay.html`.

**Scope:** Presentation + read-only data additions only. No strategy/driver changes, no schema break.

**Reference spec:** `docs/superpowers/specs/2026-10-21-analyzer-replay-report-polish-design.md`

---

## File map

| File | Responsibility |
|---|---|
| `src/harness/sim.rs` | `SimSignal.signal_id`, `Position`/`ClosedTrade.signal_id`, `AccountStep`, per-step account series |
| `src/harness/stats.rs` | `tp_hits`, `sl_hits`, `other_exits`, `avg_risk_reward` |
| `src/harness/report.rs` | Payload additions: `signal_id`, `account[]`, summary fields |
| `templates/replay.html` | TP/SL ledger columns, open-position cards, chart labels, summary, polish |
| `tests/sim.rs` | signal_id, account series, exit classification, avg R:R |
| `tests/replay_report.rs` | payload additions and template assertions |
| `src/bin/analyzer-replay.rs` | pass `signal_id` into `SimSignal`; docs/example use live fetch |

---

### Task 1: Simulator carries signal_id and emits per-step account state

**Files:**
- Modify: `process/analyzer-rs/src/harness/sim.rs`
- Modify: `process/analyzer-rs/src/harness/stats.rs`
- Modify: `process/analyzer-rs/src/bin/analyzer-replay.rs`
- Test: `process/analyzer-rs/tests/sim.rs`

- [ ] **Step 1: Write the failing tests**

Add to `tests/sim.rs`:

```rust
#[test]
fn trades_and_positions_carry_signal_id() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 104, 105),
    ];
    let mut long = signal(0, Direction::Long, 100, 98, 104);
    long.signal_id = "sig-a".into();
    let mut open = signal(2, Direction::Long, 105, 95, 120);
    open.signal_id = "sig-b".into();
    let outcome = simulate(&candles, &[long, open], 1000, 0);
    assert_eq!(outcome.trades[0].signal_id, "sig-a");
    assert_eq!(outcome.open_positions[0].signal_id, "sig-b");
}

#[test]
fn account_series_is_consistent_at_every_step() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 104, 105),
    ];
    let mut first = signal(0, Direction::Long, 100, 98, 104);
    first.signal_id = "sig-a".into();
    let mut second = signal(1, Direction::Long, 105, 95, 120);
    second.signal_id = "sig-b".into();
    let outcome = simulate(&candles, &[first, second], 1000, 0);
    assert_eq!(outcome.account.len(), 3);
    for step in &outcome.account {
        assert_eq!(step.equity, step.available_fund + step.reserved + step.unrealized);
    }
    let last = outcome.account.last().unwrap();
    assert_eq!(last.open.len(), outcome.open_positions.len());
}

#[test]
fn exit_classification_and_avg_risk_reward() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105), // TP
        candle(2, 105, 106, 94, 95),   // next entry opens here
        candle(3, 95, 100, 90, 92),    // SL
    ];
    let mut a = signal(0, Direction::Long, 100, 98, 104);
    a.signal_id = "sig-a".into();
    let mut b = signal(2, Direction::Long, 95, 90, 120);
    b.signal_id = "sig-b".into();
    let outcome = simulate(&candles, &[a, b], 1000, 0);
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(92), 3, 4);
    assert_eq!(summary.tp_hits, 1);
    assert_eq!(summary.sl_hits, 1);
    assert_eq!(summary.other_exits, 0);
    assert!(summary.avg_risk_reward.is_some());
}
```

Update the test `signal(...)` helper in `tests/sim.rs` to set `signal_id: format!("sig-{index}-{entry}")`, then override it per test as shown. Adding the `signal_id` field to `Position`/`ClosedTrade` also breaks the struct literals in `src/harness/report.rs` `mod tests` and in `tests/replay_report.rs`'s `run` helper — add `signal_id` there too (the compiler will point at each).

- [ ] **Step 2: Run to verify failure**

```bash
cargo test -p analyzer-rs --features harness --test sim
```

Expected: compile errors (`signal_id`, `account`, `tp_hits`, `avg_risk_reward` missing).

- [ ] **Step 3: Extend the simulator**

`src/harness/sim.rs`:

```rust
#[derive(Debug, Clone)]
pub struct SimSignal {
    pub candle_index: usize,
    pub signal: Signal,
    pub signal_id: String,
}

#[derive(Debug, Clone)]
pub struct OpenStep {
    pub signal_id: String,
    pub unrealized: Decimal,
}

#[derive(Debug, Clone)]
pub struct AccountStep {
    pub index: usize,
    pub available_fund: Decimal,
    pub reserved: Decimal,
    pub unrealized: Decimal,
    pub equity: Decimal,
    pub open: Vec<OpenStep>,
}
```

Add `pub signal_id: String` to `Position` and `ClosedTrade`.

In `simulate`:
- when opening: set `signal_id: sim_signal.signal_id.clone()`;
- when closing: copy `position.signal_id` into the `ClosedTrade`;
- after step 2 of each candle, compute and push an `AccountStep`:

```rust
let close = candle.close;
let mut reserved = Decimal::ZERO;
let mut unrealized = Decimal::ZERO;
let mut open = Vec::with_capacity(positions.len());
for position in &positions {
    reserved += position.e;
    let ret = (close - position.fill) / position.fill;
    let ret = match position.side {
        Direction::Long => ret,
        Direction::Short => -ret,
    };
    let value = position.e * ret;
    unrealized += value;
    open.push(OpenStep {
        signal_id: position.signal_id.clone(),
        unrealized: value,
    });
}
account.push(AccountStep {
    index,
    available_fund: available,
    reserved,
    unrealized,
    equity: available + reserved + unrealized,
    open,
});
```

Add `pub account: Vec<AccountStep>` to `SimOutcome` and return it.

- [ ] **Step 4: Extend the summary**

`src/harness/stats.rs` `SimSummary` additions:

```rust
pub tp_hits: usize,
pub sl_hits: usize,
pub other_exits: usize,
pub avg_risk_reward: Option<Decimal>,
```

In `summarize`:

```rust
let tp_hits = trades
    .iter()
    .filter(|t| t.reason == "TP" || t.reason == "gap_tp")
    .count();
let sl_hits = trades
    .iter()
    .filter(|t| t.reason == "SL" || t.reason == "gap_sl")
    .count();
let other_exits = trades.len() - tp_hits - sl_hits;

let eligible: Vec<Decimal> = trades
    .iter()
    .filter_map(|t| t.risk_reward)
    .collect();
let avg_risk_reward = if eligible.is_empty() {
    None
} else {
    Some(eligible.iter().copied().sum::<Decimal>() / Decimal::from(eligible.len()))
};
```

Set these in the returned `SimSummary`.

- [ ] **Step 5: Update the CLI mapping**

`src/bin/analyzer-replay.rs`: when building `SimSignal`, include `signal_id: s.signal_id.clone()`.

- [ ] **Step 6: Run**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test sim
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): carry signal ids and per-step account state"
```

---

### Task 2: Payload additions

**Files:**
- Modify: `process/analyzer-rs/src/harness/report.rs`
- Test: `process/analyzer-rs/tests/replay_report.rs`

- [ ] **Step 1: Write the failing test**

Extend `tests/replay_report.rs`:

```rust
#[test]
fn payload_includes_signal_ids_account_and_exit_counts() {
    let cfg = Config::default();
    let run = run("SYNTH-USDT", "synthetic_1day.csv", &cfg);
    let payload = build_payload(&inputs(&[run]), &cfg, "2026-10-21T00:00:00Z".into(), vec![]);
    let symbol = &payload["symbols"][0];
    assert!(symbol["account"].as_array().unwrap().len() > 0);
    assert!(symbol["trades"].as_array().unwrap().iter().all(|t| t["signal_id"].is_string()));
    for step in symbol["account"].as_array().unwrap() {
        let available = step["available_fund"].as_str().unwrap().parse::<rust_decimal::Decimal>().unwrap();
        let reserved = step["reserved"].as_str().unwrap().parse::<rust_decimal::Decimal>().unwrap();
        let unrealized = step["unrealized"].as_str().unwrap().parse::<rust_decimal::Decimal>().unwrap();
        let equity = step["equity"].as_str().unwrap().parse::<rust_decimal::Decimal>().unwrap();
        assert_eq!(equity, available + reserved + unrealized);
    }
    assert!(symbol["summary"]["tp_hits"].is_number());
    assert!(symbol["summary"]["sl_hits"].is_number());
    assert!(symbol["summary"]["other_exits"].is_number());
    assert!(symbol["summary"]["avg_risk_reward"].is_string() || symbol["summary"]["avg_risk_reward"].is_null());
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo test -p analyzer-rs --features harness --test replay_report
```

- [ ] **Step 3: Implement the payload additions**

In `report.rs`:
- trades: add `"signal_id": t.signal_id`.
- open_positions: add `"signal_id": p.signal_id` and `"unrealized"` from the last account step matching that `signal_id`.
- add `"account"`: map each `AccountStep` to
  `{ i, available_fund, reserved, unrealized, equity, open: [{ signal_id, unrealized }] }` with
  Decimal values as strings.
- `summary_json`: add `tp_hits`, `sl_hits`, `other_exits`, and `avg_risk_reward` (`Some` → string,
  `None` → `null`).

- [ ] **Step 4: Run**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test replay_report
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): expose signal ids, account series, and exit counts"
```

---

### Task 3: Template — TP/SL, open positions, chart labels, summary

**Files:**
- Modify: `process/analyzer-rs/templates/replay.html`
- Test: `process/analyzer-rs/tests/replay_report.rs`

- [ ] **Step 1: Write the failing template assertions**

Add to `tests/replay_report.rs`:

```rust
#[test]
fn template_shows_levels_and_exit_counts() {
    let cfg = Config::default();
    let run = run("SYNTH-USDT", "synthetic_1day.csv", &cfg);
    let payload = build_payload(&inputs(&[run]), &cfg, "2026-10-21T00:00:00Z".into(), vec![]);
    let out = std::env::temp_dir().join("analyzer-rs-levels.html");
    write_report(&payload, &out).unwrap();
    let html = std::fs::read_to_string(&out).unwrap();
    for needle in ["SL", "TP", "R:R", "TP hits", "SL hits", "avg planned R:R", "beyond TP", "beyond SL", "planned TP/SL"] {
        assert!(html.contains(needle), "template must mention {needle}");
    }
    assert!(html.contains("signal_id"), "template must resolve signals by signal_id");
}
```

Note: the `run` helper builds `SimSignal` from `replay.signals`, so add
`signal_id: signal.signal_id.clone()` there; `synthetic_1day.csv` must exist under
`tests/fixtures` (use the existing fixture name used by the other tests in this file).

- [ ] **Step 2: Run to verify failure**

```bash
cargo test -p analyzer-rs --features harness --test replay_report
```

- [ ] **Step 3: Update the template**

`templates/replay.html` changes:

1. Build a signal lookup per tab: `const byId = new Map(tab.signals.map(s => [s.signal_id, s]))`.
2. Ledger header and rows: `Entry · Side · Signal Entry · Fill · SL · TP · R:R · Exit · Exit Price · PnL · Result`; SL/TP from `byId.get(trade.signal_id)`; R:R from the trade; open rows use the current `account` step's `open[].unrealized` and show `Open`/`—`.
3. Open-position cards: side, fill, SL, TP, `distancePct(price, level)` with `above`/`below`, a direction-aware bar, and unrealized from the current account step; render `beyond TP`/`beyond SL` when price is outside the range and `—` when a level is missing.
4. Chart: TP/SL dashed lines start at `entry_index` and end at `exit_index` (or the step); add right-edge labels `TP $…`/`SL $…` clamped inside the plot and staggered when close; keep the signal/fill/exit markers and update the legend to `▲ signal · ● fill · × exit · ┄ planned TP/SL (from the signal)`.
5. Summary: add `TP hits`, `SL hits`, `other exits`, `avg planned R:R`; read equity/unrealized from `account[step - window]` (no JS account math).
6. Polish: sticky header/tab bar, denser monospace table with horizontal scroll on narrow widths, responsive cards, TP teal / SL red.
7. Keep the banner and the read-only rule: no indicator, signal, exit, or account recomputation in JS.

- [ ] **Step 4: Run**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test replay_report
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): show TP/SL levels and account state in the report"
```

---

### Task 4: Data-source docs and live-fetch example

**Files:**
- Modify: `process/analyzer-rs/README.md`
- Modify: `Makefile` (help text only)
- Test: acceptance commands below

- [ ] **Step 1: Update the README and help**

- Make the primary example a **live fetch** (no `--data`):
  ```bash
  cargo run -p analyzer-rs --features harness --bin analyzer-replay -- \
    --symbol BTC-USDT --timeframe 15min --limit 500 \
    --out process/analyzer-rs/out/replay.html
  ```
- Document `--data SYMBOL=path.csv` as the deterministic offline option, with `--as-of`/`--generated-at`.
- Document partial-failure and all-failure exit behavior, and that live fetches are not byte-reproducible.
- Update `make help` wording for `analyzer-replay` to mention the default live fetch.

- [ ] **Step 2: Run acceptance**

```bash
make analyzer-rs-check
cargo run -p analyzer-rs --features harness --bin analyzer-replay -- \
  --symbol BTC-USDT --timeframe 15min --limit 500 \
  --out /tmp/replay-live.html
```

Expected: `/tmp/replay-live.html` exists when KuCoin is reachable; if not, the run exits non-zero
with a clear error and no empty report.

- [ ] **Step 3: Commit**

```bash
git add process/analyzer-rs Makefile
git commit -m "docs(analyzer-rs): default replay examples to live KuCoin fetch"
```

---

### Task 5: Final verification

- [ ] **Step 1: Run the full gate**

```bash
make analyzer-rs-check
cargo build -p analyzer-rs --no-default-features
```

- [ ] **Step 2: Render the report**

```bash
cargo run -p analyzer-rs --features harness --bin analyzer-replay -- \
  --timeframe 1day --as-of 1800000000000 \
  --data BTC-USDT=tests/fixtures/btc_usdt_1day.csv \
  --data XRP-USDT=tests/fixtures/xrp_usdt_1day.csv \
  --out /tmp/replay-final.html
google-chrome --headless=new --no-sandbox --screenshot=/tmp/replay-final.png --window-size=1440,1200 file:///tmp/replay-final.html
```

Expected: no JS errors; the ledger shows SL/TP/R:R, open positions show both levels, and the chart
labels TP/SL.

- [ ] **Step 3: Commit any formatting changes**

```bash
git add process/analyzer-rs
git commit -m "chore(analyzer-rs): finalize replay report polish"
```

---

## Self-review

- **Spec coverage:** signal attribution (Task 1), exit counts/avg R:R (Tasks 1–2), account series
  (Tasks 1–2), ledger/open-position/chart/summary/polish (Task 3), default live fetch docs and
  acceptance (Task 4), full verification (Task 5).
- **Placeholder scan:** no `TBD`/`TODO`; each code step contains code.
- **Type consistency:** `SimSignal.signal_id`, `Position`/`ClosedTrade.signal_id`, `AccountStep`,
  `SimSummary.tp_hits/sl_hits/other_exits/avg_risk_reward`, and payload keys are defined before use.
- **Read-only rule:** the template only formats and aggregates precomputed replay/account data.
