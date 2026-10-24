# Donchian Trend-Following — BTC-USDT (1day, 5 years)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| Date range | 2021-10-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 1824 |
| SHA-256 | 652cffe669ec32a0a4ab541f49d10eded5678070d6e5d6243338ad31230bd0fa |

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
| 1 | entry20-exit55-ema200-atr14x2 | 3 | 23.8730 | 4.8676 | 1.8506 | 33.2772 | 22.3348 | 4.2119 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 3 | 4.3720 | 1.7083 | 1.8506 | 14.4160 | 2.8632 | 1.4117 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 3 | 4.8942 | 1.2937 | 4.9996 | 30.5348 | 3.3846 | 1.1918 | no |

### M1-M24 training (M25-M30)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x2 | 5 | 16.9889 | 3.5013 | 1.8506 | 19.9712 | 15.4612 | 3.0461 | no |
| 2 | entry55-exit55-ema200-atr14x2 | 4 | 3.0301 | 1.6211 | 1.8506 | 10.2816 | 1.5233 | 1.2609 | no |
| 3 | entry20-exit55-ema200-atr14x3 | 5 | 2.1638 | 1.1438 | 4.9996 | 18.3541 | 0.6583 | 1.0413 | no |

### M1-M30 training (M31-M36)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x4 | 7 | 37.0887 | 3.6660 | 7.6013 | 31.5093 | 35.5307 | 3.4446 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 11 | 26.5159 | 3.2800 | 5.0856 | 22.4023 | 24.9738 | 3.0182 | yes |
| 3 | entry20-exit55-ema200-atr14x3 | 9 | 25.2124 | 2.3609 | 4.9645 | 27.8386 | 23.6722 | 2.2244 | yes |

### M1-M36 training (M37-M42)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry20-exit55-ema200-atr14x2 | 15 | 16.8528 | 2.4657 | 5.8672 | 16.9059 | 15.3252 | 2.2504 | yes |
| 2 | entry55-exit55-ema200-atr14x4 | 6 | 20.3076 | 2.2522 | 9.7304 | 28.8774 | 18.7749 | 2.0919 | yes |
| 3 | entry55-exit55-ema200-atr14x2 | 10 | 13.8617 | 2.2215 | 5.4235 | 16.5186 | 12.3386 | 2.0139 | yes |

### M1-M42 training (M43-M48)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 7 | 44.3841 | 4.1930 | 9.7304 | 34.2830 | 42.8152 | 3.9051 | yes |
| 2 | entry55-exit55-ema200-atr14x3 | 10 | 22.3256 | 2.4938 | 5.5556 | 22.5418 | 20.7898 | 2.3197 | yes |
| 3 | entry20-exit55-ema200-atr14x4 | 12 | 29.9682 | 2.4643 | 7.4975 | 28.6502 | 28.4209 | 2.3336 | yes |

### M1-M48 training (M49-M54)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 10 | 27.3693 | 2.7360 | 9.7304 | 25.4570 | 25.8259 | 2.5517 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 25 | 14.8826 | 2.3929 | 6.8727 | 11.4284 | 13.3580 | 2.1621 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 13 | 17.1741 | 2.1963 | 5.5556 | 17.7696 | 15.6461 | 2.0291 | yes |

### M1-M54 training (M55-M60)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF | Stability |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | entry55-exit55-ema200-atr14x4 | 10 | 27.3693 | 2.7360 | 9.7304 | 25.4570 | 25.8259 | 2.5517 | yes |
| 2 | entry20-exit55-ema200-atr14x2 | 25 | 14.8826 | 2.3929 | 6.8727 | 11.4284 | 13.3580 | 2.1621 | yes |
| 3 | entry55-exit55-ema200-atr14x3 | 13 | 17.1741 | 2.1963 | 5.5556 | 17.7696 | 15.6461 | 2.0291 | yes |

## 4. Holdout

Top 3 configs per window on the following 6 months, with buy-and-hold comparison.

### M13-M18 holdout (M1-M12)

Buy & hold benchmark (half budget): base net PnL 216.4672 (43.2934%), stress net PnL 214.6398 (42.9280%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 3 | 23.8730 | 4.8676 | 1.8506 | 71.6189 | 216.4672 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | 22.3348 | 4.2119 | 2.0861 | 67.0045 | 214.6398 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 3 | 4.8942 | 1.2937 | 4.9996 | 14.6826 | 216.4672 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 3 | 3.3846 | 1.1918 | 5.2926 | 10.1537 | 214.6398 | no | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 2 | -38.0063 | 0 | 7.6013 | -18.6199 | 216.4672 | no | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 2 | -39.4515 | 0 | 7.8903 | -23.0987 | 214.6398 | no | no | Fail |

### M19-M24 holdout (M1-M18)

Buy & hold benchmark (half budget): base net PnL 41.9709 (8.3942%), stress net PnL 40.4056 (8.0811%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | 6.6629 | 1.8629 | 1.5011 | 13.3258 | 41.9709 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | 5.1506 | 1.6088 | 1.6473 | 10.3013 | 40.4056 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 1 | -0.9957 | 0 | 0.0996 | -0.9957 | 41.9709 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 1 | -2.4964 | 0 | 0.2496 | -2.4964 | 40.4056 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | -1.9318 | 0.8470 | 2.4720 | -3.8636 | 41.9709 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | -3.4312 | 0.7431 | 2.6193 | -6.8623 | 40.4056 | no | no | Fail |

### M25-M30 holdout (M1-M24)

Buy & hold benchmark (half budget): base net PnL 611.4707 (122.2941%), stress net PnL 609.0499 (121.8100%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 6 | 34.4550 | 3.2000 | 5.4841 | 206.7298 | 611.4707 | no | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 6 | 32.9010 | 3.0075 | 5.7697 | 197.4058 | 609.0499 | no | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 6 | 21.0828 | 2.3462 | 5.4841 | 126.4969 | 611.4707 | no | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 6 | 19.5489 | 2.1928 | 5.7697 | 117.2933 | 609.0499 | no | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 4 | 54.0231 | 3.3618 | 5.0065 | 216.0924 | 611.4707 | no | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 4 | 52.4397 | 3.2229 | 5.1361 | 209.7587 | 609.0499 | no | no | Fail |

### M31-M36 holdout (M1-M30)

Buy & hold benchmark (half budget): base net PnL 2.7243 (0.5449%), stress net PnL 1.2179 (0.2436%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x4 | Base | 3 | -36.9507 | 0 | 11.0852 | -66.9131 | 2.7243 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 3 | -38.3975 | 0 | 11.5192 | -72.8216 | 1.2179 | no | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 4 | -9.7206 | 0.1270 | 4.4482 | -38.1211 | 2.7243 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 4 | -11.2083 | 0.0607 | 4.7729 | -45.5751 | 1.2179 | no | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 4 | -21.1617 | 0 | 8.4647 | -83.8854 | 2.7243 | no | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 4 | -22.6322 | 0 | 9.1271 | -91.2707 | 1.2179 | no | yes | Fail |

### M37-M42 holdout (M1-M36)

Buy & hold benchmark (half budget): base net PnL 147.7589 (29.5518%), stress net PnL 146.0347 (29.2069%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 4 | 17.3795 | 2.5960 | 3.3973 | 69.5181 | 147.7589 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 4 | 15.8512 | 2.3210 | 3.6741 | 63.4047 | 146.0347 | no | yes | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 1 | 160.3589 | ∞ | 0 | 160.3589 | 147.7589 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 1 | 158.6158 | ∞ | 0 | 158.6158 | 146.0347 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 3 | 28.8195 | 4.2483 | 1.8672 | 86.4586 | 147.7589 | no | yes | — |
| entry55-exit55-ema200-atr14x2 | Stress | 3 | 27.2740 | 3.7661 | 2.0058 | 81.8219 | 146.0347 | no | yes | Fail |

### M43-M48 holdout (M1-M42)

Buy & hold benchmark (half budget): base net PnL 117.1822 (23.4364%), stress net PnL 115.5039 (23.1008%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 3 | -12.3320 | 0.3870 | 5.8974 | -36.9960 | 117.1822 | no | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 3 | -13.8157 | 0.3449 | 6.1915 | -41.4472 | 115.5039 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 3 | 0.0026 | 1.0002 | 3.5835 | 0.0079 | 117.1822 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 3 | -1.4996 | 0.8879 | 3.8737 | -4.4989 | 115.5039 | no | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 5 | -10.7490 | 0.4450 | 9.2835 | -53.7450 | 117.1822 | no | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 5 | -12.2351 | 0.3955 | 9.7302 | -61.1755 | 115.5039 | no | yes | Fail |

### M49-M54 holdout (M1-M48)

Buy & hold benchmark (half budget): base net PnL -148.9388 (-29.7878%), stress net PnL -150.2173 (-30.0435%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -148.9388 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -150.2173 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -148.9388 | yes | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -150.2173 | yes | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -148.9388 | yes | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -150.2173 | yes | yes | Fail |

### M55-M60 holdout (M1-M54)

Buy & hold benchmark (half budget): base net PnL 49.2724 (9.8545%), stress net PnL 47.6961 (9.5392%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Stability | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 74.2280 | 49.2724 | yes | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 72.6142 | 47.6961 | yes | yes | Fail |
| entry20-exit55-ema200-atr14x2 | Base | 2 | 0.6899 | 1.0589 | 2.2862 | 1.3798 | 49.2724 | no | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -0.8134 | 0.9347 | 2.4330 | -1.6268 | 47.6961 | no | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 2 | -9.0484 | 0.4850 | 3.4552 | -18.0969 | 49.2724 | no | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 2 | -10.5371 | 0.4241 | 3.6031 | -21.0742 | 47.6961 | no | yes | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates + parameter stability)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|---|
| M1-M12 | Fail (stability) | Fail (stability) | Fail (stability) | — | — | — |
| M1-M18 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M24 | Fail (stability) | Fail (stability) | — | Fail (stability) | — | — |
| M1-M30 | Fail | Fail | Fail | — | — | — |
| M1-M36 | Fail | — | — | Fail | — | Fail |
| M1-M42 | — | — | Fail | — | Fail | Fail |
| M1-M48 | Fail | — | — | — | Fail | Fail |
| M1-M54 | Fail | — | — | — | Fail | Fail |

**Config consistency (across the 8 expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x2 | 7 | 0 | 32.9010 |
| entry20-exit55-ema200-atr14x3 | 4 | 0 | 52.4397 |
| entry55-exit55-ema200-atr14x4 | 4 | 0 | 158.6158 |
| entry20-exit55-ema200-atr14x4 | 3 | 0 | -12.2351 |
| entry55-exit55-ema200-atr14x2 | 3 | 0 | 27.2740 |
| entry55-exit55-ema200-atr14x3 | 3 | 0 | 0 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M12 | entry20-exit55-ema200-atr14x2 | 67.0045 | 214.6398 | -147.6353 | Underperform |
| M1-M18 | entry20-exit55-ema200-atr14x2 | 10.3013 | 40.4056 | -30.1043 | Underperform |
| M1-M24 | entry20-exit55-ema200-atr14x3 | 209.7587 | 609.0499 | -399.2911 | Underperform |
| M1-M30 | entry20-exit55-ema200-atr14x2 | -45.5751 | 1.2179 | -46.7930 | Underperform |
| M1-M36 | entry55-exit55-ema200-atr14x4 | 158.6158 | 146.0347 | 12.5811 | Beat |
| M1-M42 | entry55-exit55-ema200-atr14x3 | -4.4989 | 115.5039 | -120.0028 | Underperform |
| M1-M48 | entry55-exit55-ema200-atr14x3 | 0 | -150.2173 | 150.2173 | Beat |
| M1-M54 | entry55-exit55-ema200-atr14x4 | 72.6142 | 47.6961 | 24.9181 | Beat |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark), the minimum of 10 OOS trades, and the parameter-stability rule.
- M1-M12: best top-3 stress run entry20-exit55-ema200-atr14x2 had 3 OOS trades (minimum 10), net expectancy 22.334842400465618161273355351, PF 4.2119365604619157324188577431, net PnL 67.004527201396854483820066054 vs benchmark 214.63979295333118093670912443; parameter stability fail.
- M1-M18: best top-3 stress run entry20-exit55-ema200-atr14x2 had 2 OOS trades (minimum 10), net expectancy 5.150642224685287597614072309, PF 1.6087582852052445081650581189, net PnL 10.301284449370575195228144618 vs benchmark 40.405571021553317819004328064; parameter stability fail.
- M1-M24: best top-3 stress run entry20-exit55-ema200-atr14x3 had 4 OOS trades (minimum 10), net expectancy 52.439683822243707997777986002, PF 3.222941495035442531907098086, net PnL 209.75873528897483199111194401 vs benchmark 609.04985411560712916187332236; parameter stability fail.
- M1-M30: best top-3 stress run entry20-exit55-ema200-atr14x2 had 4 OOS trades (minimum 10), net expectancy -11.208256118407935957784268355, PF 0.0606706833661885009677111079, net PnL -45.575078126662753929285771000 vs benchmark 1.2179495047163457076913405409; parameter stability pass.
- M1-M36: best top-3 stress run entry55-exit55-ema200-atr14x4 had 1 OOS trades (minimum 10), net expectancy 158.61576247623774802812107139, PF infinite, net PnL 158.61576247623774802812107139 vs benchmark 146.03467135679883637227181987; parameter stability pass.
- M1-M42: best top-3 stress run entry55-exit55-ema200-atr14x3 had 3 OOS trades (minimum 10), net expectancy -1.4996315157870635146230015967, PF 0.887853689800260651317599455, net PnL -4.498894547361190543869004790 vs benchmark 115.50386660664133109990333484; parameter stability pass.
- M1-M48: best top-3 stress run entry55-exit55-ema200-atr14x3 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 0 vs benchmark -150.21726162723600692440854010; parameter stability pass.
- M1-M54: best top-3 stress run entry55-exit55-ema200-atr14x4 had 0 OOS trades (minimum 10), net expectancy 0, PF infinite, net PnL 72.614228922080821000477930954 vs benchmark 47.696132770385779766637890491; parameter stability pass.

Successful runs: none.

