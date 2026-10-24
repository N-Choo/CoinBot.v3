# Donchian Trend-Following — BNB-USDT (4hour, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 10949 |
| SHA-256 | e17d22372e025ec109e9c3d1fd0cf6bae18b884d8fe0a48c006c3f77ceb05846 |

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
| 1 | entry55-exit55-ema200-atr14x4 | 12 | -2.7185 | 0.7575 | 5.9075 | 7.6863 | -4.2167 | 0.6577 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 19 | -4.5163 | 0.3972 | 8.5810 | 2.6886 | -6.0118 | 0.3126 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 18 | -6.8692 | 0.4903 | 14.6645 | 6.3060 | -8.3611 | 0.4316 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 20 | 3.9161 | 1.3991 | 5.9075 | 7.4703 | 2.4080 | 1.2235 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 28 | -0.9220 | 0.9215 | 14.6645 | 6.0242 | -2.4229 | 0.8106 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 26 | -0.7148 | 0.9185 | 9.9223 | 4.1910 | -2.2160 | 0.7711 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 24 | 1.8309 | 1.1905 | 6.0783 | 6.2756 | 0.3259 | 1.0306 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 30 | -1.5854 | 0.8149 | 9.9223 | 3.6513 | -3.0853 | 0.6770 | no |
| 3 | entry55-exit55-ema200-atr14x2 | 40 | -1.8302 | 0.7083 | 8.5810 | 1.9851 | -3.3297 | 0.5473 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 34 | 8.7860 | 1.9037 | 10.2971 | 7.3052 | 7.2705 | 1.6802 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 43 | 4.4060 | 1.5914 | 9.9223 | 4.7475 | 2.8972 | 1.3471 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 54 | 2.5549 | 1.2387 | 20.5189 | 5.2125 | 1.0488 | 1.0889 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 43 | 7.4057 | 1.8433 | 10.2971 | 5.8617 | 5.8923 | 1.6062 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 54 | 3.9743 | 1.5879 | 9.9223 | 3.8885 | 2.4661 | 1.3216 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 65 | 2.9967 | 1.3207 | 20.5189 | 4.3689 | 1.4899 | 1.1437 | yes |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 52 | 3.5499 | 1.3574 | 11.1779 | 5.0262 | 2.0423 | 1.1869 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 63 | 2.4193 | 1.3230 | 9.9223 | 3.4494 | 0.9134 | 1.1083 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 79 | 0.8507 | 1.0884 | 20.5189 | 3.6802 | -0.6529 | 0.9389 | no |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 63 | 5.5230 | 1.6045 | 12.6414 | 4.5344 | 4.0125 | 1.3982 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 79 | 3.2137 | 1.4419 | 9.7909 | 3.0922 | 1.7067 | 1.2073 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 94 | 2.3639 | 1.2486 | 20.5189 | 3.5410 | 0.8581 | 1.0814 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 69 | 3.8170 | 1.3990 | 12.6414 | 4.2099 | 2.3090 | 1.2188 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 85 | 2.4404 | 1.3288 | 9.7909 | 2.9073 | 0.9345 | 1.1113 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 102 | 1.3721 | 1.1434 | 20.5189 | 3.2865 | -0.1322 | 0.9876 | no |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 99.4941 (19.8988%), stress net PnL 97.8424 (19.5685%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | 13.8681 | 2.7971 | 4.7147 | 110.9447 | 99.4941 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | 12.3450 | 2.4598 | 4.9912 | 98.7600 | 97.8424 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 15 | 3.4316 | 1.7375 | 2.7017 | 51.4740 | 99.4941 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 15 | 1.9242 | 1.3598 | 3.1541 | 28.8629 | 97.8424 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 10 | 9.7829 | 2.1364 | 5.7896 | 97.8287 | 99.4941 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | 8.2659 | 1.8844 | 6.2126 | 82.6592 | 97.8424 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL -163.9093 (-32.7819%), stress net PnL -165.1653 (-33.0331%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 4 | -8.5950 | 0 | 3.5995 | -35.9947 | -163.9093 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 4 | -10.0844 | 0 | 4.3452 | -43.4519 | -165.1653 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | -9.6780 | 0 | 10.6458 | -97.9430 | -163.9093 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | -11.1657 | 0 | 12.2823 | -115.8229 | -165.1653 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | -7.2441 | 0 | 3.0591 | -30.5911 | -163.9093 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | -8.7355 | 0 | 3.8056 | -38.0564 | -165.1653 | yes | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 848.6509 (169.7302%), stress net PnL 845.8738 (169.1748%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 10 | 24.0773 | 3.1349 | 6.4825 | 237.0460 | 848.6509 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | 22.5389 | 2.8531 | 6.9860 | 220.1651 | 845.8738 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 13 | 17.1330 | 4.3543 | 3.9126 | 219.0019 | 848.6509 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 13 | 15.6050 | 3.6409 | 4.3637 | 197.6413 | 845.8738 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 20 | 4.0349 | 1.9870 | 3.2762 | 76.9704 | 848.6509 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 20 | 2.5266 | 1.5244 | 3.9010 | 45.3075 | 845.8738 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -6.4327 (-1.2865%), stress net PnL -7.9253 (-1.5851%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | 4.3756 | 2.1014 | 1.1855 | 35.0052 | -6.4327 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | 2.8668 | 1.6076 | 1.5575 | 22.9346 | -7.9253 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | 3.5956 | 2.0616 | 3.2278 | 35.9558 | -6.4327 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | 2.0879 | 1.4875 | 3.9601 | 20.8792 | -7.9253 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 10 | 3.6343 | 2.0399 | 1.1335 | 36.3433 | -6.4327 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | 2.1266 | 1.5015 | 1.3713 | 21.2661 | -7.9253 | yes | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL 18.1285 (3.6257%), stress net PnL 16.5990 (3.3198%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 9 | -14.8723 | 0.0358 | 13.3851 | -122.4084 | 18.1285 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 9 | -16.3522 | 0.0209 | 14.7170 | -137.2471 | 16.5990 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 9 | -6.9109 | 0.4174 | 6.2198 | -50.7557 | 18.1285 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 9 | -8.4027 | 0.3543 | 7.5625 | -65.7020 | 16.5990 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | -9.1130 | 0.1656 | 13.4400 | -116.1395 | 18.1285 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | -10.6015 | 0.1288 | 15.2541 | -138.4988 | 16.5990 | no | yes | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 363.9700 (72.7940%), stress net PnL 361.9209 (72.3842%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 10 | 17.0635 | 4.2908 | 4.1411 | 170.6346 | 363.9700 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | 15.5356 | 3.6886 | 4.5791 | 155.3558 | 361.9209 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 15 | 6.9092 | 2.0308 | 7.5618 | 103.6373 | 363.9700 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 15 | 5.3965 | 1.7014 | 9.1683 | 80.9478 | 361.9209 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 15 | 10.0266 | 2.1501 | 6.0397 | 150.3995 | 363.9700 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 15 | 8.5093 | 1.8858 | 6.2627 | 127.6397 | 361.9209 | no | no | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -204.3249 (-40.8650%), stress net PnL -205.5202 (-41.1040%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 6 | -14.0958 | 0 | 9.6648 | -96.6480 | -204.3249 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 6 | -15.5769 | 0 | 10.7019 | -107.0185 | -205.5202 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 6 | -7.7418 | 0.1760 | 5.8524 | -58.5237 | -204.3249 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | -9.2324 | 0.1273 | 6.8952 | -68.9516 | -205.5202 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 8 | -10.2814 | 0 | 8.8950 | -88.9502 | -204.3249 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 8 | -11.7682 | 0 | 10.2337 | -102.3369 | -205.5202 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 74.0165 (14.8033%), stress net PnL 72.4030 (14.4806%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 12 | -2.3096 | 0.6734 | 7.4305 | -27.7150 | 74.0165 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 12 | -3.8084 | 0.5349 | 8.4709 | -45.7004 | 72.4030 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 14 | -2.3843 | 0.6603 | 6.2235 | -33.3807 | 74.0165 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 14 | -3.8830 | 0.5258 | 7.5662 | -54.3621 | 72.4030 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | 2.3545 | 1.4260 | 5.3407 | 32.9634 | 74.0165 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | 0.8487 | 1.1331 | 6.3888 | 11.8823 | 72.4030 | no | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | — | Fail (stability) |
| M1-M18 | Fail (stability) | — | Fail (stability) | Fail (stability) |
| M1-M24 | — | Fail (stability) | Fail (stability) | Fail (stability) |
| M1-M30 | Fail (stability) | — | Fail (stability) | Fail (stability) |
| M1-M36 | Fail | — | Fail (stability) | Fail |
| M1-M42 | Fail (stability) | — | Fail (stability) | Fail (stability) |
| M1-M48 | Fail | — | Fail (stability) | Fail |
| M1-M54 | Fail (stability) | — | Fail (stability) | Fail (stability) |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x4 | 8 | 0 | 22.5389 |
| entry20-exit55-ema200-atr14x4 | 7 | 0 | 8.5093 |
| entry55-exit55-ema200-atr14x3 | 7 | 0 | 15.6050 |
| entry55-exit55-ema200-atr14x2 | 2 | 0 | 2.5266 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry55-exit55-ema200-atr14x4 | 98.7600 | 97.8424 | 0.9177 | Beat |
| M1-M18 | entry55-exit55-ema200-atr14x3 | -38.0564 | -165.1653 | 127.1089 | Beat |
| M1-M24 | entry55-exit55-ema200-atr14x4 | 220.1651 | 845.8738 | -625.7087 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x4 | 22.9346 | -7.9253 | 30.8599 | Beat |
| M1-M36 | entry55-exit55-ema200-atr14x3 | -65.7020 | 16.5990 | -82.3009 | Underperform |
| M1-M42 | entry55-exit55-ema200-atr14x4 | 155.3558 | 361.9209 | -206.5652 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x3 | -68.9516 | -205.5202 | 136.5687 | Beat |
| M1-M54 | entry20-exit55-ema200-atr14x4 | 11.8823 | 72.4030 | -60.5207 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy 12.345000429074296663195857922, PF 2.4598071858045301320983924966, net PnL 98.76000343259437330556686338 vs benchmark 97.84235336215458316293743063; parameter stability fail.
- M1-M18: best top-3 stress run entry55-exit55-ema200-atr14x3 had 4 OOS trades (minimum 10), net expectancy -8.735506529166943729680171671, PF 0, net PnL -38.056438159330121347774284607 vs benchmark -165.16533191202463601742014249; parameter stability fail.
- M1-M24: best top-3 stress run entry55-exit55-ema200-atr14x4 had 10 OOS trades (minimum 10), net expectancy 22.538921274189902066220181399, PF 2.8531297722124021262788395931, net PnL 220.16509462085330023722867323 vs benchmark 845.8737728162769237694402557; parameter stability fail.
- M1-M30: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy 2.8668212772007929188066212112, PF 1.6076334412315211754497789512, net PnL 22.934570217606343350452969690 vs benchmark -7.925303626532287744529060385; parameter stability fail.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x3 had 9 OOS trades (minimum 10), net expectancy -8.402740735799679262633550028, PF 0.3543073120295349424211545645, net PnL -65.701950571994466417685237040 vs benchmark 16.598967567003858587965376994; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x4 had 10 OOS trades (minimum 10), net expectancy 15.535577832958233964570731551, PF 3.6886208223519829986255581155, net PnL 155.35577832958233964570731551 vs benchmark 361.92094406499854335933706669; parameter stability fail.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x3 had 6 OOS trades (minimum 10), net expectancy -9.232416492462534762377399774, PF 0.1272758284887080362847673738, net PnL -68.951553187530535720836415259 vs benchmark -205.52022507503747201279842113; parameter stability fail.
- M1-M54: best top-3 stress run entry20-exit55-ema200-atr14x4 had 14 OOS trades (minimum 10), net expectancy 0.8487365507805254505983812946, PF 1.1331027100513846910371004221, net PnL 11.882311710927356308377338124 vs benchmark 72.403038412404739879300471349; parameter stability fail.

Successful runs: none.

