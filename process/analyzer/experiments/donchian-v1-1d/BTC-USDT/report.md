# Donchian Trend-Following — BTC-USDT (1day, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 455 |
| SHA-256 | e88ab80acc9d335ed7fee81c387b7e0279a922afb8910981c593a7a8cf23553a |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M6 | 0..182 | 182 | M7-M11 | 182..333 | 151 | 2025-07-24T00:00:00Z | 2026-01-21T00:00:00Z | 2026-01-22T00:00:00Z | 2026-06-21T00:00:00Z |
| M1-M7 | 0..212 | 212 | M8-M12 | 212..364 | 152 | 2025-07-24T00:00:00Z | 2026-02-20T00:00:00Z | 2026-02-21T00:00:00Z | 2026-07-22T00:00:00Z |
| M1-M8 | 0..242 | 242 | M9-M13 | 242..394 | 152 | 2025-07-24T00:00:00Z | 2026-03-22T00:00:00Z | 2026-03-23T00:00:00Z | 2026-08-21T00:00:00Z |
| M1-M9 | 0..273 | 273 | M10-M14 | 273..424 | 151 | 2025-07-24T00:00:00Z | 2026-04-22T00:00:00Z | 2026-04-23T00:00:00Z | 2026-09-20T00:00:00Z |
| M1-M10 | 0..303 | 303 | M11-M15 | 303..455 | 152 | 2025-07-24T00:00:00Z | 2026-05-22T00:00:00Z | 2026-05-23T00:00:00Z | 2026-10-21T00:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy | donchian-v1 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT, BNB-USDT, XRP-USDT (5) |
| Entry | Close > prior Donchian high (N ∈ [20, 55]) and close > EMA(200) |
| Exit | ATR(14) trailing stop (highest high since entry − multiplier × ATR) or close < EMA(200) at the next open |
| Grid | entry N [20, 55] × Donchian exit N 55 × ATR multiplier [2, 3, 4] × EMA 200 × ATR 14 = 6 configs (Donchian exit N is fixed metadata and is not an active exit in this variant) |
| Walk-forward | Expanding training windows M1-M6 through M1-M10; each tested on the next 5 months |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Initial fund | 1000 |
| Budget per position | init_fund / 2 |

Execution assumptions:

- Long-only; one position per symbol; no shorts and no pyramiding.
- A signal produced at a candle close fills at the next candle open.
- The trailing stop is checked intrabar using the highest high and ATR known at the prior close. A gap below the stop exits at the open; otherwise the exit price is the stop level.
- An EMA cross below is acted on at the next open; the stop takes precedence when both trigger in the same bar.
- Costs are charged on the actual entry and exit notional, not on the initial budget.

Selection and pass policy:

- Each window ranks all 6 configs on base-cost training metrics by the sum of the profit-factor rank (∞ first) and net-expectancy rank. The top 3 advance to the holdout.
- There is no minimum-trade gate.
- A run passes only when stress OOS net expectancy > 0, stress OOS PF > 1, and stress OOS net PnL > the stress buy-and-hold benchmark.
- A symbol passes when any top-3 run in any window passes. The experiment passes only when all 5 symbols pass.
- The benchmark buys at the first OOS open and sells at the last OOS close with the same half-budget and cost scenario.

## 3. Validation

Top 3 configs per expanding training window, selected on base-cost training metrics.

### M1-M6 training (M7-M11)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -147.4191 (-29.4838%), stress net PnL -148.6999 (-29.7400%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -147.4191 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -148.6999 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -147.4191 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -148.6999 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -147.4191 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -148.6999 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -15.4339 (-3.0868%), stress net PnL -16.9130 (-3.3826%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -15.4339 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -16.9130 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -15.4339 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -16.9130 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -15.4339 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -16.9130 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 75.5151 (15.1030%), stress net PnL 73.8994 (14.7799%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 34.8113 | 75.5151 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 33.2568 | 73.8994 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 34.8113 | 75.5151 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 33.2568 | 73.8994 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 34.8113 | 75.5151 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 33.2568 | 73.8994 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 17.5892 (3.5178%), stress net PnL 16.0605 (3.2121%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 1 | 24.8089 | ∞ | 0 | 24.8089 | 17.5892 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 1 | 23.2694 | ∞ | 0 | 23.2694 | 16.0605 | yes | Pass |
| entry20-exit55-ema200-atr14x3 | Base | 1 | 17.0442 | ∞ | 0 | 17.0442 | 17.5892 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 1 | 15.5164 | ∞ | 0 | 15.5164 | 16.0605 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 54.2007 | 17.5892 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 52.6170 | 16.0605 | yes | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 55.0513 (11.0103%), stress net PnL 53.4663 (10.6933%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | 0.6899 | 1.0589 | 2.2862 | 1.3798 | 55.0513 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -0.8134 | 0.9347 | 2.4330 | -1.6268 | 53.4663 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -9.0484 | 0.4850 | 3.4552 | -18.0969 | 55.0513 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -10.5371 | 0.4241 | 3.6031 | -21.0742 | 53.4663 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 74.2280 | 55.0513 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 72.6142 | 53.4663 | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 |
|---|---|---|---|
| M1-M6 | Fail | Fail | Fail |
| M1-M7 | Fail | Fail | Fail |
| M1-M8 | Fail | Fail | Fail |
| M1-M9 | Pass | Fail | Fail |
| M1-M10 | Fail | Fail | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x2 | 5 | 1 | 23.2694 |
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 15.5164 |
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 0 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | 0 | -148.6999 | 148.6999 | Beat |
| M1-M7 | entry20-exit55-ema200-atr14x4 | 0 | -16.9130 | 16.9130 | Beat |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 33.2568 | 73.8994 | -40.6427 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x4 | 52.6170 | 16.0605 | 36.5566 | Beat |
| M1-M10 | entry20-exit55-ema200-atr14x4 | 72.6142 | 53.4663 | 19.1479 | Beat |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M9: entry20-exit55-ema200-atr14x2 passed OOS stress with 1 trades, net expectancy 23.269374333642324827376574229, PF infinite, net PnL 23.269374333642324827376574229 > benchmark 16.060487280141313287516516186.

Successful runs:

| Window | Config | OOS trades | Stress expectancy | Stress PF | Stress net PnL | Benchmark stress net PnL |
|---|---|---:|---:|---:|---:|---:|
| M1-M9 | entry20-exit55-ema200-atr14x2 | 1 | 23.2694 | ∞ | 23.2694 | 16.0605 |

