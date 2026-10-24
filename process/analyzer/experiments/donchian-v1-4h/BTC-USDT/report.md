# Donchian Trend-Following — BTC-USDT (4hour, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 1ee09051cb265eafd9e32a673fbb283dc0c345e4bec0130ad508df29c2c6ecfb |

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
| 1 | entry20-exit55-ema200-atr14x4 | 7 | 2.5705 | 2.3595 | 0.8277 | 3.2598 | 1.0644 | 1.3876 |
| 2 | entry55-exit55-ema200-atr14x4 | 5 | 2.2350 | 1.8469 | 1.2882 | 4.7802 | 0.7294 | 1.2063 |
| 3 | entry55-exit55-ema200-atr14x2 | 8 | 0.7651 | 1.2232 | 1.3862 | 3.5348 | -0.7383 | 0.8293 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x4 | 7 | 2.5705 | 2.3595 | 0.8277 | 3.2598 | 1.0644 | 1.3876 |
| 2 | entry55-exit55-ema200-atr14x4 | 5 | 2.2350 | 1.8469 | 1.2882 | 4.7802 | 0.7294 | 1.2063 |
| 3 | entry55-exit55-ema200-atr14x2 | 8 | 0.7651 | 1.2232 | 1.3862 | 3.5348 | -0.7383 | 0.8293 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x4 | 10 | -1.1176 | 0.7364 | 3.0161 | 3.1185 | -2.6182 | 0.5047 |
| 2 | entry55-exit55-ema200-atr14x2 | 10 | -2.1422 | 0.6102 | 3.1881 | 3.4814 | -3.6413 | 0.4407 |
| 3 | entry20-exit55-ema200-atr14x2 | 15 | -2.5665 | 0.5127 | 4.5666 | 2.3996 | -4.0649 | 0.3507 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x4 | 13 | -2.0979 | 0.5481 | 4.7760 | 2.5357 | -3.5970 | 0.3662 |
| 2 | entry55-exit55-ema200-atr14x3 | 12 | -3.9043 | 0.4242 | 6.1779 | 3.2817 | -5.4007 | 0.3074 |
| 3 | entry20-exit55-ema200-atr14x3 | 16 | -3.6983 | 0.4049 | 6.5810 | 2.5741 | -5.1950 | 0.2842 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x4 | 16 | -2.6774 | 0.5105 | 6.1215 | 2.4907 | -4.1756 | 0.3571 |
| 2 | entry20-exit55-ema200-atr14x2 | 25 | -4.0654 | 0.2994 | 10.8332 | 1.6020 | -5.5615 | 0.1981 |
| 3 | entry55-exit55-ema200-atr14x3 | 15 | -5.3089 | 0.3024 | 8.3027 | 2.7393 | -6.8032 | 0.2199 |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -144.6447 (-28.9289%), stress net PnL -145.9297 (-29.1859%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 9 | -5.0415 | 0.2287 | 4.5373 | -45.3734 | -144.6447 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 9 | -6.5362 | 0.1506 | 5.8825 | -58.8255 | -145.9297 | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 7 | -8.9030 | 0.0288 | 6.2321 | -62.3209 | -144.6447 | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 7 | -10.3919 | 0.0047 | 7.2743 | -72.7431 | -145.9297 | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 10 | -7.7582 | 0 | 7.7582 | -77.5825 | -144.6447 | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 10 | -9.2488 | 0 | 9.2488 | -92.4885 | -145.9297 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -28.1507 (-5.6301%), stress net PnL -29.6106 (-5.9221%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 13 | -8.4223 | 0.1094 | 11.6019 | -116.0195 | -28.1507 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 13 | -9.9119 | 0.0749 | 13.6877 | -136.8768 | -29.6106 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 9 | -9.6306 | 0.0209 | 9.7537 | -97.5374 | -28.1507 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 9 | -11.1184 | 0.0034 | 11.2413 | -112.4134 | -29.6106 | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 13 | -7.9102 | 0 | 10.2832 | -102.8325 | -28.1507 | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 13 | -9.4006 | 0 | 12.2207 | -122.2073 | -29.6106 | no | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 41.9069 (8.3814%), stress net PnL 40.3417 (8.0683%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 12 | -6.3392 | 0.1503 | 7.6070 | 22.5222 | 41.9069 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | -7.8319 | 0.0999 | 9.3983 | 2.9590 | 40.3417 | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 12 | -2.1138 | 0.6968 | 8.3651 | -25.3662 | 41.9069 | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 12 | -3.6129 | 0.5667 | 10.0050 | -43.3551 | 40.3417 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 18 | -1.6630 | 0.6716 | 8.8219 | -29.9344 | 41.9069 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 18 | -3.1628 | 0.5052 | 11.3625 | -56.9300 | 40.3417 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 47.6004 (9.5201%), stress net PnL 46.0266 (9.2053%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 10 | 1.9751 | 1.2171 | 6.7170 | 46.8146 | 47.6004 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | 0.4699 | 1.0457 | 7.9087 | 30.2195 | 46.0266 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -3.0559 | 0.6816 | 4.4799 | 2.6170 | 47.6004 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -4.5535 | 0.5822 | 5.2243 | -10.9072 | 46.0266 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 12 | -0.8149 | 0.9017 | 6.9931 | 17.2852 | 47.6004 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 12 | -2.3159 | 0.7602 | 8.3346 | -2.2701 | 46.0266 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 34.6765 (6.9353%), stress net PnL 33.1221 (6.6244%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 11 | 2.4477 | 1.3062 | 3.9393 | 26.9245 | 34.6765 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | 0.9417 | 1.1022 | 4.6845 | 10.3592 | 33.1221 | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 14 | -0.4111 | 0.9116 | 3.8976 | -5.7560 | 34.6765 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 14 | -1.9128 | 0.6792 | 4.9588 | -26.7789 | 33.1221 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | 0.2894 | 1.0322 | 3.1145 | 2.3150 | 34.6765 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -1.2133 | 0.8798 | 3.4226 | -9.7065 | 33.1221 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M6 | — | — | Fail | Fail | — | Fail |
| M1-M7 | — | — | Fail | Fail | — | Fail |
| M1-M8 | Fail | — | Fail | Fail | — | — |
| M1-M9 | — | Fail | Fail | — | Fail | — |
| M1-M10 | Fail | — | Fail | — | Fail | — |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 0.9417 |
| entry55-exit55-ema200-atr14x2 | 3 | 0 | -3.6129 |
| entry20-exit55-ema200-atr14x2 | 2 | 0 | -1.9128 |
| entry55-exit55-ema200-atr14x3 | 2 | 0 | -1.2133 |
| entry55-exit55-ema200-atr14x4 | 2 | 0 | -10.3919 |
| entry20-exit55-ema200-atr14x3 | 1 | 0 | -2.3159 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | -58.8255 | -145.9297 | 87.1042 | Beat |
| M1-M7 | entry55-exit55-ema200-atr14x4 | -112.4134 | -29.6106 | -82.8028 | Underperform |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 2.9590 | 40.3417 | -37.3826 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x4 | 30.2195 | 46.0266 | -15.8072 | Underperform |
| M1-M10 | entry20-exit55-ema200-atr14x4 | 10.3592 | 33.1221 | -22.7629 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x4 had 9 OOS trades, net expectancy -6.5361657898259079825542326537, PF 0.1505807193833047505438343182, net PnL -58.825492108433171842988093883 vs benchmark -145.92965502192157220758804813.
- M1-M7: best top-3 stress run entry55-exit55-ema200-atr14x2 had 13 OOS trades, net expectancy -9.400559355438509040287929033, PF 0, net PnL -122.20727162070061752374307743 vs benchmark -29.610633140898298983121003790.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x2 had 18 OOS trades, net expectancy -3.1627758612991210045148541506, PF 0.5051835250252803040716759815, net PnL -56.929965503384178081267374711 vs benchmark 40.341651270044991044102506246.
- M1-M9: best top-3 stress run entry20-exit55-ema200-atr14x4 had 10 OOS trades, net expectancy 0.4698521689921358370427171555, PF 1.0456861516536465705082706127, net PnL 30.219472403743390966895970029 vs benchmark 46.026644426511603363308634919.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x4 had 11 OOS trades, net expectancy 0.941747138863817816842666256, PF 1.1022364662831879538592536345, net PnL 10.359218527501995985269328816 vs benchmark 33.122123551412200379534609507.

Successful runs: none.

