# Analyzer Fresh Confirmation — Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development.

**Goal:** Implement a one-shot confirmation runner that verifies the freeze, fetches a fresh disjoint 24-month 4hour dataset, runs the frozen `max_age=4` + `retest_2c_b2`, and reports pass/fail against the pre-registered rule.

**Spec:** `docs/superpowers/specs/2026-10-22-analyzer-fresh-confirmation-design.md`
**Lock:** `freeze/confirmation-lock.json`

---

### Task 1: Freeze loader and hash verification

**Files:**
- Create `process/analyzer-rs/src/harness/sweep/freeze.rs`
- Modify `process/analyzer-rs/src/harness/sweep/mod.rs` (`pub mod freeze;`)
- Test `process/analyzer-rs/tests/confirmation.rs`

- [ ] **Step 1: Lock struct**

```rust
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct FrozenConfig {
    pub rsi_period: usize, pub rsi_order: usize, pub max_age: Option<usize>,
    pub sma_enabled: bool, pub sma_period: usize, pub atr_period: usize,
    pub atr_sl_multiplier: String, pub atr_tp_multiplier: String,
    pub price_diff_threshold: String, pub atr_pct_high: String,
    pub atr_high_adjust: String, pub atr_sl_max_pct: String, pub window: usize,
}
#[derive(Debug, Clone, Deserialize)]
pub struct FrozenModel { pub name: String, pub entry: String, pub offset_bps: String,
    pub valid_candles: usize, pub buffer_bps: String, pub maker_bps: String,
    pub taker_bps: String, pub slip_bps: String }
#[derive(Debug, Clone, Deserialize)]
pub struct FrozenDataset { pub symbols: Vec<String>, pub timeframe: String,
    pub as_of_ms: i64, pub months: u32, pub start_ms: i64 }
#[derive(Debug, Clone, Deserialize)]
pub struct DecisionRule { pub min_pooled_trades: usize,
    pub require_positive_net_expectancy: bool, pub require_profit_factor_above: f64,
    pub min_positive_symbols: usize }
#[derive(Debug, Clone, Deserialize)]
pub struct FrozenLock { pub created_at: String, pub git_commit: String,
    pub signal_generator_sha256: String, pub frozen_files: Vec<String>,
    pub config: FrozenConfig, pub model: FrozenModel, pub dataset: FrozenDataset,
    pub decision_rule: DecisionRule }

pub fn load(path: &Path) -> Result<FrozenLock, String> { /* serde_json::from_reader */ }

/// Sorted path + NUL + content + NUL, SHA-256.
pub fn compute_hash(repo_root: &Path, files: &[String]) -> Result<String, String> {
    let mut paths: Vec<PathBuf> = files.iter().map(|f| repo_root.join(f)).collect();
    paths.sort();
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.strip_prefix(repo_root).unwrap_or(&path).to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher.update(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?);
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify(repo_root: &Path, lock: &FrozenLock) -> Result<(), String> {
    let actual = compute_hash(repo_root, &lock.frozen_files)?;
    if actual != lock.signal_generator_sha256 {
        return Err(format!("frozen-file hash mismatch: expected {}, got {}", lock.signal_generator_sha256, actual));
    }
    Ok(())
}
```

- [ ] **Step 2: Tests**
- `lock_loads_and_hash_matches` (repo root = `env!("CARGO_MANIFEST_DIR")/../..`).
- `tampered_file_hash_mismatch` (copy one file to a temp dir and alter it, or pass a wrong expected hash).
- `locked_config_values` (baseline + max_age=4 + retest model).

### Task 2: Full-period evaluation + confirmation bin

**Files:**
- Create `process/analyzer-rs/src/harness/sweep/confirmation.rs` (full-period evaluation; do **not** modify the frozen `execution.rs`)
- Modify `process/analyzer-rs/src/harness/sweep/mod.rs` (`pub mod confirmation;`)
- Create `process/analyzer-rs/src/bin/analyzer-confirm.rs`
- Modify `process/analyzer-rs/Cargo.toml`
- Test `process/analyzer-rs/tests/confirmation.rs`

- [ ] **Step 1: `evaluate_full_period` in `confirmation.rs`**

Import `crate::harness::sweep::execution::{simulate_execution, ExecModel, ExecPartition}`. Compute fill rates locally (the helper may be private):

```rust
pub fn evaluate_full_period(
    dataset: &Dataset, cfg: &Config, max_age: Option<usize>,
    window: usize, init_fund: i64, model: &ExecModel,
) -> Result<ExecPartition, String> {
    let slice = slice_with_warmup(&dataset.candles, Slice { start: 0, end: dataset.candles.len() }, window);
    let ds = Dataset { symbol: dataset.symbol.clone(), candles: slice.candles, provenance: dataset.provenance.clone() };
    let replay = build_replay(&ds, cfg, max_age).map_err(|e| e.to_string())?;
    let eval_start = window.max(slice.partition_start);
    let signals: Vec<ReplaySignal> = replay.signals.into_iter().filter(|s| s.candle_index >= eval_start).collect();
    let outcome = simulate_execution(&ds.candles, &signals, init_fund, window, model);
    let last_close = ds.candles.last().ok_or("empty dataset")?.close;
    let last_index = ds.candles.len() - 1;
    let replayed = ds.candles.len().saturating_sub(window);
    let sim_outcome = outcome.to_sim_outcome();
    let metrics = metrics_from_outcome(&sim_outcome, last_close, last_index, replayed, Decimal::from(init_fund), &zero_cost());
    let (execution_fill_rate, capital_fill_rate) = fill_rates(outcome.filled, outcome.signals_total, outcome.skipped_funds, outcome.invalid_levels);
    Ok(ExecPartition { metrics, signals: outcome.signals_total, filled: outcome.filled, expired: outcome.expired, skipped_funds: outcome.skipped_funds, invalid_levels: outcome.invalid_levels, execution_fill_rate, capital_fill_rate })
}
```
Use the existing `fill_rates` helper (make it `pub(crate)` if private).

- [ ] **Step 2: Bin**

Flags:
`--lock` (default `freeze/confirmation-lock.json`), `--data-dir` (default `data/confirm`), `--out-dir` (default `out/confirm`), `--refresh`, `--base-url` (default env), `--settle-ms` (5000), `--init-fund` (1000), `--generated-at` (optional).

Flow:
1. Load lock; `freeze::verify(repo_root, &lock)`; abort on mismatch.
2. Build `Config` from `lock.config`; build `ExecModel` from `lock.model` (assert `entry == "limit"`, `name == "retest_2c_b2"`).
3. For each symbol in `lock.dataset.symbols`:
   - cache path `data_dir/<symbol>_4hour.csv`.
   - `load_with_cache(..., refresh, settle, window, || fetch_range(base_url, symbol, tf, start, end, settle, now))`.
   - `Dataset { symbol, candles, provenance }`.
   - `evaluate_full_period`.
4. Pool metrics; decision:
   - trades >= min_pooled_trades
   - net_expectancy > 0
   - PF > 1
   - positive symbols >= min_positive_symbols
5. Write `confirmation.csv`, `results.json`, `report.md`:
   - metadata (lock commit, hash, dataset range, config, model)
   - per-symbol and pooled metrics
   - decision and reasons
   - data quality (candles, missing, duplicates, hash)
6. Print `confirmation passed` or `confirmation failed`.
7. If the fetch returns fewer than `window + 2` candles, error.

- [ ] **Step 3: Cargo target**

```toml
[[bin]]
name = "analyzer-confirm"
path = "src/bin/analyzer-confirm.rs"
required-features = ["harness"]
```

- [ ] **Step 4: Tests**
- `evaluate_full_period` on `tests/fixtures/synthetic_1day.csv` with D1: metrics finite, trade counts, fill rates.
- `decision_fails_on_negative_expectancy` / `passes_on_positive` with synthetic `ExecPartition`.
- `lock_hash_verification` (Task 1).

### Task 3: Run the confirmation

```bash
cargo run -q -p analyzer-rs --features harness --bin analyzer-confirm -- \
  --lock freeze/confirmation-lock.json \
  --data-dir data/confirm \
  --out-dir out/confirm \
  --refresh \
  --generated-at 2026-10-22T00:00:00Z
```

- Verify the fetched fresh period: effective range ≈ 2022-11-02 → 2024-10-22, 4h, ~4,300 candles per symbol, missing/duplicates reported.
- Run once; no tuning.
- Commit source + artifacts.

## Acceptance

- `cargo fmt -p analyzer-rs`
- `cargo test -p analyzer-rs --features harness`
- `cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings`
- `cargo build -p analyzer-rs --no-default-features`
- The runner aborts if the lock hash or config/model does not match.
- Report `confirmation passed` or `confirmation failed` with the pre-registered reasons.
