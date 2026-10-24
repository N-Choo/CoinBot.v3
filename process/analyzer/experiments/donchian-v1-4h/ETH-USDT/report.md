# Donchian Trend-Following — ETH-USDT (4hour, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | ETH-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 1357ea92f4d97764eb54cac30a1bce587e60400c3848332650ddce336ecbb0ed |

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
| 1 | entry20-exit55-ema200-atr14x3 | 6 | 11.6229 | 6.0272 | 1.3271 | 6.0300 | 10.1032 | 4.5964 |
| 2 | entry20-exit55-ema200-atr14x4 | 6 | 3.6907 | 1.8788 | 2.4550 | 5.3282 | 2.1829 | 1.4650 |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 3.0678 | 1.9370 | 1.3499 | 4.6713 | 1.5609 | 1.3883 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x3 | 6 | 11.6229 | 6.0272 | 1.3271 | 6.0300 | 10.1032 | 4.5964 |
| 2 | entry20-exit55-ema200-atr14x4 | 6 | 3.6907 | 1.8788 | 2.4550 | 5.3282 | 2.1829 | 1.4650 |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 3.0678 | 1.9370 | 1.3499 | 4.6713 | 1.5609 | 1.3883 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x3 | 9 | 6.5046 | 3.1924 | 1.3271 | 4.7226 | 4.9926 | 2.3753 |
| 2 | entry20-exit55-ema200-atr14x2 | 13 | 2.4223 | 1.5993 | 2.7762 | 3.4075 | 0.9164 | 1.1938 |
| 3 | entry55-exit55-ema200-atr14x3 | 8 | 1.6582 | 1.5021 | 1.5537 | 3.6294 | 0.1535 | 1.0379 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x3 | 15 | -1.8513 | 0.7543 | 9.1151 | 4.0206 | -3.3508 | 0.6069 |
| 2 | entry20-exit55-ema200-atr14x2 | 20 | -2.2219 | 0.6541 | 8.2567 | 2.6985 | -3.7208 | 0.4965 |
| 3 | entry55-exit55-ema200-atr14x3 | 11 | -2.7038 | 0.5716 | 5.5799 | 3.5707 | -4.2020 | 0.4211 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x3 | 18 | -3.4991 | 0.5751 | 12.4068 | 3.4533 | -4.9960 | 0.4632 |
| 2 | entry20-exit55-ema200-atr14x2 | 25 | -3.7449 | 0.4730 | 12.9788 | 2.2581 | -5.2415 | 0.3590 |
| 3 | entry55-exit55-ema200-atr14x3 | 12 | -4.2580 | 0.4372 | 7.6579 | 3.6111 | -5.7538 | 0.3275 |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -212.9697 (-42.5939%), stress net PnL -214.1521 (-42.8304%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 12 | -11.2079 | 0.0120 | 13.4495 | -134.4947 | -212.9697 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 12 | -12.6933 | 0.0009 | 15.2320 | -152.3197 | -214.1521 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | -13.9305 | 0 | 15.3236 | -153.2358 | -212.9697 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | -15.4118 | 0 | 16.9530 | -169.5303 | -214.1521 | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 6 | -11.5837 | 0.0230 | 6.9502 | -69.5024 | -212.9697 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | -13.0686 | 0.0017 | 7.8412 | -78.4116 | -214.1521 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -24.8086 (-4.9617%), stress net PnL -26.2736 (-5.2547%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 15 | -10.4092 | 0.0104 | 15.7515 | -156.1375 | -24.8086 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 15 | -11.8958 | 0.0007 | 17.8543 | -178.4367 | -26.2736 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 12 | -12.6283 | 0.0220 | 15.4947 | -152.4807 | -24.8086 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | -14.1116 | 0.0111 | 17.1781 | -171.7809 | -26.2736 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -10.2706 | 0.0195 | 8.3663 | -82.1650 | -24.8086 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -11.7575 | 0.0014 | 9.4177 | -94.0596 | -26.2736 | no | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 61.2281 (12.2456%), stress net PnL 59.6339 (11.9268%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 16 | -10.9065 | 0 | 17.4503 | -38.9672 | 61.2281 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 16 | -12.3923 | 0 | 19.8277 | -64.4470 | 59.6339 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 22 | -3.5899 | 0.5268 | 14.1069 | -78.9789 | 61.2281 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 22 | -5.0868 | 0.4267 | 17.0902 | -111.9098 | 59.6339 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 7 | -11.9714 | 0 | 8.3800 | -4.6839 | 61.2281 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 7 | -13.4556 | 0 | 9.4189 | -16.6949 | 59.6339 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 83.5873 (16.7175%), stress net PnL 81.9595 (16.3919%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 13 | 0.0526 | 1.0055 | 9.1797 | 26.6062 | 83.5873 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 13 | -1.4497 | 0.8684 | 10.6682 | 5.5347 | 81.9595 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 19 | -1.8034 | 0.7196 | 7.3239 | -34.2645 | 83.5873 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 19 | -3.3029 | 0.5703 | 9.2658 | -62.7558 | 81.9595 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 6 | 2.3979 | 1.2567 | 4.0792 | 13.0544 | 83.5873 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | 0.8921 | 1.0843 | 4.6740 | 2.5190 | 81.9595 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 72.8678 (14.5736%), stress net PnL 71.2561 (14.2512%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 12 | 3.2838 | 1.3787 | 5.6584 | 39.4060 | 72.8678 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 12 | 1.7766 | 1.1793 | 6.7015 | 21.3198 | 71.2561 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 15 | 0.1508 | 1.0264 | 5.3461 | 2.2626 | 72.8678 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 15 | -1.3516 | 0.8042 | 6.2699 | -20.2746 | 71.2561 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 6 | 4.3966 | 1.5989 | 2.3414 | 26.3799 | 72.8678 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | 2.8878 | 1.3365 | 2.6391 | 17.3267 | 71.2561 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x3 |
|---|---|---|---|---|
| M1-M6 | — | Fail | Fail | Fail |
| M1-M7 | — | Fail | Fail | Fail |
| M1-M8 | Fail | Fail | — | Fail |
| M1-M9 | Fail | Fail | — | Fail |
| M1-M10 | Fail | Fail | — | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 1.7766 |
| entry55-exit55-ema200-atr14x3 | 5 | 0 | 2.8878 |
| entry20-exit55-ema200-atr14x2 | 3 | 0 | -1.3516 |
| entry20-exit55-ema200-atr14x4 | 2 | 0 | -14.1116 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry55-exit55-ema200-atr14x3 | -78.4116 | -214.1521 | 135.7405 | Beat |
| M1-M7 | entry55-exit55-ema200-atr14x3 | -94.0596 | -26.2736 | -67.7860 | Underperform |
| M1-M8 | entry55-exit55-ema200-atr14x3 | -16.6949 | 59.6339 | -76.3287 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x3 | 5.5347 | 81.9595 | -76.4248 | Underperform |
| M1-M10 | entry20-exit55-ema200-atr14x3 | 21.3198 | 71.2561 | -49.9363 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x3 had 12 OOS trades, net expectancy -12.693306926396778732545052862, PF 0.0008515606585050699265049646, net PnL -152.31968311676134479054063435 vs benchmark -214.15206220779696825296326781.
- M1-M7: best top-3 stress run entry55-exit55-ema200-atr14x3 had 8 OOS trades, net expectancy -11.757453582108310553641651786, PF 0.001378286008504135220941586, net PnL -94.05962865686648442913321429 vs benchmark -26.273630364364324085065287810.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x2 had 22 OOS trades, net expectancy -5.0868084501022511784555841009, PF 0.4266584139083553616074721117, net PnL -111.90978590224952592602285022 vs benchmark 59.633880530438903809160943788.
- M1-M9: best top-3 stress run entry55-exit55-ema200-atr14x3 had 6 OOS trades, net expectancy 0.8920880106273495915776235657, PF 1.0843373478292509674811389937, net PnL 2.5189739693492917580647830078 vs benchmark 81.95947993852335854791486001.
- M1-M10: best top-3 stress run entry55-exit55-ema200-atr14x3 had 6 OOS trades, net expectancy 2.887787382602786027385274357, PF 1.3364968204362391634470394588, net PnL 17.326724295616716164311646142 vs benchmark 71.256066623342238910149950654.

Successful runs: none.

