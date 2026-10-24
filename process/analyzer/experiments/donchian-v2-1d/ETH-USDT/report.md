# Donchian Trend-Following — ETH-USDT (1day, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | ETH-USDT |
| Timeframe | 1day |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 1824 |
| SHA-256 | 9838a08c690f02932cd6de89d04b65bc72ae0968ef82471d0601b295c38822bd |

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
| 1 | entry20-exit55-ema200-atr14x2 | 3 | 12.9546 | 1.8394 | 4.6299 | 36.5016 | 11.4329 | 1.6966 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 3 | 3.1480 | 1.2040 | 4.6299 | 26.8406 | 1.6410 | 1.1000 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 3 | 2.7830 | 1.1692 | 4.9353 | 28.1625 | 1.2765 | 1.0732 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x2 | 5 | -2.2553 | 0.8831 | 4.8265 | 22.0580 | -3.7542 | 0.8165 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 4 | -4.1488 | 0.7706 | 4.6299 | 20.3335 | -5.6448 | 0.7058 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 5 | -14.0011 | 0.4518 | 7.7705 | 18.5491 | -15.4823 | 0.4202 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 6 | 23.3395 | 2.5815 | 7.9569 | 29.4748 | 21.8022 | 2.4078 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 10 | 5.5610 | 1.3742 | 4.2874 | 14.7715 | 4.0504 | 1.2573 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 12 | 3.0942 | 1.1667 | 8.5767 | 15.8800 | 1.5873 | 1.0813 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 7 | 13.1815 | 1.6769 | 7.9569 | 26.9022 | 11.6594 | 1.5743 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 11 | 3.6952 | 1.2485 | 5.6441 | 13.4909 | 2.1874 | 1.1384 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 13 | 3.0164 | 1.1761 | 8.5767 | 14.6077 | 1.5096 | 1.0837 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 8 | 18.4811 | 2.0846 | 7.9569 | 23.8932 | 16.9511 | 1.9542 | yes |
| 2 | entry55-exit55-ema200-atr14x2 | 13 | 6.8760 | 1.5464 | 5.6441 | 11.6762 | 5.3634 | 1.4011 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 10 | 9.8974 | 1.4251 | 9.9224 | 21.6638 | 8.3802 | 1.3470 | yes |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 11 | 24.2941 | 2.0927 | 9.4308 | 28.0729 | 22.7554 | 1.9885 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 14 | 9.8331 | 1.2904 | 17.4920 | 27.9439 | 8.3161 | 1.2392 | no |
| 3 | entry55-exit55-ema200-atr14x2 | 19 | 4.4052 | 1.2705 | 7.5618 | 10.5795 | 2.8963 | 1.1691 | no |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 11 | 24.2941 | 2.0927 | 9.4308 | 28.0729 | 22.7554 | 1.9885 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 14 | 9.8331 | 1.2904 | 17.4920 | 27.9439 | 8.3161 | 1.2392 | no |
| 3 | entry55-exit55-ema200-atr14x2 | 19 | 4.4052 | 1.2705 | 7.5618 | 10.5795 | 2.8963 | 1.1691 | no |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 206.8154 (41.3631%), stress net PnL 205.0024 (41.0005%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | 42.7777 | ∞ | 0 | 85.5554 | 206.8154 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | 41.2112 | 75.2320 | 0.1110 | 82.4223 | 205.0024 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | 19.5008 | 3.0856 | 1.8700 | 39.0016 | 206.8154 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | 17.9692 | 2.7814 | 2.0174 | 35.9385 | 205.0024 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | -13.3551 | 0 | 1.3355 | 32.8791 | 206.8154 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | -14.8373 | 0 | 1.4837 | 29.8252 | 205.0024 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL -54.6066 (-10.9213%), stress net PnL -56.0268 (-11.2054%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 3 | -25.8153 | 0 | 7.7446 | -77.4458 | -54.6066 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | -27.2787 | 0 | 8.1836 | -81.8362 | -56.0268 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 1 | -26.0393 | 0 | 2.6039 | -26.0393 | -54.6066 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -27.5024 | 0 | 2.7502 | -27.5024 | -56.0268 | yes | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 3 | -33.4825 | 0 | 10.0448 | -100.4476 | -54.6066 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 3 | -34.9345 | 0 | 10.4803 | -104.8034 | -56.0268 | no | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 459.6234 (91.9247%), stress net PnL 457.4307 (91.4861%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 6 | 9.1754 | 1.4601 | 8.4433 | 55.0525 | 459.6234 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 6 | 7.6594 | 1.3707 | 8.7386 | 45.9563 | 457.4307 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 5 | 15.7702 | 2.1323 | 4.1990 | 78.8511 | 459.6234 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 5 | 14.2443 | 1.9819 | 4.3500 | 71.2214 | 457.4307 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 5 | -3.2974 | 0.9145 | 13.6369 | -16.4869 | 459.6234 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 5 | -4.7947 | 0.8784 | 13.9379 | -23.9734 | 457.4307 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -84.7135 (-16.9427%), stress net PnL -86.0885 (-17.2177%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | -47.7665 | 0 | 4.7767 | -47.7665 | -84.7135 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -49.1970 | 0 | 4.9197 | -49.1970 | -86.0885 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 1 | -14.9627 | 0 | 1.4963 | -14.9627 | -84.7135 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -16.4425 | 0 | 1.6443 | -16.4425 | -86.0885 | yes | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 1 | 2.0824 | ∞ | 0 | 2.0824 | -84.7135 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 1 | 0.5770 | ∞ | 0 | 0.5770 | -86.0885 | yes | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL -205.0446 (-41.0089%), stress net PnL -206.2388 (-41.2478%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | 43.0767 | ∞ | 0 | 43.0767 | -205.0446 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 41.5097 | ∞ | 0 | 41.5097 | -206.2388 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 2 | 18.2879 | ∞ | 0 | 36.5759 | -205.0446 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 2 | 16.7582 | ∞ | 0 | 33.5164 | -206.2388 | yes | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 2 | 18.2879 | ∞ | 0 | 36.5759 | -205.0446 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | 16.7582 | ∞ | 0 | 33.5164 | -206.2388 | yes | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 723.5207 (144.7041%), stress net PnL 720.9315 (144.1863%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 3 | 56.5113 | 3.9178 | 5.8103 | 169.5339 | 723.5207 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 3 | 54.9242 | 3.7003 | 6.1021 | 164.7725 | 720.9315 | no | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 6 | 1.4998 | 1.0686 | 5.5202 | 8.9989 | 723.5207 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 6 | -0.0047 | 0.9998 | 5.8124 | -0.0282 | 720.9315 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 5 | -9.7487 | 0.6607 | 8.0365 | -48.7434 | 723.5207 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 5 | -11.2363 | 0.6202 | 8.3249 | -56.1815 | 720.9315 | no | yes | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -200.7865 (-40.1573%), stress net PnL -201.9871 (-40.3974%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -200.7865 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -201.9871 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 3 | -15.2674 | 0 | 4.5802 | -45.8022 | -200.7865 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 3 | -16.7467 | 0 | 5.0240 | -50.2401 | -201.9871 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -200.7865 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -201.9871 | yes | no | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 29.9240 (5.9848%), stress net PnL 28.3767 (5.6753%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | 45.7864 | ∞ | 0 | 45.7864 | 29.9240 | yes | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 44.2154 | ∞ | 0 | 44.2154 | 28.3767 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | 45.7864 | ∞ | 0 | 45.7864 | 29.9240 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 44.2154 | ∞ | 0 | 44.2154 | 28.3767 | yes | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 2 | -2.4423 | 0.8248 | 2.7255 | -4.8845 | 29.9240 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 2 | -3.9408 | 0.7314 | 2.8726 | -7.8817 | 28.3767 | no | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | Fail (stability) | — | — | — |
| M1-M18 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M24 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M30 | Fail (stability) | — | — | Fail (stability) | — | Fail (stability) |
| M1-M36 | Fail (stability) | — | — | Fail (stability) | — | Fail (stability) |
| M1-M42 | — | — | — | Fail | Fail | Fail |
| M1-M48 | — | — | Fail (stability) | Fail (stability) | — | Fail (stability) |
| M1-M54 | — | — | Fail (stability) | Fail (stability) | — | Fail (stability) |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x2 | 7 | 0 | 16.7582 |
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 41.2112 |
| entry55-exit55-ema200-atr14x4 | 5 | 0 | 54.9242 |
| entry20-exit55-ema200-atr14x3 | 3 | 0 | 17.9692 |
| entry20-exit55-ema200-atr14x4 | 3 | 0 | 44.2154 |
| entry55-exit55-ema200-atr14x3 | 1 | 0 | -11.2363 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x2 | 82.4223 | 205.0024 | -122.5801 | Underperform |
| M1-M18 | entry55-exit55-ema200-atr14x2 | -27.5024 | -56.0268 | 28.5244 | Beat |
| M1-M24 | entry55-exit55-ema200-atr14x2 | 71.2214 | 457.4307 | -386.2093 | Underperform |
| M1-M30 | entry20-exit55-ema200-atr14x2 | 0.5770 | -86.0885 | 86.6656 | Beat |
| M1-M36 | entry55-exit55-ema200-atr14x4 | 41.5097 | -206.2388 | 247.7485 | Beat |
| M1-M42 | entry55-exit55-ema200-atr14x4 | 164.7725 | 720.9315 | -556.1590 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x2 | 0 | -201.9871 | 201.9871 | Beat |
| M1-M54 | entry20-exit55-ema200-atr14x4 | 44.2154 | 28.3767 | 15.8386 | Beat |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x2 had 2 OOS trades (minimum 10), net expectancy 41.21117082180279723789097079, PF 75.232035971223049728892370546, net PnL 82.42234164360559447578194158 vs benchmark 205.00241683793864657075435792; parameter stability fail.
- M1-M18: best top-3 stress run entry20-exit55-ema200-atr14x2 had 3 OOS trades (minimum 10), net expectancy -27.278725070850089812970624717, PF 0, net PnL -81.83617521255026943891187415 vs benchmark -56.026787872277573381314284180; parameter stability fail.
- M1-M24: best top-3 stress run entry55-exit55-ema200-atr14x2 had 5 OOS trades (minimum 10), net expectancy 14.244273286404312162597328602, PF 1.9818680518279313597187202148, net PnL 71.221366432021560812986643010 vs benchmark 457.43068955774768113589775957; parameter stability fail.
- M1-M30: best top-3 stress run entry20-exit55-ema200-atr14x2 had 1 OOS trades (minimum 10), net expectancy 0.5770471818652676544008395230, PF infinite, net PnL 0.5770471818652676544008395230 vs benchmark -86.08852581449561581680801465; parameter stability fail.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 41.509688305923223139707451959, PF infinite, net PnL 41.509688305923223139707451959 vs benchmark -206.23881414150504732334368297; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x4 had 3 OOS trades (minimum 10), net expectancy 54.924153536209753642700988803, PF 3.7002796819448051305603080438, net PnL 164.77246060862926092810296641 vs benchmark 720.93150134217990275526742301; parameter stability pass.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x2 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -201.98712732229715076471863931; parameter stability fail.
- M1-M54: best top-3 stress run entry20-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 44.215395908859116738632220125, PF infinite, net PnL 44.215395908859116738632220125 vs benchmark 28.376747263891097064725572677; parameter stability fail.

Successful runs: none.

