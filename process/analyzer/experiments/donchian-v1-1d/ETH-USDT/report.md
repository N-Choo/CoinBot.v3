# Donchian Trend-Following — ETH-USDT (1day, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | ETH-USDT |
| Timeframe | 1day |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 455 |
| SHA-256 | 6e02f6d730d87fb49df2f797bedf268f2c278cce463d6e9fc8df19da153898d5 |

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

Buy & hold benchmark (half budget): base net PnL -214.8875 (-42.9775%), stress net PnL -216.0669 (-43.2134%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -214.8875 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -216.0669 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -214.8875 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -216.0669 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -214.8875 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -216.0669 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -10.3392 (-2.0678%), stress net PnL -11.8260 (-2.3652%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -10.3392 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -11.8260 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -10.3392 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -11.8260 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -10.3392 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -11.8260 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 110.8835 (22.1767%), stress net PnL 109.2146 (21.8429%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 56.9464 | 110.8835 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 55.3586 | 109.2146 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 56.9464 | 110.8835 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 55.3586 | 109.2146 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 56.9464 | 110.8835 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 55.3586 | 109.2146 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 55.4815 (11.0963%), stress net PnL 53.8959 (10.7792%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 1 | 22.9976 | ∞ | 0 | 22.9976 | 55.4815 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 1 | 21.4608 | ∞ | 0 | 21.4608 | 53.8959 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 1 | 25.7114 | ∞ | 0 | 25.7114 | 55.4815 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 1 | 24.1705 | ∞ | 0 | 24.1705 | 53.8959 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 85.5534 | 55.4815 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 83.9226 | 53.8959 | yes | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 96.9867 (19.3973%), stress net PnL 95.3387 (19.0677%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | -2.4423 | 0.8248 | 2.7255 | -4.8845 | 96.9867 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -3.9408 | 0.7314 | 2.8726 | -7.8817 | 95.3387 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -8.2199 | 0.6100 | 4.1095 | -16.4398 | 96.9867 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -9.7098 | 0.5545 | 4.2561 | -19.4196 | 95.3387 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | 45.7864 | ∞ | 0 | 45.7864 | 96.9867 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 44.2154 | ∞ | 0 | 44.2154 | 95.3387 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 |
|---|---|---|---|
| M1-M6 | Fail | Fail | Fail |
| M1-M7 | Fail | Fail | Fail |
| M1-M8 | Fail | Fail | Fail |
| M1-M9 | Fail | Fail | Fail |
| M1-M10 | Fail | Fail | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 21.4608 |
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 24.1705 |
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 44.2154 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | 0 | -216.0669 | 216.0669 | Beat |
| M1-M7 | entry20-exit55-ema200-atr14x4 | 0 | -11.8260 | 11.8260 | Beat |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 55.3586 | 109.2146 | -53.8560 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x4 | 83.9226 | 53.8959 | 30.0267 | Beat |
| M1-M10 | entry20-exit55-ema200-atr14x4 | 44.2154 | 95.3387 | -51.1233 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -216.06694439322056629394310238.
- M1-M7: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -11.825966387835016988922917523.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 55.358579020035249253062112378 vs benchmark 109.21462200994210985008934266.
- M1-M9: best top-3 stress run entry20-exit55-ema200-atr14x3 had 1 OOS trades, net expectancy 24.170486980170054490452157389, PF infinite, net PnL 24.170486980170054490452157389 vs benchmark 53.895904494358360955393356388.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x4 had 1 OOS trades, net expectancy 44.215395908148442639358994656, PF infinite, net PnL 44.215395908148442639358994656 vs benchmark 95.33872833489020429696385963.

Successful runs: none.

