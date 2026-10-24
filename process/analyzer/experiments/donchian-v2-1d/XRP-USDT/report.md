# Donchian Trend-Following — XRP-USDT (1day, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 1day |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 1824 |
| SHA-256 | c32f14e85b6131f2a7c55f926d35e41bfeb5be04373ac14750cdf6114af7b578 |

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
| 1 | entry20-exit55-ema200-atr14x2 | 1 | -5.2695 | 0 | 0.5270 | 0 | -6.7639 | 0 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 1 | -5.2695 | 0 | 0.5270 | 0 | -6.7639 | 0 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 1 | -19.7035 | 0 | 1.9703 | 0 | -21.1761 | 0 | no |

### M1-M18 training (M19-M24)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x3 | 3 | -2.7139 | 0.7942 | 3.9565 | 17.0688 | -4.2121 | 0.7028 | no |
| 2 | entry55-exit55-ema200-atr14x4 | 3 | -7.4064 | 0.4384 | 3.9565 | 12.3764 | -8.8975 | 0.3721 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 4 | -11.0896 | 0.4147 | 7.5782 | 14.6910 | -12.5752 | 0.3726 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x3 | 6 | -23.1896 | 0.1842 | 13.9138 | 15.6600 | -24.6570 | 0.1680 | no |
| 2 | entry55-exit55-ema200-atr14x3 | 4 | -23.5818 | 0.2499 | 9.4327 | 24.1069 | -25.0486 | 0.2297 | no |
| 3 | entry20-exit55-ema200-atr14x2 | 7 | -23.4451 | 0.1198 | 16.4116 | 11.7732 | -24.9121 | 0.1065 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 8 | -20.4517 | 0.3769 | 16.3614 | 19.2468 | -21.9233 | 0.3569 | no |
| 2 | entry20-exit55-ema200-atr14x2 | 13 | -19.7274 | 0.3211 | 25.6457 | 12.2196 | -21.2000 | 0.3000 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 7 | -28.1128 | 0.3315 | 19.6790 | 23.4535 | -29.5728 | 0.3133 | no |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 9 | -18.7148 | 0.3701 | 16.8433 | 17.0627 | -20.1890 | 0.3488 | no |
| 2 | entry20-exit55-ema200-atr14x2 | 15 | -17.5509 | 0.3154 | 26.3264 | 10.6368 | -19.0268 | 0.2927 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 8 | -27.9381 | 0.3039 | 22.3505 | 20.3121 | -29.3984 | 0.2865 | no |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 10 | 55.0637 | 2.1651 | 33.2883 | 103.7149 | 53.4787 | 2.1085 | no |
| 2 | entry20-exit55-ema200-atr14x4 | 16 | 38.7577 | 1.8936 | 47.4194 | 82.9312 | 37.1972 | 1.8354 | no |
| 3 | entry55-exit55-ema200-atr14x2 | 13 | 18.4682 | 1.7423 | 16.8433 | 29.3635 | 16.9382 | 1.6515 | no |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 11 | 56.1701 | 2.3073 | 33.2883 | 93.8202 | 54.5834 | 2.2446 | yes |
| 2 | entry55-exit55-ema200-atr14x2 | 15 | 20.7985 | 1.9646 | 16.8433 | 25.3611 | 19.2651 | 1.8551 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 18 | 37.0312 | 1.8905 | 47.4194 | 73.7236 | 35.4733 | 1.8309 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 11 | 56.1701 | 2.3073 | 33.2883 | 93.8202 | 54.5834 | 2.2446 | yes |
| 2 | entry55-exit55-ema200-atr14x2 | 15 | 20.7985 | 1.9646 | 16.8433 | 25.3611 | 19.2651 | 1.8551 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 19 | 34.5186 | 1.8638 | 47.4194 | 69.7808 | 32.9645 | 1.8023 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL -2.4009 (-0.4802%), stress net PnL -3.8996 (-0.7799%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 5 | -31.9139 | 0 | 15.9570 | -159.5697 | -2.4009 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 5 | -33.3682 | 0 | 16.6841 | -166.8412 | -3.8996 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 3 | -32.8947 | 0 | 9.8684 | -98.6841 | -2.4009 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 3 | -34.3475 | 0 | 10.3043 | -103.0426 | -3.8996 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 4 | -22.8313 | 0.2560 | 12.2749 | -91.3253 | -2.4009 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 4 | -24.2993 | 0.2351 | 12.7071 | -97.1971 | -3.8996 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 60.8297 (12.1659%), stress net PnL 59.2361 (11.8472%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 1 | -86.1855 | 0 | 8.6186 | -86.1855 | 60.8297 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 1 | -87.5583 | 0 | 8.7558 | -87.5583 | 59.2361 | no | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | -90.9398 | 0 | 9.0940 | -90.9398 | 60.8297 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -92.3054 | 0 | 9.2305 | -92.3054 | 59.2361 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -47.3897 | 0 | 9.4779 | -94.7793 | 60.8297 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -48.8207 | 0 | 9.7641 | -97.6414 | 59.2361 | no | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 30.4191 (6.0838%), stress net PnL 28.8711 (5.7742%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x3 | Base | 4 | -27.0828 | 0.3792 | 16.3666 | -108.3313 | 30.4191 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 4 | -28.5444 | 0.3612 | 16.7899 | -114.1776 | 28.8711 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 3 | -34.1541 | 0.3924 | 15.8161 | -102.4624 | 30.4191 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 3 | -35.6051 | 0.3767 | 16.0983 | -106.8152 | 28.8711 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 5 | -15.9338 | 0.5540 | 16.2556 | -79.6692 | 30.4191 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 5 | -17.4122 | 0.5278 | 16.8032 | -87.0608 | 28.8711 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL -11.4931 (-2.2986%), stress net PnL -12.9781 (-2.5956%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 1 | -4.8195 | 0 | 0.4820 | -4.8195 | -11.4931 | yes | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -6.3145 | 0 | 0.6315 | -6.3145 | -12.9781 | yes | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 2 | -3.4037 | 0 | 0.6807 | -6.8073 | -11.4931 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -4.9008 | 0 | 0.9802 | -9.8016 | -12.9781 | yes | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 1 | -26.7150 | 0 | 2.6715 | -26.7150 | -11.4931 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 1 | -28.1771 | 0 | 2.8177 | -28.1771 | -12.9781 | no | no | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL 1409.3960 (281.8792%), stress net PnL 1405.7765 (281.1553%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 4 | 102.1301 | 8.2930 | 2.1588 | 408.5204 | 1409.3960 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 4 | 100.4744 | 7.8192 | 2.2680 | 401.8977 | 1405.7765 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 6 | 71.0394 | 4.8946 | 5.5936 | 426.2365 | 1409.3960 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 6 | 69.4305 | 4.7101 | 5.7159 | 416.5827 | 1405.7765 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | 50.0694 | 2.5785 | 5.2074 | 200.2775 | 1409.3960 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | 48.4919 | 2.4956 | 5.3301 | 193.9676 | 1405.7765 | no | no | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 79.1283 (15.8257%), stress net PnL 77.5072 (15.5014%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | 67.2341 | ∞ | 0 | 67.2341 | 79.1283 | no | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 65.6308 | ∞ | 0 | 65.6308 | 77.5072 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 2 | 23.2192 | 1.8506 | 5.4595 | 46.4385 | 79.1283 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | 21.6821 | 1.7742 | 5.6015 | 43.3642 | 77.5072 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 2 | 35.9456 | ∞ | 0 | 71.8913 | 79.1283 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 2 | 34.3894 | ∞ | 0 | 68.7788 | 77.5072 | no | no | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -205.1826 (-41.0365%), stress net PnL -206.3766 (-41.2753%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -205.1826 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -206.3766 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -205.1826 | yes | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -206.3766 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | -10.7080 | 0 | 1.0708 | -10.7080 | -205.1826 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | -12.1941 | 0 | 1.2194 | -12.1941 | -206.3766 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL -25.4870 (-5.0974%), stress net PnL -26.9510 (-5.3902%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | -32.6329 | 0 | 3.2633 | -32.6329 | -25.4870 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -34.0862 | 0 | 3.4086 | -34.0862 | -26.9510 | no | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 1 | 1.2156 | ∞ | 0 | 1.2156 | -25.4870 | yes | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -0.2885 | 0 | 0.0288 | -0.2885 | -26.9510 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 2 | -45.1620 | 0 | 9.0324 | -90.3239 | -25.4870 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | -46.5964 | 0 | 9.3193 | -93.1927 | -26.9510 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M18 | — | Fail (stability) | — | — | Fail (stability) | Fail (stability) |
| M1-M24 | Fail (stability) | Fail (stability) | — | — | Fail (stability) | — |
| M1-M30 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M36 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M42 | — | — | Fail (stability) | Fail (stability) | — | Fail (stability) |
| M1-M48 | — | — | Fail | Fail | — | Fail |
| M1-M54 | — | — | Fail | Fail | — | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x2 | 6 | 0 | 100.4744 |
| entry20-exit55-ema200-atr14x2 | 4 | 0 | 69.4305 |
| entry55-exit55-ema200-atr14x3 | 4 | 0 | 48.4919 |
| entry55-exit55-ema200-atr14x4 | 4 | 0 | 65.6308 |
| entry20-exit55-ema200-atr14x3 | 3 | 0 | -24.2993 |
| entry20-exit55-ema200-atr14x4 | 3 | 0 | 21.6821 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x3 | -97.1971 | -3.8996 | -93.2975 | Underperform |
| M1-M18 | entry55-exit55-ema200-atr14x3 | -87.5583 | 59.2361 | -146.7944 | Underperform |
| M1-M24 | entry20-exit55-ema200-atr14x2 | -87.0608 | 28.8711 | -115.9319 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x2 | -6.3145 | -12.9781 | 6.6635 | Beat |
| M1-M36 | entry20-exit55-ema200-atr14x2 | 416.5827 | 1405.7765 | -989.1938 | Underperform |
| M1-M42 | entry55-exit55-ema200-atr14x2 | 68.7788 | 77.5072 | -8.7284 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x2 | 0 | -206.3766 | 206.3766 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x2 | -0.2885 | -26.9510 | 26.6625 | Beat |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x3 had 4 OOS trades (minimum 10), net expectancy -24.299273561028152746392121582, PF 0.2350980264300843950904440903, net PnL -97.19709424411261098556848633 vs benchmark -3.8995681969537476637521751298; parameter stability fail.
- M1-M18: best top-3 stress run entry20-exit55-ema200-atr14x3 had 2 OOS trades (minimum 10), net expectancy -48.820712884644145920140430795, PF 0, net PnL -97.64142576928829184028086159 vs benchmark 59.236059399548046917034326913; parameter stability fail.
- M1-M24: best top-3 stress run entry20-exit55-ema200-atr14x2 had 5 OOS trades (minimum 10), net expectancy -17.412150978668085596913063318, PF 0.5278337779575327808034547063, net PnL -87.06075489334042798456531659 vs benchmark 28.871108176894961440572554878; parameter stability fail.
- M1-M30: best top-3 stress run entry20-exit55-ema200-atr14x2 had 2 OOS trades (minimum 10), net expectancy -4.900814362690688352400371018, PF 0, net PnL -9.801628725381376704800742036 vs benchmark -12.978068519018073914216347456; parameter stability fail.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x2 had 4 OOS trades (minimum 10), net expectancy 100.47442981810090938682344391, PF 7.819229163182498258248733802, net PnL 401.89771927240363754729377565 vs benchmark 1405.7765118156465580507852634; parameter stability fail.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 65.630810412741446530808154059, PF infinite, net PnL 65.630810412741446530808154059 vs benchmark 77.507173890109046965962385034; parameter stability fail.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x2 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -206.37658774074196425621882329; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x2 had 1 OOS trades (minimum 10), net expectancy -0.2884879866870212209981983290, PF 0, net PnL -0.2884879866870212209981983290 vs benchmark -26.950970684220802767007893782; parameter stability pass.

Successful runs: none.

