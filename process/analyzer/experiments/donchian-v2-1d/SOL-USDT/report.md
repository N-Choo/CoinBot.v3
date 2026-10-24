# Donchian Trend-Following — SOL-USDT (1day, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 1day |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 1824 |
| SHA-256 | ca1a218ff0f253bcd5099f00f9905547df2e29fc0dc238393f403d14defdff3c |

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
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — | no |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — | no |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x2 | 1 | 11.2063 | ∞ | 0 | 0 | 9.6872 | ∞ | no |
| 2 | entry20-exit55-ema200-atr14x2 | 2 | -9.3406 | 0.3750 | 2.9556 | 20.5469 | -10.8288 | 0.3091 | no |
| 3 | entry55-exit55-ema200-atr14x3 | 1 | -1.1650 | 0 | 0.1165 | 0 | -2.6655 | 0 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 4 | 310.0474 | 14.6324 | 2.3547 | 322.2233 | 308.0793 | 14.1319 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 5 | 138.2901 | 33.3575 | 1.2362 | 76.1123 | 136.5801 | 29.0550 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 5 | 247.0499 | 13.8788 | 9.3945 | 257.4499 | 245.1765 | 13.2250 | yes |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 8 | 147.5519 | 8.8304 | 9.3945 | 161.3436 | 145.8280 | 8.3127 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 5 | 229.4444 | 7.2369 | 4.3704 | 262.2852 | 227.5974 | 7.0476 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 105.0518 | 8.6393 | 3.9066 | 70.4758 | 103.3917 | 8.1394 | yes |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 10 | 120.4416 | 6.0396 | 9.3945 | 129.4121 | 118.7585 | 5.7666 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 7 | 167.3176 | 5.3031 | 4.1500 | 186.6697 | 165.5640 | 5.1721 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 9 | 71.8346 | 5.0244 | 5.4068 | 50.1227 | 70.2244 | 4.7639 | yes |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 13 | 91.9604 | 5.5024 | 5.0061 | 99.4642 | 90.3200 | 5.2367 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 10 | 105.0431 | 3.6730 | 9.2515 | 131.4951 | 103.3830 | 3.5660 | yes |
| 3 | entry20-exit55-ema200-atr14x2 | 21 | 31.0517 | 4.2109 | 6.8432 | 18.5665 | 29.5028 | 3.8590 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 13 | 91.9604 | 5.5024 | 5.0061 | 99.4642 | 90.3200 | 5.2367 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 10 | 105.0431 | 3.6730 | 9.2515 | 131.4951 | 103.3830 | 3.5660 | yes |
| 3 | entry20-exit55-ema200-atr14x2 | 21 | 31.0517 | 4.2109 | 6.8432 | 18.5665 | 29.5028 | 3.8590 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL -117.9649 (-23.5930%), stress net PnL -119.2900 (-23.8580%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -117.9649 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -119.2900 | yes | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -117.9649 | yes | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -119.2900 | yes | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -117.9649 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -119.2900 | yes | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 174.1603 (34.8321%), stress net PnL 172.3964 (34.4793%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 3 | 33.3894 | 3.2722 | 4.4084 | 181.0153 | 174.1603 | yes | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | 31.8370 | 3.0982 | 4.5520 | 174.7344 | 172.3964 | yes | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 3 | 20.0499 | 2.3808 | 4.3093 | 140.9968 | 174.1603 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 3 | 18.5176 | 2.1947 | 4.4530 | 134.7760 | 172.3964 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 2 | 7.8244 | 1.3631 | 4.3093 | 193.5057 | 174.1603 | yes | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | 6.3104 | 1.2834 | 4.4530 | 188.7082 | 172.3964 | yes | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 2196.8984 (439.3797%), stress net PnL 2192.0958 (438.4192%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x2 | Base | 7 | 43.6463 | 3.2284 | 11.9871 | 305.5241 | 2196.8984 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 7 | 42.0785 | 3.0830 | 12.3818 | 294.5494 | 2192.0958 | no | no | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 8 | 43.3466 | 4.6175 | 7.8073 | 346.7729 | 2196.8984 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 8 | 41.7792 | 4.2857 | 8.2019 | 334.2340 | 2192.0958 | no | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | 130.6752 | 26.8711 | 1.3785 | 522.7010 | 2196.8984 | no | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | 128.9767 | 24.8009 | 1.4827 | 515.9068 | 2192.0958 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL 27.9597 (5.5919%), stress net PnL 26.4154 (5.2831%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 1 | -92.9675 | 0 | 9.7906 | -97.9063 | 27.9597 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | -94.3301 | 0 | 10.0764 | -100.7637 | 26.4154 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 1 | -61.1397 | 0 | 6.6078 | -66.0784 | 27.9597 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 1 | -62.5501 | 0 | 6.8984 | -68.9837 | 26.4154 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 3 | -22.2395 | 0 | 7.1657 | -71.6573 | 27.9597 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 3 | -23.7083 | 0 | 7.7559 | -77.5586 | 26.4154 | no | yes | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL -90.4036 (-18.0807%), stress net PnL -91.7700 (-18.3540%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 2 | -4.9490 | 0.8878 | 8.1831 | -9.8980 | -90.4036 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | -6.4438 | 0.8562 | 8.3227 | -12.8876 | -91.7700 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 2 | -4.9490 | 0.8878 | 8.1831 | -9.8980 | -90.4036 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 2 | -6.4438 | 0.8562 | 8.3227 | -12.8876 | -91.7700 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 3 | -3.2718 | 0.9058 | 5.6329 | -9.8155 | -90.4036 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 3 | -4.7692 | 0.8663 | 5.7746 | -14.3075 | -91.7700 | yes | yes | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 177.9913 (35.5983%), stress net PnL 176.2217 (35.2443%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 4 | -16.3056 | 0.1841 | 7.9936 | -65.2222 | 177.9913 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 4 | -17.7833 | 0.1564 | 8.4323 | -71.1333 | 176.2217 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 4 | -37.5105 | 0 | 15.0042 | -150.0420 | 177.9913 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 4 | -38.9564 | 0 | 15.5826 | -155.8256 | 176.2217 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | -22.5824 | 0.1476 | 10.5969 | -90.3295 | 177.9913 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | -24.0507 | 0.1279 | 11.0317 | -96.2028 | 176.2217 | no | yes | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -269.3588 (-53.8718%), stress net PnL -270.4564 (-54.0913%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -269.3588 | yes | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -270.4564 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -269.3588 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -270.4564 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -269.3588 | yes | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -270.4564 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 119.6868 (23.9374%), stress net PnL 118.0047 (23.6009%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 1 | 65.9576 | ∞ | 0 | 65.9576 | 119.6868 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 64.3562 | ∞ | 0 | 64.3562 | 118.0047 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | 65.9576 | ∞ | 0 | 65.9576 | 119.6868 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 64.3562 | ∞ | 0 | 64.3562 | 118.0047 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 3 | -10.9703 | 0.2708 | 4.5131 | -32.9110 | 119.6868 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | -12.4561 | 0.2226 | 4.8067 | -37.3683 | 118.0047 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | Fail (stability) | — | — | — |
| M1-M18 | Fail (stability) | Fail (stability) | Fail (stability) | — | — | — |
| M1-M24 | Fail (stability) | — | — | Fail (stability) | Fail (stability) | — |
| M1-M30 | — | — | Fail | — | Fail | Fail |
| M1-M36 | — | — | Fail | — | Fail | Fail |
| M1-M42 | — | — | Fail | — | Fail | Fail |
| M1-M48 | Fail | — | Fail | — | — | Fail |
| M1-M54 | Fail | — | Fail | — | — | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x4 | 7 | 0 | 64.3562 |
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 41.7792 |
| entry55-exit55-ema200-atr14x4 | 5 | 0 | 64.3562 |
| entry55-exit55-ema200-atr14x3 | 4 | 0 | 128.9767 |
| entry20-exit55-ema200-atr14x3 | 2 | 0 | 18.5176 |
| entry55-exit55-ema200-atr14x2 | 1 | 0 | 42.0785 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x4 | 0 | -119.2900 | 119.2900 | Beat |
| M1-M18 | entry20-exit55-ema200-atr14x4 | 188.7082 | 172.3964 | 16.3118 | Beat |
| M1-M24 | entry55-exit55-ema200-atr14x3 | 515.9068 | 2192.0958 | -1676.1891 | Underperform |
| M1-M30 | entry55-exit55-ema200-atr14x3 | -68.9837 | 26.4154 | -95.3991 | Underperform |
| M1-M36 | entry55-exit55-ema200-atr14x4 | -12.8876 | -91.7700 | 78.8824 | Beat |
| M1-M42 | entry20-exit55-ema200-atr14x4 | -71.1333 | 176.2217 | -247.3550 | Underperform |
| M1-M48 | entry20-exit55-ema200-atr14x2 | 0 | -270.4564 | 270.4564 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 64.3562 | 118.0047 | -53.6485 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -119.28996607748616318514550973; parameter stability fail.
- M1-M18: best top-3 stress run entry20-exit55-ema200-atr14x2 had 3 OOS trades (minimum 10), net expectancy 31.837024521592631721223962117, PF 3.0982132920427522469630248363, net PnL 174.73437239321634806047817747 vs benchmark 172.39642890399478536176552751; parameter stability fail.
- M1-M24: best top-3 stress run entry55-exit55-ema200-atr14x3 had 4 OOS trades (minimum 10), net expectancy 128.97668909706921492972859672, PF 24.800910673043693101772462829, net PnL 515.90675638827685971891438688 vs benchmark 2192.0958234342182281444754329; parameter stability fail.
- M1-M30: best top-3 stress run entry20-exit55-ema200-atr14x4 had 3 OOS trades (minimum 10), net expectancy -23.708338087675731177512896342, PF 0, net PnL -77.558632351978476426292982788 vs benchmark 26.415434548570190917256448313; parameter stability pass.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x3 had 3 OOS trades (minimum 10), net expectancy -4.769167771238540720913584345, PF 0.8662866229185170759999833673, net PnL -14.307503313715622162740753035 vs benchmark -91.77002726271507754527235633; parameter stability pass.
- M1-M42: best top-3 stress run entry20-exit55-ema200-atr14x4 had 4 OOS trades (minimum 10), net expectancy -17.783317639380303192463296134, PF 0.1564184288128875105691196045, net PnL -71.133270557521212769853184536 vs benchmark 176.22168028825028561385007470; parameter stability pass.
- M1-M48: best top-3 stress run entry20-exit55-ema200-atr14x2 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -270.45636716224951519069166128; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 64.356242884139555033757622685, PF infinite, net PnL 64.356242884139555033757622685 vs benchmark 118.00470601905647222867766675; parameter stability pass.

Successful runs: none.

