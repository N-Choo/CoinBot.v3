# Donchian Trend-Following — XRP-USDT (1day, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 1day |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 455 |
| SHA-256 | 0abcc7a36c8030e2df64672ccd602128d9a4162a5f2a496468069e564837c69c |

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

Buy & hold benchmark (half budget): base net PnL -212.3530 (-42.4706%), stress net PnL -213.5363 (-42.7073%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -212.3530 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -213.5363 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -212.3530 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -213.5363 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -212.3530 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -213.5363 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -101.8049 (-20.3610%), stress net PnL -103.1542 (-20.6308%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -101.8049 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -103.1542 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -101.8049 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -103.1542 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -101.8049 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -103.1542 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 23.4760 (4.6952%), stress net PnL 21.9385 (4.3877%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | 23.4760 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | 21.9385 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | 23.4760 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | 21.9385 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | 23.4760 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | 21.9385 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL -8.1426 (-1.6285%), stress net PnL -9.6326 (-1.9265%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 1 | 1.2156 | ∞ | 0 | 1.2156 | -8.1426 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 1 | -0.2885 | 0 | 0.0288 | -0.2885 | -9.6326 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 1 | 0.0191 | ∞ | 0 | 0.0191 | -8.1426 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 1 | -1.4832 | 0 | 0.1483 | -1.4832 | -9.6326 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | -32.6329 | 0 | 3.2633 | -32.6329 | -8.1426 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | -34.0862 | 0 | 3.4086 | -34.0862 | -9.6326 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 9.8594 (1.9719%), stress net PnL 8.3424 (1.6685%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | -11.0787 | 0.0520 | 2.3345 | -22.1574 | 9.8594 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -12.5643 | 0 | 2.5129 | -25.1286 | 8.3424 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -16.1216 | 0.0006 | 3.2262 | -32.2431 | 9.8594 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -17.5996 | 0 | 3.5199 | -35.1992 | 8.3424 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 2 | -45.1620 | 0 | 9.0324 | -90.3239 | 9.8594 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | -46.5964 | 0 | 9.3193 | -93.1927 | 8.3424 | no | Fail |

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
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 0 |
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 0 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | 0 | -213.5363 | 213.5363 | Beat |
| M1-M7 | entry20-exit55-ema200-atr14x4 | 0 | -103.1542 | 103.1542 | Beat |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 0 | 21.9385 | -21.9385 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x2 | -0.2885 | -9.6326 | 9.3441 | Beat |
| M1-M10 | entry20-exit55-ema200-atr14x2 | -25.1286 | 8.3424 | -33.4710 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -213.53625011552798858070015096.
- M1-M7: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -103.15416803700256806779139172.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark 21.938498754377730440119868579.
- M1-M9: best top-3 stress run entry20-exit55-ema200-atr14x2 had 1 OOS trades, net expectancy -0.2884879866870212209981983290, PF 0, net PnL -0.2884879866870212209981983290 vs benchmark -9.632624937041804241983323070.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x2 had 2 OOS trades, net expectancy -12.564307584574834010933533976, PF 0, net PnL -25.128615169149668021867067952 vs benchmark 8.342362354562958412310735186.

Successful runs: none.

