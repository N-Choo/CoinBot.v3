# Donchian Trend-Following — XRP-USDT (4hour, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 759c89f2eb5d0e60e4e9d20f48ed5cf5a1f82e3230d9046582cee830444df10c |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M6 | 0..1094 | 1094 | M7-M11 | 1094..2005 | 911 | 2025-07-24T00:00:00Z | 2026-01-22T04:00:00Z | 2026-01-22T08:00:00Z | 2026-06-23T00:00:00Z |
| M1-M7 | 0..1276 | 1276 | M8-M12 | 1276..2188 | 912 | 2025-07-24T00:00:00Z | 2026-02-21T12:00:00Z | 2026-02-21T16:00:00Z | 2026-07-23T12:00:00Z |
| M1-M8 | 0..1458 | 1458 | M9-M13 | 1458..2370 | 912 | 2025-07-24T00:00:00Z | 2026-03-23T20:00:00Z | 2026-03-24T00:00:00Z | 2026-08-22T20:00:00Z |
| M1-M9 | 0..1641 | 1641 | M10-M14 | 1641..2552 | 911 | 2025-07-24T00:00:00Z | 2026-04-23T08:00:00Z | 2026-04-23T12:00:00Z | 2026-09-22T04:00:00Z |
| M1-M10 | 0..1823 | 1823 | M11-M15 | 1823..2735 | 912 | 2025-07-24T00:00:00Z | 2026-05-23T16:00:00Z | 2026-05-23T20:00:00Z | 2026-10-22T16:00:00Z |

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
| 1 | entry55-exit55-ema200-atr14x2 | 5 | 10.1008 | 2.4206 | 2.8082 | 17.5747 | 8.5834 | 2.0728 |
| 2 | entry20-exit55-ema200-atr14x2 | 8 | 5.1186 | 1.7499 | 2.8834 | 11.0227 | 3.6086 | 1.4654 |
| 3 | entry55-exit55-ema200-atr14x3 | 5 | 4.5666 | 1.5220 | 4.2583 | 15.7086 | 3.0574 | 1.3150 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x2 | 5 | 10.1008 | 2.4206 | 2.8082 | 17.5747 | 8.5834 | 2.0728 |
| 2 | entry20-exit55-ema200-atr14x2 | 8 | 5.1186 | 1.7499 | 2.8834 | 11.0227 | 3.6086 | 1.4654 |
| 3 | entry55-exit55-ema200-atr14x3 | 5 | 4.5666 | 1.5220 | 4.2583 | 15.7086 | 3.0574 | 1.3150 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x2 | 6 | 12.5005 | 3.1098 | 2.8082 | 14.5489 | 10.9794 | 2.6468 |
| 2 | entry20-exit55-ema200-atr14x2 | 9 | 7.2719 | 2.1986 | 2.8834 | 9.9568 | 5.7587 | 1.8355 |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 6.3152 | 1.8662 | 4.2583 | 12.9446 | 4.8034 | 1.5939 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x2 | 7 | 13.6282 | 3.6835 | 2.8082 | 12.3477 | 12.1055 | 3.1183 |
| 2 | entry55-exit55-ema200-atr14x3 | 7 | 7.0930 | 2.1351 | 4.2583 | 10.9678 | 5.5801 | 1.8049 |
| 3 | entry20-exit55-ema200-atr14x2 | 12 | 5.6785 | 1.9425 | 2.8834 | 7.7048 | 4.1677 | 1.6047 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x2 | 10 | 5.0582 | 1.6294 | 4.0913 | 9.5114 | 3.5483 | 1.3975 |
| 2 | entry20-exit55-ema200-atr14x2 | 16 | 3.0442 | 1.4883 | 3.0120 | 5.9734 | 1.5374 | 1.2146 |
| 3 | entry55-exit55-ema200-atr14x3 | 10 | -1.8723 | 0.8330 | 6.5140 | 8.7997 | -3.3717 | 0.7220 |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -211.7857 (-42.3571%), stress net PnL -212.9698 (-42.5940%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 6 | -1.6753 | 0.8171 | 5.2585 | -10.0519 | -211.7857 | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 6 | -3.1751 | 0.6870 | 5.8428 | -19.0503 | -212.9698 | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 9 | -0.2634 | 0.9571 | 4.1042 | -2.3707 | -211.7857 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 9 | -1.7653 | 0.7526 | 4.9864 | -15.8874 | -212.9698 | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 6 | -9.1907 | 0.3272 | 7.9822 | -55.1442 | -211.7857 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | -10.6791 | 0.2706 | 8.5809 | -64.0748 | -212.9698 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -117.5484 (-23.5097%), stress net PnL -118.8740 (-23.7748%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 8 | -3.0031 | 0.6514 | 6.5957 | -24.0244 | -117.5484 | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 8 | -4.5008 | 0.5374 | 7.4704 | -36.0064 | -118.8740 | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 12 | -1.6548 | 0.7421 | 6.1927 | -19.8581 | -117.5484 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 12 | -3.1546 | 0.5742 | 7.3710 | -37.8554 | -118.8740 | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -9.6680 | 0.2575 | 10.1442 | -77.3441 | -117.5484 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -11.1557 | 0.2104 | 11.0395 | -89.2459 | -118.8740 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 9.0410 (1.8082%), stress net PnL 7.5251 (1.5050%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 11 | 1.4658 | 1.2036 | 6.8227 | 16.1237 | 9.0410 | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 11 | -0.0387 | 0.9953 | 7.7278 | -0.4253 | 7.5251 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 15 | 1.1483 | 1.1907 | 6.4264 | 17.2244 | 9.0410 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 15 | -0.3557 | 0.9486 | 7.8360 | -5.3353 | 7.5251 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | -0.4257 | 0.9628 | 10.4007 | -4.2572 | 9.0410 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | -1.9273 | 0.8455 | 11.4532 | -19.2733 | 7.5251 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 34.6543 (6.9309%), stress net PnL 33.1000 (6.6200%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 10 | -0.9473 | 0.8877 | 6.8918 | -16.6641 | 34.6543 | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 10 | -2.4482 | 0.7417 | 7.7828 | -33.1639 | 33.1000 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 9 | -3.1967 | 0.7738 | 10.4163 | -35.9612 | 34.6543 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 9 | -4.6942 | 0.6928 | 11.3020 | -50.9297 | 33.1000 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 16 | -1.4962 | 0.7927 | 5.4632 | -23.9393 | 34.6543 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 16 | -2.9962 | 0.6321 | 6.1385 | -47.9393 | 33.1000 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL -16.9274 (-3.3855%), stress net PnL -18.4042 (-3.6808%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 8 | 5.7906 | 2.1707 | 2.4102 | 46.3246 | -16.9274 | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 8 | 4.2796 | 1.7521 | 2.8573 | 34.2370 | -18.4042 | yes | Pass |
| entry20-exit55-ema200-atr14x2 | Base | 13 | -1.5765 | 0.8029 | 7.2506 | -20.4945 | -16.9274 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 13 | -3.0764 | 0.6548 | 8.3070 | -39.9930 | -18.4042 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 7 | 5.1680 | 1.5813 | 3.5789 | 36.1759 | -16.9274 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 7 | 3.6580 | 1.3676 | 4.0242 | 25.6058 | -18.4042 | yes | Pass |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 |
|---|---|---|---|
| M1-M6 | Fail | Fail | Fail |
| M1-M7 | Fail | Fail | Fail |
| M1-M8 | Fail | Fail | Fail |
| M1-M9 | Fail | Fail | Fail |
| M1-M10 | Fail | Pass | Pass |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x2 | 5 | 1 | 4.2796 |
| entry55-exit55-ema200-atr14x3 | 5 | 1 | 3.6580 |
| entry20-exit55-ema200-atr14x2 | 5 | 0 | -0.3557 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x2 | -15.8874 | -212.9698 | 197.0823 | Beat |
| M1-M7 | entry55-exit55-ema200-atr14x2 | -36.0064 | -118.8740 | 82.8677 | Beat |
| M1-M8 | entry55-exit55-ema200-atr14x2 | -0.4253 | 7.5251 | -7.9504 | Underperform |
| M1-M9 | entry55-exit55-ema200-atr14x2 | -33.1639 | 33.1000 | -66.2638 | Underperform |
| M1-M10 | entry55-exit55-ema200-atr14x2 | 34.2370 | -18.4042 | 52.6412 | Beat |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M10: entry55-exit55-ema200-atr14x2 passed OOS stress with 8 trades, net expectancy 4.2796263417056781359379627028, PF 1.7521377051338570960235409494, net PnL 34.237010733645425087503701622 > benchmark -18.404212440617577197149643706.
- M1-M10: entry55-exit55-ema200-atr14x3 passed OOS stress with 7 trades, net expectancy 3.6579670924193830563774763864, PF 1.3676212187273079765173661831, net PnL 25.605769646935681394642334705 > benchmark -18.404212440617577197149643706.

Successful runs:

| Window | Config | OOS trades | Stress expectancy | Stress PF | Stress net PnL | Benchmark stress net PnL |
|---|---|---:|---:|---:|---:|---:|
| M1-M10 | entry55-exit55-ema200-atr14x2 | 8 | 4.2796 | 1.7521 | 34.2370 | -18.4042 |
| M1-M10 | entry55-exit55-ema200-atr14x3 | 7 | 3.6580 | 1.3676 | 25.6058 | -18.4042 |

