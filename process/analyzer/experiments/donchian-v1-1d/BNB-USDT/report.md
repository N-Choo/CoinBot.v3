# Donchian Trend-Following — BNB-USDT (1day, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 1day |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 455 |
| SHA-256 | 8fe35580aebce075f64b58f6ce5e0116774acdd2c011253df2b8b373f0714d4e |

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

Buy & hold benchmark (half budget): base net PnL -170.5392 (-34.1078%), stress net PnL -171.7853 (-34.3571%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -170.5392 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -171.7853 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -170.5392 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -171.7853 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -170.5392 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -171.7853 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -45.3395 (-9.0679%), stress net PnL -46.7737 (-9.3547%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -45.3395 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -46.7737 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -45.3395 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -46.7737 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -45.3395 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -46.7737 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 46.6024 (9.3205%), stress net PnL 45.0302 (9.0060%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | 46.6024 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | 45.0302 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | 46.6024 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | 45.0302 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | 46.6024 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | 45.0302 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 103.8881 (20.7776%), stress net PnL 102.2298 (20.4460%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | -15.0982 | 0.0173 | 3.0711 | -30.1965 | 103.8881 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -16.5778 | 0 | 3.3156 | -33.1556 | 102.2298 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 1 | 12.4733 | ∞ | 0 | 12.4733 | 103.8881 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 1 | 10.9523 | ∞ | 0 | 10.9523 | 102.2298 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 60.5847 | 103.8881 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 58.9914 | 102.2298 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 71.7069 (14.3414%), stress net PnL 70.0970 (14.0194%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 3 | -20.1025 | 0.0087 | 6.0806 | -60.3074 | 71.7069 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | -21.5745 | 0 | 6.4724 | -64.7235 | 70.0970 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -11.4962 | 0.3517 | 3.5029 | -22.9925 | 71.7069 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -12.9812 | 0.2967 | 3.6515 | -25.9624 | 70.0970 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | 25.2867 | ∞ | 0 | 25.2867 | 71.7069 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 23.7464 | ∞ | 0 | 23.7464 | 70.0970 | no | Fail |

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
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 0 |
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 10.9523 |
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 23.7464 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | 0 | -171.7853 | 171.7853 | Beat |
| M1-M7 | entry20-exit55-ema200-atr14x4 | 0 | -46.7737 | 46.7737 | Beat |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 0 | 45.0302 | -45.0302 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x4 | 58.9914 | 102.2298 | -43.2384 | Underperform |
| M1-M10 | entry20-exit55-ema200-atr14x4 | 23.7464 | 70.0970 | -46.3505 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -171.78525551031868594693247227.
- M1-M7: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -46.773669232354835153070436849.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark 45.030163340541489455079356218.
- M1-M9: best top-3 stress run entry20-exit55-ema200-atr14x3 had 1 OOS trades, net expectancy 10.952292930198755350195185903, PF infinite, net PnL 10.952292930198755350195185903 vs benchmark 102.22979169313837605671142150.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x4 had 1 OOS trades, net expectancy 23.746436539976540605684896893, PF infinite, net PnL 23.746436539976540605684896893 vs benchmark 70.096957955761025931071793039.

Successful runs: none.

