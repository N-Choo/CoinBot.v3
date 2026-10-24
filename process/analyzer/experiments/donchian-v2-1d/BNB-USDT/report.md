# Donchian Trend-Following — BNB-USDT (1day, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 1day |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 1824 |
| SHA-256 | 45327b6e32e1104aa722cf36913f0058affe21c24b4ddb5d6704cf58aeb8d56b |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M12 | 0..364 | 364 | M13-M18 | 364..547 | 183 | 2021-10-24T00:00:00Z | 2022-10-22T00:00:00Z | 2022-10-23T00:00:00Z | 2023-04-23T00:00:00Z |
| M1-M18 | 0..547 | 547 | M19-M24 | 547..729 | 182 | 2021-10-24T00:00:00Z | 2023-04-23T00:00:00Z | 2023-04-24T00:00:00Z | 2023-10-22T00:00:00Z |
| M1-M24 | 0..729 | 729 | M25-M30 | 729..912 | 183 | 2021-10-24T00:00:00Z | 2023-10-22T00:00:00Z | 2023-10-23T00:00:00Z | 2024-04-22T00:00:00Z |
| M1-M30 | 0..912 | 912 | M31-M36 | 912..1094 | 182 | 2021-10-24T00:00:00Z | 2024-04-22T00:00:00Z | 2024-04-23T00:00:00Z | 2024-10-21T00:00:00Z |
| M1-M36 | 0..1094 | 1094 | M37-M42 | 1094..1276 | 182 | 2021-10-24T00:00:00Z | 2024-10-21T00:00:00Z | 2024-10-22T00:00:00Z | 2025-04-21T00:00:00Z |
| M1-M42 | 0..1276 | 1276 | M43-M48 | 1276..1459 | 183 | 2021-10-24T00:00:00Z | 2025-04-21T00:00:00Z | 2025-04-22T00:00:00Z | 2025-10-21T00:00:00Z |
| M1-M48 | 0..1459 | 1459 | M49-M54 | 1459..1641 | 182 | 2021-10-24T00:00:00Z | 2025-10-21T00:00:00Z | 2025-10-22T00:00:00Z | 2026-04-21T00:00:00Z |
| M1-M54 | 0..1641 | 1641 | M55-M60 | 1641..1824 | 183 | 2021-10-24T00:00:00Z | 2026-04-21T00:00:00Z | 2026-04-22T00:00:00Z | 2026-10-21T00:00:00Z |

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
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — | no |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — | no |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x2 | 4 | -3.9387 | 0.6495 | 4.3669 | 12.2849 | -5.4350 | 0.5598 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 3 | -9.4928 | 0.5062 | 5.6031 | 19.3698 | -10.9808 | 0.4563 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 4 | -24.2569 | 0.0520 | 10.1807 | 11.0892 | -25.7227 | 0.0357 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x2 | 4 | -3.9387 | 0.6495 | 4.3669 | 12.2849 | -5.4350 | 0.5598 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 3 | -9.4928 | 0.5062 | 5.6031 | 19.3698 | -10.9808 | 0.4563 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 3 | -29.6962 | 0.0564 | 9.3910 | 17.5288 | -31.1539 | 0.0392 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 7 | 36.2829 | 4.8650 | 6.3849 | 25.9617 | 34.7262 | 4.4666 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 9 | 34.7891 | 4.6933 | 8.2371 | 23.5827 | 33.2345 | 4.2456 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 6 | 22.4536 | 1.9768 | 13.7926 | 38.1101 | 20.9176 | 1.8732 | yes |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 9 | 24.1092 | 3.1126 | 6.3849 | 21.4309 | 22.5708 | 2.8456 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 14 | 15.3608 | 2.1363 | 7.9568 | 16.6871 | 13.8355 | 1.9566 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 7 | 11.8330 | 1.4364 | 13.7926 | 33.9148 | 10.3130 | 1.3664 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 11 | 17.5287 | 2.5197 | 6.3849 | 17.9027 | 16.0001 | 2.2828 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 16 | 11.9303 | 1.8944 | 9.3075 | 14.7254 | 10.4101 | 1.7254 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 10 | 5.2106 | 1.2908 | 10.1128 | 16.1246 | 3.7005 | 1.1985 | no |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 11 | 25.2777 | 2.0448 | 13.7926 | 32.3412 | 23.7375 | 1.9452 | yes |
| 2 | entry55-exit55-ema200-atr14x2 | 17 | 15.2555 | 2.3084 | 9.0639 | 12.7490 | 13.7303 | 2.0962 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 16 | 19.3884 | 1.9266 | 17.0356 | 26.8190 | 17.8571 | 1.8111 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 18 | 13.3073 | 2.0987 | 9.0639 | 12.1767 | 11.7850 | 1.9057 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 17 | 17.2877 | 1.8371 | 17.0356 | 25.2795 | 15.7595 | 1.7240 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 12 | 20.3770 | 1.8160 | 13.7926 | 29.9273 | 18.8442 | 1.7266 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 109.5255 (21.9051%), stress net PnL 107.8587 (21.5717%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 4 | 5.6216 | 1.5102 | 4.1323 | 22.4865 | 109.5255 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 4 | 4.1109 | 1.3498 | 4.4207 | 16.4437 | 107.8587 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 4 | -15.1148 | 0.3455 | 8.9517 | -60.4593 | 109.5255 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 4 | -16.5944 | 0.3139 | 9.3891 | -66.3774 | 107.8587 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 3 | -14.4736 | 0.1500 | 7.6448 | -69.3717 | 109.5255 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 3 | -15.9541 | 0.1138 | 8.0927 | -75.2765 | 107.8587 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL -172.1158 (-34.4232%), stress net PnL -173.3595 (-34.6719%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -172.1158 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -173.3595 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -172.1158 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -173.3595 | yes | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -172.1158 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -173.3595 | yes | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 885.4751 (177.0950%), stress net PnL 882.6426 (176.5285%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 5 | 65.8967 | 9.4040 | 3.9205 | 329.4833 | 885.4751 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 5 | 64.2954 | 8.6268 | 4.2151 | 321.4771 | 882.6426 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 4 | 67.9383 | 15.4921 | 1.8752 | 271.7531 | 885.4751 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 4 | 66.3340 | 14.1186 | 2.0226 | 265.3359 | 882.6426 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | 42.0191 | 9.9632 | 1.8752 | 168.0762 | 885.4751 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | 40.4537 | 9.0004 | 2.0226 | 161.8147 | 882.6426 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -6.5271 (-1.3054%), stress net PnL -8.0195 (-1.6039%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 2 | -18.4988 | 0 | 3.6998 | -36.9975 | -6.5271 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 2 | -19.9732 | 0 | 3.9946 | -39.9465 | -8.0195 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 5 | -19.6100 | 0.0615 | 10.4481 | -98.0500 | -6.5271 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 5 | -21.0828 | 0.0446 | 11.0333 | -105.4139 | -8.0195 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | -51.8902 | 0 | 5.8374 | -58.3739 | -6.5271 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -53.3145 | 0 | 6.1291 | -61.2908 | -8.0195 | no | yes | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL -2.6840 (-0.5368%), stress net PnL -4.1822 (-0.8364%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 2 | -12.0836 | 0 | 2.4167 | -24.1672 | -2.6840 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 2 | -13.5677 | 0 | 2.7135 | -27.1354 | -4.1822 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 2 | -12.0836 | 0 | 2.4167 | -24.1672 | -2.6840 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -13.5677 | 0 | 2.7135 | -27.1354 | -4.1822 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | 7.7078 | ∞ | 0 | 7.7078 | -2.6840 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 6.1940 | ∞ | 0 | 6.1940 | -4.1822 | yes | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 383.1141 (76.6228%), stress net PnL 381.0364 (76.2073%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 6 | 11.0879 | 1.9327 | 5.8937 | 66.5272 | 383.1141 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 6 | 9.5690 | 1.7581 | 6.2018 | 57.4137 | 381.0364 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 7 | 15.5748 | 2.8373 | 5.6550 | 109.0235 | 383.1141 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 7 | 14.0491 | 2.5797 | 5.9588 | 98.3440 | 381.0364 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | 23.6700 | 5.8447 | 1.9543 | 94.6801 | 383.1141 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | 22.1322 | 5.2125 | 2.1016 | 88.5289 | 381.0364 | no | no | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -202.4400 (-40.4880%), stress net PnL -203.6381 (-40.7276%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | -46.5435 | 0 | 4.6543 | -46.5435 | -202.4400 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -47.9758 | 0 | 4.7976 | -47.9758 | -203.6381 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 1 | -19.8125 | 0 | 1.9813 | -19.8125 | -202.4400 | yes | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -21.2850 | 0 | 2.1285 | -21.2850 | -203.6381 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | -29.8168 | 0 | 2.9817 | -29.8168 | -202.4400 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | -31.2743 | 0 | 3.1274 | -31.2743 | -203.6381 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 88.8499 (17.7700%), stress net PnL 87.2142 (17.4428%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 3 | -20.1025 | 0.0087 | 6.0806 | -60.3074 | 88.8499 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 3 | -21.5745 | 0 | 6.4724 | -64.7235 | 87.2142 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | 25.2867 | ∞ | 0 | 25.2867 | 88.8499 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 23.7464 | ∞ | 0 | 23.7464 | 87.2142 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | 25.2867 | ∞ | 0 | 25.2867 | 88.8499 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 23.7464 | ∞ | 0 | 23.7464 | 87.2142 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | Fail (stability) | — | — | — |
| M1-M18 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M24 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M30 | Fail | — | — | Fail | — | Fail |
| M1-M36 | Fail | — | — | Fail | — | Fail (stability) |
| M1-M42 | Fail | — | — | Fail | Fail (stability) | — |
| M1-M48 | — | — | Fail | Fail | — | Fail |
| M1-M54 | — | — | Fail | Fail | — | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x2 | 7 | 0 | 66.3340 |
| entry20-exit55-ema200-atr14x2 | 6 | 0 | 64.2954 |
| entry55-exit55-ema200-atr14x4 | 4 | 0 | 23.7464 |
| entry20-exit55-ema200-atr14x4 | 3 | 0 | 23.7464 |
| entry20-exit55-ema200-atr14x3 | 2 | 0 | 0 |
| entry55-exit55-ema200-atr14x3 | 2 | 0 | 40.4537 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x2 | 16.4437 | 107.8587 | -91.4149 | Underperform |
| M1-M18 | entry20-exit55-ema200-atr14x3 | 0 | -173.3595 | 173.3595 | Beat |
| M1-M24 | entry20-exit55-ema200-atr14x2 | 321.4771 | 882.6426 | -561.1656 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x2 | -39.9465 | -8.0195 | -31.9270 | Underperform |
| M1-M36 | entry55-exit55-ema200-atr14x4 | 6.1940 | -4.1822 | 10.3762 | Beat |
| M1-M42 | entry20-exit55-ema200-atr14x2 | 98.3440 | 381.0364 | -282.6924 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x2 | -21.2850 | -203.6381 | 182.3531 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 23.7464 | 87.2142 | -63.4678 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x2 had 4 OOS trades (minimum 10), net expectancy 4.1109350981418226789827220338, PF 1.3497798457590075376180137292, net PnL 16.443740392567290715930888135 vs benchmark 107.85867873109079009159249659; parameter stability fail.
- M1-M18: best top-3 stress run entry20-exit55-ema200-atr14x3 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -173.35948445874823051142756893; parameter stability fail.
- M1-M24: best top-3 stress run entry55-exit55-ema200-atr14x2 had 4 OOS trades (minimum 10), net expectancy 66.33396499547020377805113710, PF 14.11860487624974052209799074, net PnL 265.33585998188081511220454840 vs benchmark 882.6426213324762385784471280; parameter stability fail.
- M1-M30: best top-3 stress run entry55-exit55-ema200-atr14x2 had 2 OOS trades (minimum 10), net expectancy -19.973231422661264717411963724, PF 0, net PnL -39.946462845322529434823927449 vs benchmark -8.019510162645313510126270437; parameter stability pass.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 6.1939712187428516778595699793, PF infinite, net PnL 6.1939712187428516778595699793 vs benchmark -4.1821811908487715917118555472; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x3 had 4 OOS trades (minimum 10), net expectancy 22.132221884389149068058152328, PF 5.2124928692644533118437957598, net PnL 88.52888753755659627223260931 vs benchmark 381.03635706008828040750986289; parameter stability fail.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x2 had 1 OOS trades (minimum 10), net expectancy -21.285022936339367796429042723, PF 0, net PnL -21.285022936339367796429042723 vs benchmark -203.63811622624659550696791759; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 23.746436539248415042420908430, PF infinite, net PnL 23.746436539248415042420908430 vs benchmark 87.21419356370601364506988746; parameter stability pass.

Successful runs: none.

