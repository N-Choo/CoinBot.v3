# Donchian Trend-Following — ETH-USDT (4hour, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | ETH-USDT |
| Timeframe | 4hour |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 10949 |
| SHA-256 | 1b65834360489ce07f8fd10bd95beec6566fcbb3feb5e1ff500176ae3c0e31af |

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
| 1 | entry55-exit55-ema200-atr14x2 | 15 | 0.8974 | 1.0968 | 6.5707 | 6.4492 | -0.6062 | 0.9403 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 12 | -0.1796 | 0.9844 | 7.3328 | 8.6177 | -1.6816 | 0.8638 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 25 | -3.1374 | 0.6929 | 11.1212 | 4.2738 | -4.6349 | 0.5892 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 20 | 3.3593 | 1.2709 | 10.3733 | 9.3044 | 1.8520 | 1.1386 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 31 | 1.9054 | 1.1687 | 15.4858 | 6.4493 | 0.4003 | 1.0324 | no |
| 3 | entry55-exit55-ema200-atr14x2 | 32 | 0.6677 | 1.0922 | 6.5707 | 3.4889 | -0.8356 | 0.8977 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 40 | 0.1564 | 1.0229 | 6.5707 | 2.8646 | -1.3461 | 0.8273 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 40 | 0.1879 | 1.0180 | 15.4858 | 5.0590 | -1.3147 | 0.8856 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 60 | -0.5237 | 0.9284 | 12.7740 | 2.2976 | -2.0251 | 0.7559 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 35 | 7.1326 | 1.5729 | 11.0289 | 9.0852 | 5.6197 | 1.4188 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 54 | 5.4827 | 1.5220 | 15.4858 | 6.4590 | 3.9722 | 1.3441 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 46 | 1.4240 | 1.1526 | 12.7595 | 3.6801 | -0.0803 | 0.9921 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 43 | 5.7234 | 1.5008 | 11.0289 | 7.4631 | 4.2126 | 1.3410 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 66 | 3.8507 | 1.3825 | 15.4858 | 5.3482 | 2.3427 | 1.2111 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 55 | 1.0433 | 1.1176 | 12.7595 | 3.1849 | -0.4605 | 0.9530 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 48 | 5.8099 | 1.5123 | 11.0289 | 6.8431 | 4.2989 | 1.3505 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 75 | 4.8776 | 1.5038 | 15.4858 | 4.9829 | 3.3680 | 1.3139 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 63 | 0.2115 | 1.0226 | 12.7595 | 3.0025 | -1.2911 | 0.8745 | no |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 59 | 8.8016 | 1.7594 | 11.0289 | 6.8076 | 7.2861 | 1.5827 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 92 | 6.5948 | 1.6505 | 15.4858 | 4.8994 | 5.0826 | 1.4551 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 75 | 4.0220 | 1.4418 | 12.7595 | 3.8821 | 2.5137 | 1.2515 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 67 | 6.5578 | 1.5748 | 11.0289 | 6.0392 | 5.0457 | 1.4075 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 103 | 5.1302 | 1.5121 | 15.4858 | 4.4040 | 3.6202 | 1.3271 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 84 | 2.9084 | 1.3248 | 12.7595 | 3.4951 | 1.4018 | 1.1420 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 191.7325 (38.3465%), stress net PnL 189.9422 (37.9884%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 17 | 0.4650 | 1.0854 | 4.0865 | 7.9046 | 191.7325 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 17 | -1.0380 | 0.8380 | 4.8829 | -17.6456 | 189.9422 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 14 | -0.3499 | 0.9622 | 8.1014 | -4.8979 | 191.7325 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 14 | -1.8516 | 0.8206 | 9.0251 | -25.9221 | 189.9422 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 23 | 1.3128 | 1.2215 | 5.0074 | 30.1955 | 191.7325 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 23 | -0.1914 | 0.9723 | 5.6957 | -4.4017 | 189.9422 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL -38.5601 (-7.7120%), stress net PnL -40.0045 (-8.0009%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 7 | -13.4027 | 0.0413 | 9.3819 | -81.7626 | -38.5601 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 7 | -14.8848 | 0.0237 | 10.4194 | -93.6578 | -40.0045 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 9 | -5.7281 | 0.2416 | 6.0117 | -30.5705 | -38.5601 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 9 | -7.2217 | 0.1547 | 6.9040 | -45.5471 | -40.0045 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 8 | -1.8888 | 0.6354 | 2.4755 | -15.1105 | -38.5601 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 8 | -3.3882 | 0.4621 | 3.2270 | -27.1058 | -40.0045 | yes | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 422.7909 (84.5582%), stress net PnL 420.6535 (84.1307%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 21 | -0.1635 | 0.9709 | 4.7232 | -3.4335 | 422.7909 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 21 | -1.6655 | 0.7385 | 6.0222 | -34.9757 | 420.6535 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | 17.3925 | 2.6345 | 5.6174 | 243.4954 | 422.7909 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | 15.8641 | 2.3561 | 6.0642 | 222.0981 | 420.6535 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 31 | 0.0117 | 1.0020 | 5.7562 | 0.3632 | 422.7909 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 31 | -1.4906 | 0.7755 | 7.4455 | -46.2072 | 420.6535 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -85.6371 (-17.1274%), stress net PnL -87.0107 (-17.4021%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | -0.0285 | 0.9957 | 3.7533 | -0.2277 | -85.6371 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | -1.5307 | 0.7900 | 4.4988 | -12.2454 | -87.0107 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 12 | -3.2178 | 0.5887 | 7.5615 | -38.6140 | -85.6371 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | -4.7153 | 0.4650 | 9.0524 | -56.5830 | -87.0107 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 9 | -0.5354 | 0.9133 | 2.6147 | -4.8188 | -85.6371 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 9 | -2.0369 | 0.7156 | 2.7764 | -18.3318 | -87.0107 | yes | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL -160.3834 (-32.0767%), stress net PnL -161.6448 (-32.3290%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 5 | 6.5533 | 1.6186 | 4.8788 | 41.3143 | -160.3834 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 5 | 5.0412 | 1.4391 | 5.3019 | 32.2387 | -161.6448 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 9 | 12.4082 | 2.8081 | 5.0093 | 120.2220 | -160.3834 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 9 | 10.8873 | 2.3574 | 5.7956 | 105.0188 | -161.6448 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -5.5069 | 0.5638 | 9.2451 | -35.5075 | -160.3834 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -7.0009 | 0.4902 | 10.2449 | -48.9745 | -161.6448 | yes | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 559.1006 (111.8201%), stress net PnL 556.7584 (111.3517%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 10 | 24.1949 | 2.7544 | 6.4825 | 241.9494 | 559.1006 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 10 | 22.6563 | 2.5516 | 6.8765 | 226.5634 | 556.7584 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 16 | 15.1520 | 2.1825 | 12.3252 | 242.4319 | 559.1006 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 16 | 13.6270 | 1.9923 | 13.4146 | 218.0316 | 556.7584 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 11 | 26.3503 | 4.1484 | 4.4252 | 289.8535 | 559.1006 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 11 | 24.8085 | 3.7734 | 4.8833 | 272.8933 | 556.7584 | no | no | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -193.1285 (-38.6257%), stress net PnL -194.3406 (-38.8681%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | -9.5968 | 0.0089 | 7.6775 | -76.7747 | -193.1285 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | -11.0847 | 0 | 8.8677 | -88.6774 | -194.3406 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | -6.8337 | 0.2178 | 8.6132 | -86.1319 | -193.1285 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | -8.3257 | 0.1634 | 10.4030 | -104.0295 | -194.3406 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 9 | -6.0218 | 0.1824 | 5.4196 | -54.1961 | -193.1285 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 9 | -7.5150 | 0.1182 | 6.7635 | -67.6350 | -194.3406 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 7.5705 (1.5141%), stress net PnL 6.0569 (1.2114%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 7 | -1.2979 | 0.9033 | 5.7026 | -9.0850 | 7.5705 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 7 | -2.7982 | 0.8096 | 6.2949 | -19.5871 | 6.0569 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | -0.6590 | 0.9437 | 11.0486 | -9.2262 | 7.5705 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | -2.1603 | 0.8320 | 12.3841 | -30.2439 | 6.0569 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 7 | 0.7181 | 1.0769 | 4.0792 | 5.0265 | 7.5705 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 7 | -0.7853 | 0.9260 | 4.6740 | -5.4968 | 6.0569 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | — | Fail (stability) | Fail (stability) | — |
| M1-M18 | — | Fail (stability) | Fail (stability) | — | Fail (stability) |
| M1-M24 | Fail (stability) | Fail (stability) | Fail (stability) | — | — |
| M1-M30 | — | Fail | — | Fail (stability) | Fail |
| M1-M36 | — | Fail (stability) | — | Fail (stability) | Fail (stability) |
| M1-M42 | — | Fail | — | Fail (stability) | Fail |
| M1-M48 | — | Fail | — | Fail | Fail |
| M1-M54 | — | Fail | — | Fail | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x4 | 7 | 0 | 15.8641 |
| entry55-exit55-ema200-atr14x3 | 6 | 0 | 24.8085 |
| entry55-exit55-ema200-atr14x4 | 6 | 0 | 22.6563 |
| entry55-exit55-ema200-atr14x2 | 3 | 0 | -1.0380 |
| entry20-exit55-ema200-atr14x2 | 2 | 0 | -0.1914 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x2 | -4.4017 | 189.9422 | -194.3439 | Underperform |
| M1-M18 | entry55-exit55-ema200-atr14x2 | -27.1058 | -40.0045 | 12.8986 | Beat |
| M1-M24 | entry20-exit55-ema200-atr14x4 | 222.0981 | 420.6535 | -198.5555 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x4 | -12.2454 | -87.0107 | 74.7653 | Beat |
| M1-M36 | entry20-exit55-ema200-atr14x4 | 105.0188 | -161.6448 | 266.6636 | Beat |
| M1-M42 | entry55-exit55-ema200-atr14x3 | 272.8933 | 556.7584 | -283.8651 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x3 | -67.6350 | -194.3406 | 126.7056 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x3 | -5.4968 | 6.0569 | -11.5537 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x2 had 23 OOS trades (minimum 10), net expectancy -0.1913781026625649606324703639, PF 0.9722559047086289278366421613, net PnL -4.401696361238994094546818370 vs benchmark 189.94224338115385482501296340; parameter stability fail.
- M1-M18: best top-3 stress run entry55-exit55-ema200-atr14x2 had 8 OOS trades (minimum 10), net expectancy -3.3882277221379533725896909271, PF 0.4620832760631396370579887696, net PnL -27.105821777103626980717527417 vs benchmark -40.004450861728007108375638639; parameter stability fail.
- M1-M24: best top-3 stress run entry20-exit55-ema200-atr14x4 had 14 OOS trades (minimum 10), net expectancy 15.864147394147132790517111646, PF 2.3561156691918530372926926125, net PnL 222.09806351805985906723956304 vs benchmark 420.65352314625153625563293736; parameter stability fail.
- M1-M30: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy -1.5306692583643267607035725521, PF 0.790021644509364591393903481, net PnL -12.245354066914614085628580417 vs benchmark -87.01066956461216251273387622; parameter stability pass.
- M1-M36: best top-3 stress run entry20-exit55-ema200-atr14x4 had 9 OOS trades (minimum 10), net expectancy 10.887324320011692071510452074, PF 2.3574093118005209282139491085, net PnL 105.01881249325014842360112850 vs benchmark -161.64476042265150275667099161; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x3 had 11 OOS trades (minimum 10), net expectancy 24.808482302146984109120367843, PF 3.7734059717883972734189200504, net PnL 272.89330532361682520032404627 vs benchmark 556.75844248617335349893684106; parameter stability fail.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x3 had 9 OOS trades (minimum 10), net expectancy -7.5150008723982917971897454792, PF 0.1181983598433597107363117362, net PnL -67.635007851584626174707709313 vs benchmark -194.34060197609653907114525808; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x3 had 7 OOS trades (minimum 10), net expectancy -0.7852622823820709910467717833, PF 0.9260333458969968715975242976, net PnL -5.496835976674496937327402483 vs benchmark 6.0568701475983277877314222375; parameter stability pass.

Successful runs: none.

