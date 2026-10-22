# Analyzer Hyperparameter Screening — Design

- **Status:** Approved with adjustments; awaiting written-spec review
- **Date:** 2026-10-22
- **Scope:** A small, reproducible screening experiment: 30 configurations per timeframe
  (`1hour`, `4hour`), 60 total, across BTC/ETH/SOL, with costs, chronological splits, expanding
  folds, and a one-shot holdout. This is a screening exercise, not proof of profitability.
- **Related:**
  - `docs/superpowers/specs/2026-10-21-analyzer-replay-simulator-design.md`
  - `docs/superpowers/specs/2026-10-21-analyzer-replay-report-polish-design.md`

## 1. Goal

Screen 30 unique configurations per timeframe (including the baseline) on BTC-USDT, ETH-USDT, and
SOL-USDT over the last 24 months, using the live-exact signal generator and the replay simulator
with provisional costs, and report validation vs holdout results separately with a clear
recommendation.

## 2. Non-goals

- No change to production defaults or live behavior; no live orders.
- No profitability claims from screening results.
- No full Cartesian product (648 combos); no automatic budget expansion.
- No `SMA_PERIOD` study (fixed at 20).

## 3. Locked decisions

| # | Decision |
|---|---|
| 1 | Symbols: BTC-USDT, ETH-USDT, SOL-USDT; report per-symbol and pooled. |
| 2 | History: last 24 months, identical range for both timeframes and all symbols. |
| 3 | Costs: base = 10 bps fee + 5 bps slippage per side; stress = 20 bps + 10 bps per side. Configurable. |
| 4 | Splits: chronological 60% train / 20% validation / 20% holdout, plus 3 expanding folds inside train+validation. |
| 5 | Baseline: crate `.env` (RSI 14, order 5, age cap 12, SMA on, SMA_PERIOD 20, SL 1.5×, TP 2.0×). |
| 6 | Cost model: apply costs once to actual simulated fills, on the correct notional. |
| 7 | Ranking: positive net validation expectancy → drawdown → fold consistency → trade count/uncertainty → stability. |
| 8 | Budget: exactly 30 unique configs per timeframe (60 total), baseline included. |
| 9 | Pagination and replay correctness are prerequisites; H1 must be in the worktree with tests. |
| 10 | Partition policy: each partition is a self-contained backtest; no cross-partition leakage. |

## 4. Parameter semantics (verified against code)

| Env var | Meaning | Units | Held constant |
|---|---|---|---|
| `RSI_PERIOD` | Wilder RSI period | candles | varied: 10, 14, 21 |
| `RSI_DIVERGENCE_ORDER` | peak/match radius | candles | varied: 3, 5, 7 |
| `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES` | divergence age cap | candles; **unset/empty = uncapped**; `0` = age 0 only | varied: 4, 8, 12, uncapped |
| `SMA_ENABLED` | SMA trend filter | bool | varied: false, true |
| `SMA_PERIOD` | SMA length | candles | **fixed 20** |
| `ATR_SL_MULTIPLIER` | stop = ATR × mult | multiple | varied: 1.5, 2.0, 2.5 |
| `ATR_TP_MULTIPLIER` | target = ATR × mult | multiple | varied: 2.0, 3.0, 4.0 |
| `PRICE_DIFF_THRESHOLD` | 7-candle return bonus | fraction | fixed 0.05 |
| `ATR_PERIOD` | ATR length | candles | fixed 14 |
| `ATR_PCT_HIGH` | high-vol threshold | **percent** | fixed 5.0 |
| `ATR_HIGH_ADJUST` | high-vol multiplier | multiple | fixed 1.5 |
| `ATR_SL_MAX_PCT` | stop cap | **fraction of price**; 0 disables | fixed 0.0 |
| `ANALYZER_WINDOW_CANDLES` | rolling window / warm-up | candles | fixed 100 |

Baseline = the crate `.env` config: `(14, 5, age 12, SMA on, 20, SL 1.5, TP 2.0)`. The resolved
configuration is recorded in every artifact so code defaults cannot be confused with the baseline.

## 5. Architecture

New `analyzer-sweep` binary in `process/analyzer-rs`, behind the `harness` feature. It runs
configurations **in-process** (no 60 process spawns) and does not touch `sim.rs` or production
defaults.

```
process/analyzer-rs/src/
  bin/analyzer-sweep.rs          CLI + orchestration
  harness/sweep/
    mod.rs
    data.rs                      paginated KuCoin fetch + cache + provenance
    costs.rs                     per-side cost accounting
    partition.rs                 chronological splits + expanding folds
    sampling.rs                  deterministic 30-config candidate generation
    evaluate.rs                  per-config, per-symbol, per-partition metrics
    ranking.rs                   selection + stability + "no candidate" logic
    report.rs                    CSV + JSON + Markdown report
```

### 5.1 Data pipeline (`data.rs`)

- Paginated KuCoin fetch: page backward from `as_of` using `endAt`/`startAt`, ≤1500 candles per
  request, until the 24-month target start is reached or no data returns.
- Sort ascending by `open_time_ms`; deduplicate; drop incomplete candles using the existing
  closed-candle rule (`Timeframe::is_closed` with `ANALYZER_CANDLE_SETTLE_MS`).
- Identical `[as_of - 24 months, as_of]` range for both timeframes and all symbols.
- Cache to CSV in the existing dataset format plus a provenance JSON (per symbol/timeframe):
  effective first/last open time, candle count, expected count, missing candles, duplicates
  dropped, pages fetched, content sha256, fetched_at. Reuse cache unless `--refresh`.
- Missing candles are **reported**, never fabricated. A dataset with any gap is flagged; the
  sweep reports gaps and continues only if the gap policy allows.

### 5.2 Cost model (`costs.rs`)

Applied once to actual simulated fills; never a generic round-trip percentage.

For a position of size `E` filled at `fill` and exited at `exit`:
```
quantity      q        = E / fill
entry_notional         = E
exit_notional          = q * exit
entry_cost             = (fee_bps + slip_bps)/10_000 * entry_notional
exit_cost              = (fee_bps + slip_bps)/10_000 * exit_notional
gross_pnl              = simulator pnl (proceeds - E for long, E - proceeds for short)
net_pnl                = gross_pnl - entry_cost - exit_cost
```
- The simulator's gross loss cap (`pnl >= -E`) is unchanged; net loss may slightly exceed `E`
  because costs are real. This is documented in the report.
- Open positions: entry cost is realized; exit cost is estimated at the partition's last close and
  included in unrealized; the open position is then closed at partition end (never carried).
- Two scenarios are run for every config: `base` (10/5) and `stress` (20/10).
- `sim.rs` is untouched.

### 5.3 Partitioning (`partition.rs`)

Per symbol and timeframe, on the closed candle series with the first `window` candles reserved for
warm-up:

- `holdout` = last 20% of the timeline (never used for selection; scored once, after freeze).
- `train` = first 60%, `validation` = next 20% (selection uses validation).
- 3 expanding folds inside train+validation:
  - fold 1: train `[0, 40%)`, validate `[40%, 55%)`
  - fold 2: train `[0, 55%)`, validate `[55%, 70%)`
  - fold 3: train `[0, 70%)`, validate `[70%, 80%)`
  Fold validations never reach the holdout (starts at 80%).

**Partition policy (no leakage):** each partition is evaluated as a self-contained backtest:
- A fresh driver is fed `window` warm-up candles immediately preceding the partition start, then
  the partition candles; only events/trades with `entry_index` inside the partition are counted.
- The account resets at partition start (`init_fund` per symbol).
- A position still open at partition end is marked to market at the partition's last close and
  dropped; it is not carried across the boundary.
- Warm-up uses only past candles and never crosses into the holdout.

### 5.4 Sampling (`sampling.rs`)

Exactly 30 unique configs per timeframe, baseline included, deterministic.

1. Baseline (1).
2. One-factor-at-a-time anchors (12): `RSI_PERIOD` 10/21; `RSI_DIVERGENCE_ORDER` 3/7;
   max-age 4/8/uncapped; `SMA_ENABLED=false`; `ATR_SL_MULTIPLIER` 2.0/2.5;
   `ATR_TP_MULTIPLIER` 3.0/4.0.
3. Deterministic seeded joint draws (17) from the full grid, using a recorded seed
   (default `20261022`), excluding exact duplicates of the baseline/anchors and excluding
   SMA-off variants that differ only by `SMA_PERIOD` (period is fixed 20).

The full grid is `3 × 3 × 4 × 2 × 3 × 3 = 648`. The artifact records the seed, the complete
candidate list, dedup results, and every attempted configuration. The study cannot determine the
best `SMA_PERIOD` (50/100/200), by design.

### 5.5 Evaluation (`evaluate.rs`)

Per config × symbol × timeframe × partition:
- Run the live-exact replay + simulator, apply base and stress costs.
- Metrics: closed trades, open-at-end, gross/net PnL, **net expectancy** (mean net PnL/trade),
  expectancy in R (`mean(net_pnl/E)`), profit factor (net), win rate, max drawdown (net equity
  peak-to-trough, absolute and % of init fund), exposure, TP/SL/other exits, and the standard error
  of net expectancy (std/√n).
- Pooled metrics aggregate across symbols (sum PnL/trades; expectancy = total net / total trades).

### 5.6 Ranking and recommendation (`ranking.rs`)

Selection uses **validation only**. Priority:
1. Positive net expectancy after costs under the base scenario. The stress scenario is reported as a
   robustness check; base-positive/stress-negative candidates are flagged, not silently dropped.
2. Max drawdown acceptable (provisional: every symbol's validation max drawdown ≤ 20% of that
   symbol's `init_fund`, default 1000).
3. Fold consistency: net expectancy > 0 in ≥ 2 of 3 expanding folds.
4. Sufficient trades (provisional: ≥ 30 pooled validation closed trades, no symbol > 70%) and
   reasonable uncertainty (report standard error / bootstrap 95% CI with a fixed seed).
5. Stability: positive on ≥ 2 of 3 symbols, and the config's validation expectancy within one
   standard error of its one-step grid neighbors' median.

If no config satisfies 1–3, the report states **no candidate meets the criteria** rather than
selecting the least-bad configuration. The holdout is scored once for the frozen selection
(baseline + any qualifying candidates) and reported separately.

## 6. Deliverables

- `data/sweep/<symbol>_<timeframe>.csv` + `.provenance.json` (cached, untracked).
- `process/analyzer-rs/experiments/sweep/leaderboard.csv` — every attempted config with all metrics (base + stress).
- `process/analyzer-rs/experiments/sweep/results.json` — full structured results (configs, splits, folds, provenance).
- `process/analyzer-rs/experiments/sweep/report.md` — the human report: baseline vs candidates; exact configs and selection
  reasons; trades, net expectancy, profit factor, max drawdown, costs; walk-forward per timeframe
  and symbol; parameter sensitivity; data-quality issues; commands executed; recommendation
  (retain baseline / larger experiment / insufficient evidence).
- Validation (selection) and holdout (evaluation) results are clearly separated in every artifact.

## 7. Testing

Prerequisite tests (must pass before the study runs):
- **Pagination:** mocked multi-page responses → correctly ordered, deduplicated candles; target
  range respected; missing candles reported.
- **Closed-candle filtering:** both timeframes; incomplete candle excluded at `as_of`.
- **H1 timeframe:** the uncommitted `1hour` support is included in the worktree and covered.
- **Costs:** long and short; winning and losing; open position; base and stress; applied exactly
  once on the correct notional.
- **Partitioning:** boundaries, no overlap with holdout, warm-up does not cross into the holdout,
  expanding folds, open-position-at-boundary policy.
- **Sampling:** deterministic seed, exactly 30 unique per timeframe, baseline included, no
  SMA-off duplicates, recorded seed and candidate list.
- **Ranking:** priority ordering; the "no candidate" path.
- **Replay parity:** the sweep reuses the existing live-exact generator; existing parity/live tests
  still pass.

## 8. Safety

- No production default changes; `sim.rs` untouched.
- No live trading, no order submission.
- Screening results are labeled provisional and not a profitability claim.

## 9. Risks

- 24 months at 1h is ~17.5k candles; a single 1h strategy may produce few trades → wide
  uncertainty. Reported via trade counts and standard errors; may lead to "insufficient evidence".
- KuCoin history corrections and fetch timing; mitigated by caching with content hashes and a
  fixed `as_of`.
- Costs are provisional assumptions, not verified KuCoin fees; the stress scenario bounds this.
