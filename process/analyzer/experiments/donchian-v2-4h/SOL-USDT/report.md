# Donchian Trend-Following — SOL-USDT (4hour, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 10949 |
| SHA-256 | 0b28a6f127141dcd210ab6910b60f089b2d8b20d1dd2ceca5d82496d6e143f1c |

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
| 1 | entry20-exit55-ema200-atr14x3 | 17 | 0.9072 | 1.0791 | 8.0984 | 11.3341 | -0.5964 | 0.9530 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 12 | -2.2197 | 0.8718 | 9.9754 | 16.6273 | -3.7186 | 0.7994 | no |
| 3 | entry20-exit55-ema200-atr14x4 | 17 | -6.6414 | 0.5693 | 17.2073 | 9.9318 | -8.1337 | 0.5136 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x3 | 31 | 1.4511 | 1.1161 | 14.5787 | 7.7452 | -0.0533 | 0.9961 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 28 | 1.5781 | 1.0932 | 21.3139 | 13.0177 | 0.0735 | 1.0041 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 23 | 0.0489 | 1.0033 | 12.2009 | 9.8718 | -1.4535 | 0.9084 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 27 | 8.1146 | 1.4137 | 21.8735 | 16.4177 | 6.6002 | 1.3171 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 38 | 5.4354 | 1.3250 | 21.3139 | 11.8746 | 3.9250 | 1.2195 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 45 | 1.5276 | 1.1192 | 22.3164 | 6.4160 | 0.0231 | 1.0017 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 53 | 14.2815 | 1.9073 | 21.3139 | 11.4702 | 12.7578 | 1.7595 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 39 | 14.5764 | 1.7831 | 21.8735 | 14.0296 | 13.0522 | 1.6638 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 65 | 9.0406 | 1.7213 | 22.3164 | 7.7458 | 7.5247 | 1.5561 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 64 | 12.8943 | 1.8586 | 21.3139 | 9.7011 | 11.3727 | 1.7073 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 46 | 13.8826 | 1.8000 | 21.8735 | 12.0957 | 12.3595 | 1.6727 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 79 | 8.2037 | 1.6956 | 22.3164 | 6.5442 | 6.6892 | 1.5228 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 72 | 13.7477 | 1.9407 | 21.3139 | 8.9530 | 12.2247 | 1.7817 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 53 | 14.9944 | 1.9330 | 21.8735 | 10.8569 | 13.4696 | 1.7905 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 89 | 8.4664 | 1.7284 | 22.3164 | 6.0287 | 6.9514 | 1.5512 | no |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 87 | 12.4765 | 1.8977 | 21.3139 | 7.5617 | 10.9555 | 1.7352 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 64 | 13.2010 | 1.8584 | 21.8735 | 9.1748 | 11.6789 | 1.7157 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 109 | 6.6540 | 1.5801 | 22.3164 | 5.0430 | 5.1418 | 1.4138 | no |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 70 | 11.2213 | 1.7318 | 21.8735 | 8.4469 | 9.7022 | 1.5953 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 98 | 9.8477 | 1.7100 | 21.3139 | 6.7699 | 8.3307 | 1.5587 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 87 | 5.3751 | 1.4294 | 12.5944 | 5.7471 | 3.8648 | 1.2868 | no |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL -131.7904 (-26.3581%), stress net PnL -133.0947 (-26.6189%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 14 | 2.1115 | 1.1534 | 14.7788 | 29.5612 | -131.7904 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 14 | 0.6061 | 1.0409 | 15.8220 | 8.4853 | -133.0947 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 11 | 2.5237 | 1.2100 | 9.5865 | 27.7609 | -131.7904 | yes | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 11 | 1.0177 | 1.0786 | 10.2024 | 11.1944 | -133.0947 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | 14.2811 | 1.7411 | 12.5979 | 157.0921 | -131.7904 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | 12.7574 | 1.6274 | 13.3659 | 140.3313 | -133.0947 | yes | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 200.8834 (40.1767%), stress net PnL 199.0794 (39.8159%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 14 | 1.6971 | 1.1256 | 16.0120 | 116.6008 | 200.8834 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 14 | 0.1923 | 1.0132 | 17.1897 | 93.8918 | 199.0794 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 10 | 15.4220 | 1.9098 | 13.2241 | 307.6760 | 200.8834 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | 13.8966 | 1.7730 | 13.9554 | 290.6890 | 199.0794 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | 4.6680 | 1.4851 | 5.6079 | 130.1855 | 200.8834 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | 3.1588 | 1.2942 | 6.2004 | 116.4696 | 199.0794 | no | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 2062.8688 (412.5738%), stress net PnL 2058.2676 (411.6535%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 12 | 15.5682 | 1.9518 | 9.4387 | 186.8180 | 2062.8688 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 12 | 14.0425 | 1.8220 | 10.1177 | 168.5104 | 2058.2676 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 15 | 20.6041 | 2.5548 | 8.8491 | 309.0613 | 2062.8688 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 15 | 19.0709 | 2.3575 | 9.4004 | 286.0632 | 2058.2676 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 20 | 17.1733 | 2.4433 | 7.1595 | 343.4653 | 2062.8688 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 20 | 15.6452 | 2.2386 | 8.3407 | 312.9043 | 2058.2676 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL 46.7113 (9.3423%), stress net PnL 45.1388 (9.0278%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 10 | 8.9166 | 1.8410 | 4.4864 | 114.8236 | 46.7113 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | 7.4010 | 1.6359 | 4.9066 | 98.1264 | 45.1388 | yes | yes | Pass |
| entry55-exit55-ema200-atr14x4 | Base | 6 | 15.1614 | 2.7677 | 2.2030 | 116.6261 | 46.7113 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 6 | 13.6364 | 2.4639 | 2.3502 | 105.9352 | 45.1388 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 13 | 6.2544 | 1.8450 | 5.1073 | 81.3077 | 46.7113 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 13 | 4.7428 | 1.5657 | 6.1960 | 61.6563 | 45.1388 | yes | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL -51.4869 (-10.2974%), stress net PnL -52.9118 (-10.5824%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 8 | 15.7106 | 2.0365 | 6.5125 | 170.1444 | -51.4869 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 8 | 14.1847 | 1.8825 | 6.8032 | 156.3685 | -52.9118 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 7 | 16.7425 | 2.4004 | 6.5125 | 161.6571 | -51.4869 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 7 | 15.2151 | 2.1890 | 6.8032 | 149.3962 | -52.9118 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 10 | 10.5415 | 2.0255 | 4.1435 | 125.4730 | -51.4869 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 10 | 9.0234 | 1.8082 | 4.4377 | 108.7598 | -52.9118 | yes | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 92.6505 (18.5301%), stress net PnL 91.0090 (18.2018%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 15 | 2.5989 | 1.2060 | 6.0231 | 38.9835 | 92.6505 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 15 | 1.0927 | 1.0803 | 6.7707 | 16.3911 | 91.0090 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 11 | -0.3853 | 0.9739 | 7.3928 | -4.2381 | 92.6505 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 11 | -1.8870 | 0.8799 | 8.0025 | -20.7565 | 91.0090 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 20 | -3.0921 | 0.7431 | 13.3005 | -61.8415 | 92.6505 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 20 | -4.5897 | 0.6448 | 15.5203 | -91.7937 | 91.0090 | no | no | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -262.7744 (-52.5549%), stress net PnL -263.8819 (-52.7764%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 10 | -10.8760 | 0.2145 | 13.7331 | -118.0415 | -262.7744 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 10 | -12.3620 | 0.1855 | 15.0798 | -134.3889 | -263.8819 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 5 | -9.5519 | 0.3834 | 6.8587 | -47.7596 | -262.7744 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 5 | -11.0398 | 0.3378 | 7.3109 | -55.1991 | -263.8819 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 11 | -10.6782 | 0.0756 | 12.8710 | -126.7412 | -262.7744 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 11 | -12.1644 | 0.0570 | 14.4578 | -144.5778 | -263.8819 | yes | no | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 101.6628 (20.3326%), stress net PnL 100.0079 (20.0016%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 6 | 18.9799 | 8.7743 | 0.7453 | 113.8796 | 101.6628 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 6 | 17.4492 | 6.9382 | 0.8938 | 104.6950 | 100.0079 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 12 | 7.5589 | 1.9451 | 3.3982 | 90.7072 | 101.6628 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | 6.0453 | 1.6727 | 3.6935 | 72.5439 | 100.0079 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | 10.3923 | 2.7470 | 3.9961 | 83.1385 | 101.6628 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | 8.8744 | 2.2901 | 4.5477 | 70.9956 | 100.0079 | no | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | Fail (stability) | — |
| M1-M18 | Fail (stability) | Fail (stability) | Fail (stability) | — |
| M1-M24 | Fail (stability) | Fail | — | Fail |
| M1-M30 | Fail (stability) | Pass | — | Fail |
| M1-M36 | Fail (stability) | Fail | — | Fail |
| M1-M42 | Fail (stability) | Fail | — | Fail |
| M1-M48 | Fail (stability) | Fail | — | Fail |
| M1-M54 | — | Fail | Fail (stability) | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x4 | 8 | 1 | 19.0709 |
| entry20-exit55-ema200-atr14x3 | 7 | 0 | 15.6452 |
| entry55-exit55-ema200-atr14x4 | 6 | 0 | 17.4492 |
| entry55-exit55-ema200-atr14x3 | 3 | 0 | 8.8744 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x4 | 140.3313 | -133.0947 | 273.4259 | Beat |
| M1-M18 | entry20-exit55-ema200-atr14x4 | 290.6890 | 199.0794 | 91.6096 | Beat |
| M1-M24 | entry20-exit55-ema200-atr14x3 | 312.9043 | 2058.2676 | -1745.3633 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x4 | 105.9352 | 45.1388 | 60.7963 | Beat |
| M1-M36 | entry20-exit55-ema200-atr14x4 | 156.3685 | -52.9118 | 209.2803 | Beat |
| M1-M42 | entry20-exit55-ema200-atr14x4 | 16.3911 | 91.0090 | -74.6179 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x4 | -55.1991 | -263.8819 | 208.6828 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 104.6950 | 100.0079 | 4.6872 | Beat |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M30: entry20-exit55-ema200-atr14x4 passed OOS stress with 10 trades (minimum 10), net expectancy 7.4009581437421214304463906842, PF 1.6359084161192883364092824641, net PnL 98.12635418147743845005590627 > benchmark 45.138843690609460831532633880, and parameter stability.

Successful runs:

| Window | Config | OOS trades | Stress expectancy | Stress PF | Stress net PnL | Benchmark stress net PnL |
|---|---|---:|---:|---:|---:|---:|
| M1-M30 | entry20-exit55-ema200-atr14x4 | 10 | 7.4010 | 1.6359 | 98.1264 | 45.1388 |

