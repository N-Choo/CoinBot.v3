# Analyzer Fresh Confirmation — Pre-registration

- **Status:** Frozen; one-shot confirmation approved
- **Date:** 2026-10-22
- **Purpose:** Confirm or reject the `max_age=4` + `retest_2c_b2` result on a fresh, disjoint 24-month period. No tuning after this point.

## 1. Freeze

- **Signal generator:** frozen at commit `91e79e1` (no changes to `src/strategy`, `src/driver`, `harness/replay.rs`, `harness/sim.rs`, or `harness/sweep/execution.rs`).
- **Frozen file set and SHA-256:** recorded in `freeze/confirmation-lock.json` and verified by the confirmation runner. The hash covers 18 files under `src/strategy`, `src/driver`, `harness/replay.rs`, `harness/sim.rs`, and `harness/sweep/execution.rs`.
- **Frozen config:** baseline values with `max_age=4`:
  `RSI_PERIOD=14`, `RSI_DIVERGENCE_ORDER=5`, `ANALYZER_MAX_DIVERGENCE_AGE_CANDLES=4`, `SMA_ENABLED=true`, `SMA_PERIOD=20`, `ATR_PERIOD=14`, `ATR_SL_MULTIPLIER=1.5`, `ATR_TP_MULTIPLIER=2.0`, `PRICE_DIFF_THRESHOLD=0.05`, `ATR_PCT_HIGH=5.0`, `ATR_HIGH_ADJUST=1.5`, `ATR_SL_MAX_PCT=0.0`, `ANALYZER_WINDOW_CANDLES=100`.
- **Frozen execution model:** `retest_2c_b2` — limit 25 bps better than signal entry, valid 2 candles, 2 bps fill buffer, maker 2 bps entry/TP, taker 15 bps (10 fee + 5 slippage) SL/gap/EOD, 50% of `init_fund` per position, EOD close at partition end.

## 2. Fresh dataset

- Symbols: BTC-USDT, ETH-USDT, SOL-USDT.
- Timeframe: 4hour.
- Window: 24 months ending **2024-10-22T00:00:00Z** (`as_of_ms=1729555200000`), start `2022-11-02T00:00:00Z` (`1667347200000`).
- This period is **disjoint** from the screening window (2024-11-01 → 2026-10-22).
- Fetch with pagination; record effective range, candle count, missing candles, duplicates, content SHA-256. Do not fabricate missing candles.

## 3. Evaluation

- The whole fresh period is out-of-sample; no model or config selection is performed.
- For each symbol: full-period slice (first 100 candles warm-up), replay with the frozen config/MaxAge, simulate with `retest_2c_b2`, close at the last close.
- Pool across symbols.
- Primary metrics: closed trades, net expectancy, net profit factor, max drawdown, win rate, net PnL, standard error, execution fill rate, capital fill rate.
- Secondary: per-symbol metrics.

## 4. Decision rule (pre-registered)

The confirmation **passes** only if all of:
1. pooled closed trades ≥ 50,
2. pooled net expectancy > 0,
3. pooled net profit factor > 1.0,
4. at least 2 of 3 symbols have net expectancy > 0.

If any condition fails, the result is **confirmation failed** and no further tuning is allowed on this period. If it passes, the result is **confirmed** for paper/shadow evaluation, not live deployment.

## 5. Artifacts

- `freeze/confirmation-lock.json` (commit, hashes, config, model, dataset, decision rule).
- `process/analyzer-rs/experiments/confirm/report.md`, `confirmation.csv`, `results.json`.
- The runner aborts if the frozen-file hash or the lock does not match.

## 6. Guardrails

- One run only; no parameter changes after the freeze.
- No live orders; no production-default changes.
- Holdout reuse is not an issue here because this period was never used in prior experiments.
- Costs remain provisional assumptions.
