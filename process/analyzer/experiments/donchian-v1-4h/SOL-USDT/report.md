# Donchian Trend-Following — SOL-USDT (4hour, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 63fbbe83a517aa3429ffaac428c32b4a981954e05359494fdffcb8b4f15841ee |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M6 | 0..1094 | 1094 | M7-M11 | 1094..2005 | 911 | 2025-07-24T00:00:00Z | 2026-01-22T04:00:00Z | 2026-01-22T08:00:00Z | 2026-06-23T00:00:00Z |
| M1-M7 | 0..1276 | 1276 | M8-M12 | 1276..2188 | 912 | 2025-07-24T00:00:00Z | 2026-02-21T12:00:00Z | 2026-02-21T16:00:00Z | 2026-07-23T12:00:00Z |
| M1-M8 | 0..1458 | 1458 | M9-M13 | 1458..2370 | 912 | 2025-07-24T00:00:00Z | 2026-03-23T20:00:00Z | 2026-03-24T00:00:00Z | 2026-08-22T20:00:00Z |
| M1-M9 | 0..1641 | 1641 | M10-M14 | 1641..2552 | 911 | 2025-07-24T00:00:00Z | 2026-04-23T08:00:00Z | 2026-04-23T12:00:00Z | 2026-09-22T04:00:00Z |
| M1-M10 | 0..1823 | 1823 | M11-M15 | 1823..2735 | 912 | 2025-07-24T00:00:00Z | 2026-05-23T16:00:00Z | 2026-05-23T20:00:00Z | 2026-10-22T16:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy | donchian-v1 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT, BNB-USDT, XRP-USDT (5) |
| Entry | Close > prior Donchian high (N ∈ [20, 55]) and close > EMA(200) |
| Exit | ATR(14) trailing stop (highest high since entry − multiplier × ATR) or close < EMA(200) at the next open |
| Grid | entry N [20, 55] × Donchian exit N 55 × ATR multiplier [2, 3, 4] × EMA 200 × ATR 14 = 6 configs (Donchian exit N is fixed metadata and is not an active exit in this variant) |
| Walk-forward | Expanding training windows M1-M6 through M1-M10; each tested on the next 5 months |
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
- There is no minimum-trade gate.
- A run passes only when stress OOS net expectancy > 0, stress OOS PF > 1, and stress OOS net PnL > the stress buy-and-hold benchmark.
- A symbol passes when any top-3 run in any window passes. The experiment passes only when all 5 symbols pass.
- The benchmark buys at the first OOS open and sells at the last OOS close with the same half-budget and cost scenario.

## 3. Validation

Top 3 configs per expanding training window, selected on base-cost training metrics.

### M1-M6 training (M7-M11)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x4 | 5 | 7.5876 | 2.3825 | 2.6497 | 10.3152 | 6.0739 | 1.9518 |
| 2 | entry20-exit55-ema200-atr14x4 | 6 | 5.9036 | 1.8221 | 3.9911 | 11.1127 | 4.3925 | 1.5519 |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 4.8527 | 1.8172 | 2.4670 | 9.9558 | 3.3431 | 1.5004 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x4 | 5 | 7.5876 | 2.3825 | 2.6497 | 10.3152 | 6.0739 | 1.9518 |
| 2 | entry20-exit55-ema200-atr14x4 | 6 | 5.9036 | 1.8221 | 3.9911 | 11.1127 | 4.3925 | 1.5519 |
| 3 | entry55-exit55-ema200-atr14x3 | 6 | 4.8527 | 1.8172 | 2.4670 | 9.9558 | 3.3431 | 1.5004 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 8 | -0.1302 | 0.9842 | 5.3252 | 7.9972 | -1.6322 | 0.8216 |
| 2 | entry55-exit55-ema200-atr14x4 | 7 | -0.4396 | 0.9550 | 3.9516 | 8.9710 | -1.9412 | 0.8209 |
| 3 | entry20-exit55-ema200-atr14x4 | 9 | -1.4128 | 0.8606 | 5.7510 | 8.2599 | -2.9130 | 0.7387 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 9 | -2.7049 | 0.7268 | 7.5336 | 7.5082 | -4.2031 | 0.6139 |
| 2 | entry55-exit55-ema200-atr14x4 | 8 | -4.0228 | 0.6701 | 6.7557 | 8.5557 | -5.5190 | 0.5851 |
| 3 | entry55-exit55-ema200-atr14x2 | 12 | -4.1090 | 0.4603 | 7.2304 | 3.8898 | -5.6051 | 0.3630 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry55-exit55-ema200-atr14x3 | 10 | -0.4340 | 0.9513 | 7.5336 | 7.0891 | -1.9356 | 0.8024 |
| 2 | entry55-exit55-ema200-atr14x4 | 9 | -2.3120 | 0.7867 | 6.7557 | 7.7369 | -3.8107 | 0.6778 |
| 3 | entry20-exit55-ema200-atr14x3 | 21 | -4.6977 | 0.5381 | 16.0172 | 4.1703 | -6.1929 | 0.4554 |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -223.8120 (-44.7624%), stress net PnL -224.9781 (-44.9956%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 4 | -14.6864 | 0.1622 | 7.0120 | -58.7455 | -223.8120 | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 4 | -16.1666 | 0.1323 | 7.4522 | -64.6662 | -224.9781 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 14 | -10.8403 | 0.1706 | 16.4165 | -151.7646 | -223.8120 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | -12.3263 | 0.1467 | 18.0443 | -172.5681 | -224.9781 | yes | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | -8.3640 | 0.3742 | 5.3460 | -33.4558 | -223.8120 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | -9.8536 | 0.3191 | 5.7886 | -39.4146 | -224.9781 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -60.3744 (-12.0749%), stress net PnL -61.7859 (-12.3572%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x4 | Base | 4 | -8.7920 | 0.3884 | 5.7501 | -35.1682 | -60.3744 | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 4 | -10.2811 | 0.3193 | 6.0419 | -41.1244 | -61.7859 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 12 | -9.4678 | 0.2707 | 13.6971 | -117.7794 | -60.3744 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 12 | -10.9558 | 0.2292 | 14.8784 | -137.1317 | -61.7859 | no | Fail |
| entry55-exit55-ema200-atr14x3 | Base | 4 | 0.8474 | 1.0830 | 4.0840 | 3.3895 | -60.3744 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 4 | -0.6562 | 0.9401 | 4.3784 | -2.6246 | -61.7859 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 11.4945 (2.2989%), stress net PnL 9.9750 (1.9950%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 5 | 20.9842 | 5.0656 | 2.3302 | 104.9208 | 11.4945 | yes | — |
| entry55-exit55-ema200-atr14x3 | Stress | 5 | 19.4504 | 4.3800 | 2.4770 | 97.2520 | 9.9750 | yes | Pass |
| entry55-exit55-ema200-atr14x4 | Base | 5 | 11.7040 | 2.5935 | 2.9105 | 58.5200 | 11.4945 | yes | — |
| entry55-exit55-ema200-atr14x4 | Stress | 5 | 10.1842 | 2.2835 | 3.0564 | 50.9208 | 9.9750 | yes | Pass |
| entry20-exit55-ema200-atr14x4 | Base | 14 | -3.3100 | 0.7310 | 14.1743 | -46.3403 | 11.4945 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 14 | -4.8073 | 0.6430 | 15.2046 | -67.3022 | 9.9750 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 176.0719 (35.2144%), stress net PnL 174.3051 (34.8610%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 6 | 18.4272 | 6.4830 | 1.8915 | 106.8832 | 176.0719 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 6 | 16.8973 | 5.1144 | 2.3007 | 96.2069 | 174.3051 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 5 | 16.1193 | 6.5021 | 0.7453 | 101.9021 | 176.0719 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 5 | 14.5929 | 5.1385 | 0.8938 | 92.7355 | 174.3051 | no | Fail |
| entry55-exit55-ema200-atr14x2 | Base | 11 | 2.9214 | 1.5029 | 4.0729 | 28.4554 | 176.0719 | no | — |
| entry55-exit55-ema200-atr14x2 | Stress | 11 | 1.4148 | 1.2182 | 4.3784 | 10.3856 | 174.3051 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 108.1846 (21.6369%), stress net PnL 106.5198 (21.3040%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry55-exit55-ema200-atr14x3 | Base | 7 | 9.0192 | 2.3266 | 4.0682 | 63.1344 | 108.1846 | no | — |
| entry55-exit55-ema200-atr14x3 | Stress | 7 | 7.5034 | 1.9545 | 4.6238 | 52.5238 | 106.5198 | no | Fail |
| entry55-exit55-ema200-atr14x4 | Base | 5 | 20.5009 | 7.9977 | 0.7537 | 102.5046 | 108.1846 | no | — |
| entry55-exit55-ema200-atr14x4 | Stress | 5 | 18.9679 | 6.3792 | 0.9025 | 94.8394 | 106.5198 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 10 | 6.2952 | 1.8943 | 3.2136 | 62.9520 | 108.1846 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 10 | 4.7835 | 1.5920 | 3.9200 | 47.8349 | 106.5198 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 | entry55-exit55-ema200-atr14x2 | entry55-exit55-ema200-atr14x3 | entry55-exit55-ema200-atr14x4 |
|---|---|---|---|---|---|
| M1-M6 | — | Fail | — | Fail | Fail |
| M1-M7 | — | Fail | — | Fail | Fail |
| M1-M8 | — | Fail | — | Pass | Pass |
| M1-M9 | — | — | Fail | Fail | Fail |
| M1-M10 | Fail | — | — | Fail | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry55-exit55-ema200-atr14x3 | 5 | 1 | 19.4504 |
| entry55-exit55-ema200-atr14x4 | 5 | 1 | 18.9679 |
| entry20-exit55-ema200-atr14x4 | 3 | 0 | -4.8073 |
| entry20-exit55-ema200-atr14x3 | 1 | 0 | 4.7835 |
| entry55-exit55-ema200-atr14x2 | 1 | 0 | 1.4148 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry55-exit55-ema200-atr14x3 | -39.4146 | -224.9781 | 185.5635 | Beat |
| M1-M7 | entry55-exit55-ema200-atr14x3 | -2.6246 | -61.7859 | 59.1613 | Beat |
| M1-M8 | entry55-exit55-ema200-atr14x3 | 97.2520 | 9.9750 | 87.2770 | Beat |
| M1-M9 | entry55-exit55-ema200-atr14x3 | 96.2069 | 174.3051 | -78.0983 | Underperform |
| M1-M10 | entry55-exit55-ema200-atr14x4 | 94.8394 | 106.5198 | -11.6804 | Underperform |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M8: entry55-exit55-ema200-atr14x3 passed OOS stress with 5 trades, net expectancy 19.45039163086717499193802955, PF 4.3799896526610931020038831557, net PnL 97.25195815433587495969014775 > benchmark 9.974956255468066491688538935.
- M1-M8: entry55-exit55-ema200-atr14x4 passed OOS stress with 5 trades, net expectancy 10.184160276334634156586402352, PF 2.2834697416629245498072477004, net PnL 50.920801381673170782932011761 > benchmark 9.974956255468066491688538935.

Successful runs:

| Window | Config | OOS trades | Stress expectancy | Stress PF | Stress net PnL | Benchmark stress net PnL |
|---|---|---:|---:|---:|---:|---:|
| M1-M8 | entry55-exit55-ema200-atr14x3 | 5 | 19.4504 | 4.3800 | 97.2520 | 9.9750 |
| M1-M8 | entry55-exit55-ema200-atr14x4 | 5 | 10.1842 | 2.2835 | 50.9208 | 9.9750 |

