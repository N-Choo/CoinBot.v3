# Donchian Trend-Following — BNB-USDT (4hour, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 7829044720718c6187af2a1742d2bb1d329b42e60571315b8322353743e22ddd |

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
| 1 | entry55-exit55-ema200-atr14x3 | 6 | 21.5853 | 5.1152 | 2.2536 | 16.5119 | 20.0506 | 4.4943 |
| 2 | entry55-exit55-ema200-atr14x4 | 5 | 22.0012 | 3.3689 | 4.0156 | 23.5699 | 20.4659 | 3.0114 |
| 3 | entry20-exit55-ema200-atr14x3 | 7 | 16.8735 | 2.6902 | 5.4615 | 19.3971 | 15.3459 | 2.4461 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 6 | 21.5853 | 5.1152 | 2.2536 | 16.5119 | 20.0506 | 4.4943 |
| 2 | entry55-exit55-ema200-atr14x4 | 5 | 22.0012 | 3.3689 | 4.0156 | 23.5699 | 20.4659 | 3.0114 |
| 3 | entry20-exit55-ema200-atr14x3 | 8 | 14.2954 | 2.5531 | 5.4615 | 16.9950 | 12.7717 | 2.2847 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 8 | 13.0763 | 2.8557 | 4.0628 | 13.3169 | 11.5544 | 2.4838 |
| 2 | entry20-exit55-ema200-atr14x3 | 10 | 9.3381 | 1.9869 | 7.2150 | 13.8243 | 7.8218 | 1.7558 |
| 3 | entry55-exit55-ema200-atr14x4 | 7 | 11.0938 | 1.9856 | 6.8130 | 17.7776 | 9.5748 | 1.7777 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 9 | 11.6597 | 2.8615 | 5.2071 | 11.8295 | 10.1400 | 2.4378 |
| 2 | entry55-exit55-ema200-atr14x4 | 8 | 8.9837 | 1.8498 | 8.4810 | 15.5398 | 7.4679 | 1.6392 |
| 3 | entry55-exit55-ema200-atr14x2 | 16 | 3.1997 | 1.7215 | 5.6892 | 4.1671 | 1.6927 | 1.3154 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 12 | 6.8276 | 2.0321 | 6.0325 | 9.1168 | 5.3151 | 1.7013 |
| 2 | entry20-exit55-ema200-atr14x3 | 17 | 3.3154 | 1.4115 | 10.3564 | 8.2093 | 1.8082 | 1.1990 |
| 3 | entry55-exit55-ema200-atr14x4 | 11 | 2.7959 | 1.2447 | 10.8687 | 11.5501 | 1.2894 | 1.1020 |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -168.3524 (-33.6705%), stress net PnL -169.6017 (-33.9203%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 9 | -7.4717 | 0.0940 | 6.7246 | -67.2457 | -168.3524 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 9 | -8.9628 | 0.0599 | 8.0665 | -80.6650 | -169.6017 | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 9 | -12.9284 | 0 | 11.6355 | -116.3552 | -168.3524 | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 9 | -14.4112 | 0 | 12.9701 | -129.7006 | -169.6017 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 13 | -5.5636 | 0.2810 | 8.7069 | -72.3274 | -168.3524 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 13 | -7.0575 | 0.2134 | 10.3463 | -91.7480 | -169.6017 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -49.4729 (-9.8946%), stress net PnL -50.9008 (-10.1802%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 10 | -6.3183 | 0.0995 | 6.3183 | -63.1827 | -49.4729 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | -7.8110 | 0.0617 | 7.8110 | -78.1103 | -50.9008 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 10 | -10.4334 | 0 | 10.4334 | -104.3339 | -49.4729 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | -11.9200 | 0 | 11.9200 | -119.1997 | -50.9008 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 13 | -4.7119 | 0.3157 | 7.2855 | -61.2548 | -49.4729 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 13 | -6.2071 | 0.2358 | 8.7768 | -80.6921 | -50.9008 | no | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 42.5939 (8.5188%), stress net PnL 41.0276 (8.2055%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 12 | -2.4428 | 0.6406 | 7.4884 | -29.3139 | 42.5939 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 12 | -3.9414 | 0.5080 | 9.0994 | -47.2969 | 41.0276 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 16 | -0.5454 | 0.9052 | 6.3731 | -8.7259 | 42.5939 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 16 | -2.0468 | 0.7051 | 8.6169 | -32.7488 | 41.0276 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 10 | -9.3126 | 0.0368 | 9.6687 | -40.6126 | 42.5939 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | -10.8009 | 0.0187 | 11.0062 | -57.0764 | 41.0276 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 117.7071 (23.5414%), stress net PnL 116.0280 (23.2056%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 12 | -1.5021 | 0.7826 | 6.2235 | -23.0153 | 117.7071 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 12 | -3.0021 | 0.6259 | 7.5662 | -42.5100 | 116.0280 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 10 | -1.7151 | 0.7692 | 7.4305 | -6.6491 | 117.7071 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | -3.2148 | 0.6205 | 8.4709 | -23.1639 | 116.0280 | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 15 | -0.8727 | 0.8650 | 5.3265 | -13.0898 | 117.7071 | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 15 | -2.3736 | 0.6900 | 6.8207 | -35.6039 | 116.0280 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 59.2575 (11.8515%), stress net PnL 57.6662 (11.5332%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 12 | -1.9206 | 0.7379 | 5.1902 | -23.0476 | 59.2575 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 12 | -3.4200 | 0.5950 | 6.2340 | -41.0400 | 57.6662 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 13 | 2.8701 | 1.5338 | 3.3219 | 37.3108 | 59.2575 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 13 | 1.3635 | 1.2128 | 4.0750 | 17.7254 | 57.6662 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 10 | -0.3195 | 0.9471 | 4.9785 | -3.1953 | 59.2575 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | -1.8213 | 0.7427 | 5.7222 | -18.2130 | 57.6662 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|
| M1-M6 | Fail | — | Fail | Fail |
| M1-M7 | Fail | — | Fail | Fail |
| M1-M8 | Fail | — | Fail | Fail |
| M1-M9 | — | Fail | Fail | Fail |
| M1-M10 | Fail | — | Fail | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x3 | 5 | 0 | -3.0021 |
| entry55-exit55-ema200-atr14x4 | 5 | 0 | -1.8213 |
| entry20-exit55-ema200-atr14x3 | 4 | 0 | 1.3635 |
| entry55-exit55-ema200-atr14x2 | 1 | 0 | -2.3736 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry55-exit55-ema200-atr14x3 | -80.6650 | -169.6017 | 88.9367 | Beat |
| M1-M7 | entry55-exit55-ema200-atr14x3 | -78.1103 | -50.9008 | -27.2095 | Underperform |
| M1-M8 | entry20-exit55-ema200-atr14x3 | -32.7488 | 41.0276 | -73.7764 | Underperform |
| M1-M9 | entry55-exit55-ema200-atr14x4 | -23.1639 | 116.0280 | -139.1919 | Underperform |
| M1-M10 | entry20-exit55-ema200-atr14x3 | 17.7254 | 57.6662 | -39.9408 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x3 had 13 OOS trades, net expectancy -7.0575397162752163090805460231, PF 0.2134061639876380603173979073, net PnL -91.74801631157781201804709830 vs benchmark -169.60171716023866074552792382.
- M1-M7: best top-3 stress run entry20-exit55-ema200-atr14x3 had 13 OOS trades, net expectancy -6.2070818623120679089922623838, PF 0.2357524992195862437593168214, net PnL -80.69206421005688281689941099 vs benchmark -50.900831976445921531015100483.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x3 had 16 OOS trades, net expectancy -2.0468009465739262837499701269, PF 0.7051272753709729879850818517, net PnL -32.748815145182820539999522030 vs benchmark 41.027623341120800674739970049.
- M1-M9: best top-3 stress run entry55-exit55-ema200-atr14x2 had 15 OOS trades, net expectancy -2.3735943122911900714770815331, PF 0.690036689605046450011088674, net PnL -35.603914684367851072156222997 vs benchmark 116.02800525315536585827845512.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x3 had 13 OOS trades, net expectancy 1.3634938842297792396189466582, PF 1.212752736500156838448916001, net PnL 17.725420494987130115046306557 vs benchmark 57.666192249915693307581470926.

Successful runs: none.

