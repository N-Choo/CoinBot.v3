# Donchian Trend-Following — BTC-USDT (4hour, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 10949 |
| SHA-256 | c350a642b1bcc40ae22e9e7393ee815306c74600c22e720647b008fe57436bde |

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
| 1 | entry55-exit55-ema200-atr14x2 | 9 | 5.5333 | 1.8572 | 5.3069 | 7.2333 | 4.0228 | 1.5526 | no |
| 2 | entry20-exit55-ema200-atr14x2 | 15 | 1.8960 | 1.3579 | 4.1660 | 4.2719 | 0.3909 | 1.0612 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 9 | -2.2240 | 0.7740 | 8.3160 | 6.6676 | -3.7229 | 0.6507 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 16 | 8.2200 | 1.8640 | 12.1738 | 11.7315 | 6.7054 | 1.6541 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 31 | 5.5727 | 2.4184 | 4.3948 | 3.0911 | 4.0620 | 1.8522 | yes |
| 3 | entry55-exit55-ema200-atr14x2 | 23 | 5.7232 | 2.2958 | 5.3069 | 3.6873 | 4.2124 | 1.8248 | yes |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 31 | 4.0718 | 1.8841 | 5.3069 | 3.0263 | 2.5634 | 1.4741 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 23 | 4.4601 | 1.4781 | 12.1738 | 8.3979 | 2.9512 | 1.2885 | yes |
| 3 | entry20-exit55-ema200-atr14x2 | 50 | 2.9302 | 1.6712 | 6.8305 | 2.1865 | 1.4235 | 1.2700 | yes |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 50 | 4.0532 | 1.8948 | 5.3069 | 2.4010 | 2.5448 | 1.4753 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 53 | 5.7923 | 1.6794 | 14.2980 | 5.1676 | 4.2813 | 1.4477 | yes |
| 3 | entry55-exit55-ema200-atr14x4 | 36 | 6.7956 | 1.6695 | 12.1738 | 6.8630 | 5.2831 | 1.4762 | yes |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 44 | 5.8188 | 1.5960 | 12.1738 | 5.7295 | 4.3078 | 1.4037 | yes |
| 2 | entry20-exit55-ema200-atr14x3 | 72 | 3.9599 | 1.6346 | 11.0899 | 2.8414 | 2.4517 | 1.3419 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 65 | 4.5354 | 1.5337 | 14.2980 | 4.3172 | 3.0263 | 1.3177 | yes |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x3 | 59 | 5.0967 | 1.7430 | 8.6560 | 3.9021 | 3.5868 | 1.4607 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 53 | 6.1803 | 1.6575 | 12.1738 | 5.4114 | 4.6688 | 1.4511 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 84 | 4.7011 | 1.7428 | 11.0899 | 3.0225 | 3.1918 | 1.4391 | yes |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 62 | 6.9531 | 1.8138 | 12.1738 | 4.7292 | 5.4404 | 1.5770 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 74 | 4.2876 | 1.6466 | 8.6560 | 3.2210 | 2.7789 | 1.3671 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 91 | 4.8773 | 1.6132 | 14.2980 | 3.5592 | 3.3677 | 1.3763 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 70 | 5.1490 | 1.5985 | 12.1738 | 4.2405 | 3.6390 | 1.3817 | yes |
| 2 | entry20-exit55-ema200-atr14x4 | 101 | 3.9305 | 1.5094 | 14.2980 | 3.2243 | 2.4224 | 1.2774 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 115 | 3.0959 | 1.4984 | 11.0899 | 2.3085 | 1.5890 | 1.2215 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 202.2135 (40.4427%), stress net PnL 200.4075 (40.0815%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 14 | 5.8453 | 2.8816 | 1.5586 | 81.8346 | 202.2135 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 14 | 4.3343 | 2.1682 | 2.0627 | 60.6802 | 200.4075 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 16 | 8.8218 | 4.1018 | 1.4620 | 141.1495 | 202.2135 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 16 | 7.3063 | 3.1231 | 1.7613 | 116.9014 | 200.4075 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 10 | 7.8392 | 2.5319 | 3.1943 | 78.3917 | 202.2135 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 10 | 6.3251 | 2.1076 | 3.6230 | 63.2514 | 200.4075 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 69.4707 (13.8941%), stress net PnL 67.8641 (13.5728%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 7 | -4.1339 | 0.5357 | 5.5324 | -20.4196 | 69.4707 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 7 | -5.6300 | 0.4469 | 6.2734 | -32.4069 | 67.8641 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 19 | -1.9922 | 0.6077 | 8.0557 | -29.3344 | 69.4707 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 19 | -3.4915 | 0.4275 | 10.0027 | -59.3354 | 67.8641 | no | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 8 | -0.6762 | 0.8687 | 3.9767 | 3.1081 | 69.4707 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 8 | -2.1775 | 0.6526 | 4.8553 | -10.4169 | 67.8641 | no | yes | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 551.8374 (110.3675%), stress net PnL 549.5061 (109.9012%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 19 | 2.5066 | 1.5690 | 3.6523 | 47.6247 | 551.8374 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 19 | 1.0005 | 1.1899 | 4.5555 | 19.0103 | 549.5061 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 13 | 7.7636 | 1.6691 | 7.0961 | 100.9263 | 551.8374 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 13 | 6.2496 | 1.4950 | 7.8151 | 81.2454 | 549.5061 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 24 | 3.6550 | 1.8594 | 4.1042 | 83.0809 | 551.8374 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 24 | 2.1473 | 1.4141 | 5.3657 | 45.3997 | 549.5061 | no | yes | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL 8.8522 (1.7704%), stress net PnL 7.3367 (1.4673%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 17 | -6.0942 | 0.0874 | 10.3602 | -103.6019 | 8.8522 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 17 | -7.5873 | 0.0408 | 12.8985 | -128.9846 | 7.3367 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | 0.3276 | 1.0425 | 5.7649 | 3.6036 | 8.8522 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | -1.1751 | 0.8641 | 6.7909 | -12.9266 | 7.3367 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 8 | 1.4236 | 1.1775 | 4.7544 | 11.3886 | 8.8522 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | -0.0808 | 0.9908 | 5.4946 | -0.6465 | 7.3367 | no | yes | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL 195.7590 (39.1518%), stress net PnL 193.9626 (38.7925%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 9 | 7.9477 | 2.0420 | 6.0207 | 109.7711 | 195.7590 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 9 | 6.4335 | 1.7324 | 6.9535 | 94.5836 | 193.9626 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 12 | 9.1479 | 2.3324 | 5.5548 | 148.0171 | 195.7590 | no | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 12 | 7.6319 | 1.9714 | 6.2429 | 128.2654 | 193.9626 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 12 | 4.8135 | 1.5441 | 7.2281 | 96.0041 | 195.7590 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | 3.3040 | 1.3360 | 8.5590 | 76.3306 | 193.9626 | no | yes | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 71.4096 (14.2819%), stress net PnL 69.8001 (13.9600%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 15 | -1.9198 | 0.7091 | 5.4631 | -28.7975 | 71.4096 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 15 | -3.4192 | 0.5553 | 7.3763 | -51.2880 | 69.8001 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 9 | 6.5027 | 2.7368 | 3.0404 | 58.5247 | 71.4096 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 9 | 4.9907 | 2.1327 | 3.4806 | 44.9165 | 69.8001 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 20 | -1.3075 | 0.7868 | 5.6348 | -26.1508 | 71.4096 | no | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 20 | -2.8078 | 0.6095 | 8.0505 | -56.1566 | 69.8001 | no | yes | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -139.4926 (-27.8985%), stress net PnL -140.7853 (-28.1571%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | -8.4030 | 0.0267 | 7.6435 | -76.4349 | -139.4926 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | -9.8926 | 0.0043 | 8.9840 | -89.8403 | -140.7853 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 8 | -5.8802 | 0.1652 | 5.6253 | -56.2529 | -139.4926 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 8 | -7.3736 | 0.0964 | 6.9689 | -69.6887 | -140.7853 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 11 | -4.6269 | 0.1461 | 5.2743 | -48.9750 | -139.4926 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 11 | -6.1222 | 0.0779 | 6.7686 | -66.9285 | -140.7853 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 20.5050 (4.1010%), stress net PnL 18.9719 (3.7944%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 8 | 1.7886 | 1.2287 | 3.0078 | 14.3090 | 20.5050 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 8 | 0.2837 | 1.0317 | 3.6042 | 2.2694 | 18.9719 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | -0.0609 | 0.9933 | 6.7170 | -0.8527 | 20.5050 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | -1.5631 | 0.8477 | 7.9087 | -21.8829 | 18.9719 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 16 | -1.0763 | 0.8663 | 6.9931 | -17.2211 | 20.5050 | no | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 16 | -2.5770 | 0.7245 | 8.3346 | -41.2312 | 18.9719 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M18 | Fail | — | — | Fail | — | Fail |
| M1-M24 | Fail | — | — | Fail | — | Fail |
| M1-M30 | — | — | Fail | Fail | — | Fail |
| M1-M36 | — | Fail | Fail | — | — | Fail |
| M1-M42 | — | Fail | — | — | Fail | Fail |
| M1-M48 | — | — | Fail | — | Fail | Fail |
| M1-M54 | — | Fail | Fail | — | — | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x4 | 7 | 0 | 6.4335 |
| entry20-exit55-ema200-atr14x4 | 4 | 0 | 3.3040 |
| entry55-exit55-ema200-atr14x2 | 4 | 0 | 4.3343 |
| entry20-exit55-ema200-atr14x2 | 3 | 0 | 7.3063 |
| entry20-exit55-ema200-atr14x3 | 3 | 0 | 7.6319 |
| entry55-exit55-ema200-atr14x3 | 3 | 0 | 6.3251 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x2 | 116.9014 | 200.4075 | -83.5061 | Underperform |
| M1-M18 | entry55-exit55-ema200-atr14x2 | -10.4169 | 67.8641 | -78.2810 | Underperform |
| M1-M24 | entry55-exit55-ema200-atr14x4 | 81.2454 | 549.5061 | -468.2607 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x4 | -0.6465 | 7.3367 | -7.9832 | Underperform |
| M1-M36 | entry20-exit55-ema200-atr14x3 | 128.2654 | 193.9626 | -65.6972 | Underperform |
| M1-M42 | entry55-exit55-ema200-atr14x4 | 44.9165 | 69.8001 | -24.8835 | Underperform |
| M1-M48 | entry20-exit55-ema200-atr14x4 | -66.9285 | -140.7853 | 73.8568 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 2.2694 | 18.9719 | -16.7025 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x2 had 16 OOS trades (minimum 10), net expectancy 7.3063372044536561048153995544, PF 3.1231400340102563995566373049, net PnL 116.90139527125849767704639287 vs benchmark 200.40745176279914244976253244; parameter stability fail.
- M1-M18: best top-3 stress run entry55-exit55-ema200-atr14x2 had 8 OOS trades (minimum 10), net expectancy -2.1774565671619608134551896739, PF 0.6525937574481468311145391298, net PnL -10.416893826862686084726002878 vs benchmark 67.864092346247431993355965790; parameter stability pass.
- M1-M24: best top-3 stress run entry55-exit55-ema200-atr14x4 had 13 OOS trades (minimum 10), net expectancy 6.2496453583719022010849741877, PF 1.4949818357613131250106536679, net PnL 81.24538965883472861410466444 vs benchmark 549.50610094464182065200058686; parameter stability pass.
- M1-M30: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy -0.0808109621952428094905199609, PF 0.9907742150967328383328389437, net PnL -0.646487697561942475924159687 vs benchmark 7.3366697575356949992966129992; parameter stability pass.
- M1-M36: best top-3 stress run entry20-exit55-ema200-atr14x3 had 12 OOS trades (minimum 10), net expectancy 7.6319177425548887567588855142, PF 1.9713810421315654835323025726, net PnL 128.26540988390737660161564152 vs benchmark 193.96262391387790972534060107; parameter stability pass.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x4 had 9 OOS trades (minimum 10), net expectancy 4.9907230227128280763542473479, PF 2.1326601065146722967316963301, net PnL 44.916507204415452687188226131 vs benchmark 69.800053114078417625375782106; parameter stability pass.
- M1-M48: best top-3 stress run entry20-exit55-ema200-atr14x4 had 11 OOS trades (minimum 10), net expectancy -6.1222085887721565249171280507, PF 0.0778816677031879486045531857, net PnL -66.928466277160367801559178318 vs benchmark -140.78528095345810265100808021; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x4 had 8 OOS trades (minimum 10), net expectancy 0.2836789865509207356070148932, PF 1.0317442253144636267154086367, net PnL 2.269431892407365884856119146 vs benchmark 18.971918691874952756884042654; parameter stability pass.

Successful runs: none.

