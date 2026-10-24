# Donchian Trend-Following — XRP-USDT (4hour, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 10949 |
| SHA-256 | 994e622b26a33d2aa361a96fe8e66a0604b351ec061ab3c3fffbcac9e5ec9d0a |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M12 | 0..2189 | 2189 | M13-M18 | 2189..3284 | 1095 | 2021-10-24T00:00:00Z | 2022-10-23T16:00:00Z | 2022-10-23T20:00:00Z | 2023-04-24T04:00:00Z |
| M1-M18 | 0..3284 | 3284 | M19-M24 | 3284..4379 | 1095 | 2021-10-24T00:00:00Z | 2023-04-24T04:00:00Z | 2023-04-24T08:00:00Z | 2023-10-23T16:00:00Z |
| M1-M24 | 0..4379 | 4379 | M25-M30 | 4379..5474 | 1095 | 2021-10-24T00:00:00Z | 2023-10-23T16:00:00Z | 2023-10-23T20:00:00Z | 2024-04-23T04:00:00Z |
| M1-M30 | 0..5474 | 5474 | M31-M36 | 5474..6569 | 1095 | 2021-10-24T00:00:00Z | 2024-04-23T04:00:00Z | 2024-04-23T08:00:00Z | 2024-10-22T16:00:00Z |
| M1-M36 | 0..6569 | 6569 | M37-M42 | 6569..7664 | 1095 | 2021-10-24T00:00:00Z | 2024-10-22T16:00:00Z | 2024-10-22T20:00:00Z | 2025-04-23T04:00:00Z |
| M1-M42 | 0..7664 | 7664 | M43-M48 | 7664..8759 | 1095 | 2021-10-24T00:00:00Z | 2025-04-23T04:00:00Z | 2025-04-23T08:00:00Z | 2025-10-22T16:00:00Z |
| M1-M48 | 0..8759 | 8759 | M49-M54 | 8759..9854 | 1095 | 2021-10-24T00:00:00Z | 2025-10-22T16:00:00Z | 2025-10-22T20:00:00Z | 2026-04-23T04:00:00Z |
| M1-M54 | 0..9854 | 9854 | M55-M60 | 9854..10949 | 1095 | 2021-10-24T00:00:00Z | 2026-04-23T04:00:00Z | 2026-04-23T08:00:00Z | 2026-10-22T16:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy | donchian-v1 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT, BNB-USDT, XRP-USDT (5) |
| Entry | Close > prior Donchian high (N ∈ [20, 55]) and close > EMA(200) |
| Exit | ATR(14) trailing stop (highest high since entry − multiplier × ATR) or close < EMA(200) at the next open |
| Grid | entry N [20, 55] × Donchian exit N 55 × ATR multiplier [2, 3, 4] × EMA 200 × ATR 14 = 6 configs (Donchian exit N is fixed metadata and is not an active exit in this variant) |
| Walk-forward | 5 years (60 months); expanding training windows M1-M12 through M1-M54, each tested on the next 6 months (OOS M13-M18 through M55-M60) |
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
- A run requires at least 10 OOS trades.
- A run passes only when stress OOS net expectancy > 0, stress OOS PF > 1, stress OOS net PnL > the stress buy-and-hold benchmark, stress OOS trades >= 10, and the parameter-stability rule holds.
- Parameter stability requires every available grid neighbor to have positive base-cost train net expectancy. Neighbors share the same exit, EMA, and ATR period and sit within one grid step on both entry N and ATR multiplier; each selected config needs at least one neighbor.
- A symbol passes when any top-3 run in any window passes. The experiment passes only when all 5 symbols pass.
- The benchmark buys at the first OOS open and sells at the last OOS close with the same half-budget and cost scenario.

## 3. Validation

Top 3 configs per expanding training window, selected on base-cost training metrics. Stability is checked against every available grid neighbor.

### M1-M12 training (M13-M18)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 13 | 3.5934 | 1.4935 | 7.5061 | 8.4550 | 2.0858 | 1.2619 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 12 | -0.1098 | 0.9899 | 11.4486 | 9.0061 | -1.6119 | 0.8657 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 19 | -1.3064 | 0.8660 | 15.5480 | 5.8622 | -2.8067 | 0.7391 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 23 | 3.2306 | 1.4667 | 10.4554 | 5.5271 | 1.7235 | 1.2240 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 20 | -0.5746 | 0.9437 | 11.4486 | 5.9249 | -2.0760 | 0.8154 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 39 | -2.2731 | 0.7340 | 21.5896 | 3.4940 | -3.7719 | 0.6060 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 32 | 6.3327 | 1.8090 | 10.4554 | 7.3606 | 4.8209 | 1.5536 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 27 | 4.6477 | 1.4275 | 11.4486 | 8.5768 | 3.1385 | 1.2635 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 54 | -0.5556 | 0.9404 | 23.7559 | 4.5562 | -2.0570 | 0.8026 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 45 | 2.9852 | 1.3591 | 10.4554 | 5.4366 | 1.4784 | 1.1605 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 37 | 2.2194 | 1.1887 | 10.8178 | 6.7515 | 0.7138 | 1.0559 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 72 | -1.0238 | 0.8820 | 23.7559 | 3.5205 | -2.5245 | 0.7405 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 56 | 2.1406 | 1.2601 | 10.4391 | 4.4895 | 0.6352 | 1.0695 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 48 | -0.7918 | 0.9351 | 20.8406 | 5.3596 | -2.2929 | 0.8268 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 88 | -0.7331 | 0.9120 | 23.7559 | 2.9867 | -2.2343 | 0.7613 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 74 | 4.8890 | 1.5431 | 11.3793 | 5.1085 | 3.3794 | 1.3404 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 83 | 3.8624 | 1.2995 | 34.5350 | 7.7678 | 2.3543 | 1.1678 | yes |
| 3 | entry20-exit55-ema200-atr14x2 | 109 | 3.1758 | 1.3638 | 23.7559 | 3.8857 | 1.6688 | 1.1716 | yes |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 88 | 4.2248 | 1.4635 | 18.1849 | 4.4763 | 2.7162 | 1.2700 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 72 | 3.7966 | 1.2957 | 21.1249 | 5.6155 | 2.2886 | 1.1654 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 65 | 4.2616 | 1.2626 | 18.3291 | 9.8405 | 2.7529 | 1.1590 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 91 | 5.4425 | 1.6174 | 18.1849 | 4.4127 | 3.9321 | 1.4042 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 75 | 4.8745 | 1.3955 | 21.1249 | 5.4542 | 3.3649 | 1.2533 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 68 | 4.9781 | 1.3209 | 18.3291 | 9.4296 | 3.4683 | 1.2096 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL -6.4345 (-1.2869%), stress net PnL -7.9271 (-1.5854%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 10 | 2.7589 | 1.4275 | 5.4320 | 27.5887 | -6.4345 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 10 | 1.2525 | 1.1705 | 6.6256 | 12.5247 | -7.9271 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -1.2718 | 0.8615 | 5.0186 | -10.1747 | -6.4345 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -2.7722 | 0.7258 | 5.5982 | -22.1774 | -7.9271 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 14 | -3.7841 | 0.6167 | 7.9404 | -52.9774 | -6.4345 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 14 | -5.2807 | 0.5196 | 9.4307 | -73.9294 | -7.9271 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 79.6498 (15.9300%), stress net PnL 78.0279 (15.6056%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 9 | 14.2603 | 2.4062 | 3.4464 | 128.3425 | 79.6498 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 9 | 12.7366 | 2.1277 | 4.1923 | 114.6294 | 78.0279 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 7 | 19.5686 | 2.5328 | 3.5265 | 150.7418 | 79.6498 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 7 | 18.0370 | 2.3051 | 3.6860 | 138.4973 | 78.0279 | yes | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 15 | 3.9098 | 1.3445 | 7.1960 | 58.6473 | 79.6498 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 15 | 2.4017 | 1.1901 | 7.9676 | 36.0254 | 78.0279 | no | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 10.7291 (2.1458%), stress net PnL 9.2107 (1.8421%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 13 | -5.2548 | 0.4475 | 8.2365 | -68.3124 | 10.7291 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 13 | -6.7492 | 0.3541 | 9.6360 | -87.7391 | 9.2107 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | -6.9991 | 0.5120 | 12.3683 | -69.9906 | 10.7291 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | -8.4908 | 0.4476 | 13.5216 | -84.9080 | 9.2107 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 18 | -2.4284 | 0.6394 | 5.8160 | -43.7111 | 10.7291 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 18 | -3.9270 | 0.4860 | 7.9257 | -70.6860 | 9.2107 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -15.0814 (-3.0163%), stress net PnL -16.5610 (-3.3122%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 10 | -0.8118 | 0.8991 | 6.4602 | -8.1178 | -15.0814 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 10 | -2.3128 | 0.7411 | 7.2123 | -23.1281 | -16.5610 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | -10.7977 | 0.2173 | 12.1934 | -107.9773 | -15.0814 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | -12.2838 | 0.1799 | 13.1240 | -122.8376 | -16.5610 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 15 | 1.0358 | 1.1521 | 5.7675 | 15.5371 | -15.0814 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 15 | -0.4680 | 0.9392 | 6.4005 | -7.0200 | -16.5610 | yes | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL 1608.3418 (321.6684%), stress net PnL 1604.4234 (320.8847%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 17 | 14.5206 | 2.2322 | 12.2992 | 255.0493 | 1608.3418 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 17 | 12.9965 | 2.0203 | 13.5349 | 227.6256 | 1604.4234 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 13 | 18.9413 | 2.1523 | 10.0732 | 254.4358 | 1608.3418 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 13 | 17.4106 | 2.0042 | 10.9895 | 233.0221 | 1604.4234 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 20 | 20.7808 | 2.9474 | 7.4299 | 437.7012 | 1608.3418 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 20 | 19.2473 | 2.6764 | 8.8508 | 405.4964 | 1604.4234 | no | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 24.9483 (4.9897%), stress net PnL 23.4085 (4.6817%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 13 | 0.8412 | 1.0810 | 10.3643 | 10.9350 | 24.9483 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 13 | -0.6624 | 0.9419 | 11.4003 | -8.6108 | 23.4085 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 16 | -3.9482 | 0.7221 | 10.0418 | -63.1712 | 24.9483 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 16 | -5.4445 | 0.6444 | 10.9491 | -87.1124 | 23.4085 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 20 | 0.1291 | 1.0141 | 12.0627 | 2.5822 | 24.9483 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 20 | -1.3733 | 0.8641 | 13.4076 | -27.4667 | 23.4085 | no | yes | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -202.3455 (-40.4691%), stress net PnL -203.5438 (-40.7088%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 3 | 41.1598 | ∞ | 0 | 123.4794 | -202.3455 | yes | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 3 | 39.5957 | ∞ | 0 | 118.7871 | -203.5438 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 3 | 30.7447 | ∞ | 0 | 92.2342 | -202.3455 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 3 | 29.1963 | ∞ | 0 | 87.5889 | -203.5438 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 3 | 20.5018 | ∞ | 0 | 61.5055 | -202.3455 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 3 | 18.9688 | ∞ | 0 | 56.9064 | -203.5438 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL -42.0193 (-8.4039%), stress net PnL -43.4585 (-8.6917%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 11 | 0.1372 | 1.0179 | 6.8918 | 1.5088 | -42.0193 | yes | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 11 | -1.3653 | 0.8415 | 7.7828 | -15.0183 | -43.4585 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | -3.2198 | 0.7535 | 10.4163 | -32.1980 | -42.0193 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | -4.7172 | 0.6688 | 11.3020 | -47.1722 | -43.4585 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 8 | 5.7996 | 1.3480 | 8.1630 | 46.3966 | -42.0193 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | 4.2886 | 1.2388 | 8.9019 | 34.3089 | -43.4585 | yes | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | — | Fail (stability) | — | Fail (stability) | Fail (stability) | — |
| M1-M18 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M24 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M30 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M36 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M42 | Fail | — | Fail | Fail | — | — |
| M1-M48 | — | — | — | Fail | Fail | Fail |
| M1-M54 | — | — | — | Fail | Fail | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x2 | 8 | 0 | 39.5957 |
| entry55-exit55-ema200-atr14x3 | 7 | 0 | 29.1963 |
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 19.2473 |
| entry55-exit55-ema200-atr14x4 | 2 | 0 | 18.9688 |
| entry20-exit55-ema200-atr14x3 | 1 | 0 | -5.2807 |
| entry20-exit55-ema200-atr14x4 | 1 | 0 | -5.4445 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry55-exit55-ema200-atr14x2 | 12.5247 | -7.9271 | 20.4518 | Beat |
| M1-M18 | entry55-exit55-ema200-atr14x3 | 138.4973 | 78.0279 | 60.4694 | Beat |
| M1-M24 | entry20-exit55-ema200-atr14x2 | -70.6860 | 9.2107 | -79.8967 | Underperform |
| M1-M30 | entry20-exit55-ema200-atr14x2 | -7.0200 | -16.5610 | 9.5410 | Beat |
| M1-M36 | entry20-exit55-ema200-atr14x2 | 405.4964 | 1604.4234 | -1198.9270 | Underperform |
| M1-M42 | entry55-exit55-ema200-atr14x2 | -8.6108 | 23.4085 | -32.0193 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x2 | 118.7871 | -203.5438 | 322.3309 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 34.3089 | -43.4585 | 77.7674 | Beat |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry55-exit55-ema200-atr14x2 had 10 OOS trades (minimum 10), net expectancy 1.2524705453633769114676145326, PF 1.1705077265787494366131171587, net PnL 12.524705453633769114676145326 vs benchmark -7.927053565673277325369029462; parameter stability fail.
- M1-M18: best top-3 stress run entry55-exit55-ema200-atr14x3 had 7 OOS trades (minimum 10), net expectancy 18.036958291560541985613765127, PF 2.3050906866784868574470748791, net PnL 138.49728559810814972161604960 vs benchmark 78.027886284722222222222222223; parameter stability fail.
- M1-M24: best top-3 stress run entry20-exit55-ema200-atr14x2 had 18 OOS trades (minimum 10), net expectancy -3.9269997615522948272796918299, PF 0.4859800620307873565577113622, net PnL -70.685995707941306891034452938 vs benchmark 9.210746424704081251633620853; parameter stability fail.
- M1-M30: best top-3 stress run entry20-exit55-ema200-atr14x2 had 15 OOS trades (minimum 10), net expectancy -0.4680020497701581822128795849, PF 0.9392334285571394851277596659, net PnL -7.020030746552372733193193773 vs benchmark -16.561044447276898705366084596; parameter stability fail.
- M1-M36: best top-3 stress run entry20-exit55-ema200-atr14x2 had 20 OOS trades (minimum 10), net expectancy 19.247348282109541144585034309, PF 2.6764480083122563659987724752, net PnL 405.49638006538374771329000288 vs benchmark 1604.4233972859148338792700046; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x2 had 13 OOS trades (minimum 10), net expectancy -0.6623656885995288324998770479, PF 0.9419279143915633208753325382, net PnL -8.610753951793874822498401623 vs benchmark 23.408548121956514996433113114; parameter stability pass.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x2 had 3 OOS trades (minimum 10), net expectancy 39.595700834377388731357529127, PF infinite, net PnL 118.78710250313216619407258738 vs benchmark -203.54380750444890386581348681; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy 4.2886137425737525734497713509, PF 1.2388247234066872539134325372, net PnL 34.308909940590020587598170807 vs benchmark -43.458459134767645630555496907; parameter stability pass.

Successful runs: none.

