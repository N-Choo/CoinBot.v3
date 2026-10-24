# RSI Per-Symbol Tuning — XRP-USDT (4hour, 1 year)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2025-10-23T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | f77c6c8898b414c7ba8cf4962ab66ef5cbc0983813bba1baefe906cadf39e31d |

| Split | Candle range | First open | Last open |
|---|---|---|---|
| M1-M4 development | 0..729 | 2025-10-23T00:00:00Z | 2026-02-21T08:00:00Z |
| M5 first out-of-sample | 729..912 | 2026-02-21T12:00:00Z | 2026-03-23T20:00:00Z |
| M6-M10 extended OOS log | 912..1824 | 2026-03-24T00:00:00Z | 2026-08-22T20:00:00Z |
| M11-M12 excluded | 1824..2189 | 2026-08-23T00:00:00Z | 2026-10-22T16:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy version | rsi-mom-v1 |
| RSI grid | period 14, 21 × long 55, 60, 65, 70 × short 40, 45, 50 (24 candidates) |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Initial fund | 1000 |
| Simulation window | 100 warm-up candles |
| Frozen top configs | 3 |

Execution assumptions:

- A signal produced at a candle close fills at the next candle open.
- One position per symbol, reversal-only exits.
- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks.
- Ranking uses base costs on M1-M4; stress costs are reported for context.

Development policy:

- Only the max drawdown gate is applied: max drawdown <= 20%. There is no minimum trade-count gate.
- Candidates are ranked by profit factor descending (infinite first, then net expectancy and label) and separately by net expectancy descending (then profit factor and label).
- Combined score = PF rank + expectancy rank; lower is better. The top 3 are frozen before M5 is read.
- M5 pass rule: net expectancy > 0 and PF > 1 with no trade-count gate. The symbol passes if any frozen top config passes M5; M6-M10 is logged only.

| Top | Config | Dev trades | Dev net expectancy | Dev PF | Dev max DD % |
|---|---|---:|---:|---:|---:|
| top-1 | rsi21-long65-short40 (period 21, long 65, short 40) | 3 | 41.9116 | 72.7201 | 5.6656 |
| top-2 | rsi21-long70-short40 (period 21, long 70, short 40) | 3 | 39.5555 | 22.8410 | 5.7014 |
| top-3 | rsi21-long65-short50 (period 21, long 65, short 50) | 4 | 39.4613 | 3.5789 | 5.0209 |

## 3. Validation — M1-M4 development

All 24 candidates, base costs. Frozen configs are marked **top-1**, **top-2**, and **top-3**.

| Label | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Top |
|---|---|---:|---:|---:|---:|---:|---|
| rsi21-long65-short40 | period 21 / long 65 / short 40 | 3 | 41.9116 | 72.7201 | 5.6656 | 24.8501 | **top-1** |
| rsi21-long70-short40 | period 21 / long 70 / short 40 | 3 | 39.5555 | 22.8410 | 5.7014 | 25.9046 | **top-2** |
| rsi21-long65-short50 | period 21 / long 65 / short 50 | 4 | 39.4613 | 3.5789 | 5.0209 | 37.2885 | **top-3** |
| rsi21-long70-short50 | period 21 / long 70 / short 50 | 4 | 37.7012 | 3.4638 | 5.0500 | 37.1045 |  |
| rsi21-long65-short45 | period 21 / long 65 / short 45 | 4 | 25.6432 | 2.3803 | 7.4928 | 37.2796 |  |
| rsi21-long70-short45 | period 21 / long 70 / short 45 | 4 | 23.8626 | 2.2844 | 7.5385 | 37.1208 |  |
| rsi14-long70-short45 | period 14 / long 70 / short 45 | 7 | 19.9251 | 2.5379 | 4.1389 | 23.5416 |  |
| baseline | period 21 / long 60 / short 50 | 9 | 19.6246 | 2.8493 | 3.6557 | 18.3434 |  |
| rsi14-long70-short50 | period 14 / long 70 / short 50 | 8 | 18.0921 | 2.1818 | 9.0791 | 23.0389 |  |
| rsi14-long65-short50 | period 14 / long 65 / short 50 | 10 | 15.6479 | 2.4669 | 5.8131 | 17.1403 |  |
| rsi14-long65-short45 | period 14 / long 65 / short 45 | 9 | 13.3994 | 2.1044 | 4.3406 | 18.2014 |  |
| rsi14-long60-short50 | period 14 / long 60 / short 50 | 14 | 11.3515 | 2.1886 | 5.8131 | 12.6393 |  |
| rsi21-long60-short40 | period 21 / long 60 / short 40 | 6 | 9.6926 | 1.5170 | 11.2485 | 26.4243 |  |
| rsi14-long55-short50 | period 14 / long 55 / short 50 | 24 | 8.1091 | 2.0682 | 6.6460 | 7.2880 |  |
| rsi14-long60-short45 | period 14 / long 60 / short 45 | 13 | 7.5677 | 1.6624 | 5.2169 | 13.0219 |  |
| rsi14-long70-short40 | period 14 / long 70 / short 40 | 7 | 5.7985 | 1.3069 | 9.1937 | 24.2114 |  |
| rsi14-long55-short45 | period 14 / long 55 / short 45 | 21 | 4.9401 | 1.5051 | 7.2076 | 8.0118 |  |
| rsi21-long55-short50 | period 21 / long 55 / short 50 | 17 | 3.4622 | 1.2916 | 13.9615 | 10.5995 |  |
| rsi14-long65-short40 | period 14 / long 65 / short 40 | 9 | 0.4829 | 1.0255 | 10.9668 | 18.7558 |  |
| rsi21-long60-short45 | period 21 / long 60 / short 45 | 10 | -0.0939 | 0.9957 | 10.5962 | 17.5783 |  |
| rsi14-long55-short40 | period 14 / long 55 / short 40 | 19 | -2.2912 | 0.8426 | 16.6286 | 8.1406 |  |
| rsi14-long60-short40 | period 14 / long 60 / short 40 | 13 | -4.0867 | 0.7912 | 16.7141 | 13.4497 |  |
| rsi21-long55-short45 | period 21 / long 55 / short 45 | 16 | -5.7786 | 0.7129 | 21.0367 | 11.4539 |  |
| rsi21-long55-short40 | period 21 / long 55 / short 40 | 10 | -7.6244 | 0.6864 | 24.3092 | 17.1308 |  |

## 4. Holdout

M5 is the decision partition; M6-M10 is logged for every frozen config and never changes the decision.

### Top-1: rsi21-long65-short40 (period 21, long 65, short 40) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 2 | -38.1760 | 0 | 9.2619 | 10.5276 | -76.3519 | -16.2670 |
| Stress | 2 | -39.6917 | 0 | 9.7172 | 10.5826 | -79.3833 | -17.7891 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 11 | -19.1461 | 0.1949 | 21.0607 | 8.1346 | -210.6069 | 183.0053 |
| Stress | 11 | -20.6311 | 0.1793 | 22.6942 | 8.1380 | -226.9417 | 181.2282 |

### Top-2: rsi21-long70-short40 (period 21, long 70, short 40) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 2 | -59.3079 | 0 | 13.4883 | 12.5103 | -118.6159 | -16.2670 |
| Stress | 2 | -60.8266 | 0 | 13.9442 | 12.5970 | -121.6532 | -17.7891 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 9 | -27.0841 | 0.1456 | 24.3757 | 10.1296 | -243.7565 | 183.0053 |
| Stress | 9 | -28.5662 | 0.1349 | 25.7095 | 10.1307 | -257.0954 | 181.2282 |

### Top-3: rsi21-long65-short50 (period 21, long 65, short 50) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 2 | -12.7789 | 0 | 2.5558 | 1.8780 | -25.5578 | 1.3580 |
| Stress | 2 | -14.2817 | 0 | 2.8701 | 1.8949 | -28.5634 | -0.1377 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 13 | -6.2641 | 0.5612 | 11.2611 | 7.3575 | -81.4337 | 183.0053 |
| Stress | 13 | -7.7490 | 0.4977 | 12.1619 | 7.3654 | -100.7368 | 181.2282 |

M6-M10 is logged for context and does not change the decision.

## 5. Robustness

Using rsi21-long65-short40 (period 21, long 65, short 40) (the first top config that passes M5, else top-1).

**Long vs short (M5, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 1 | -27.6483 | 0 | -27.6483 | 0 | 0 |
| Short | 1 | -48.7036 | -16.2670 | -48.7036 | 0 | 0 |

**BTC regime (M5, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 1 | -27.6483 | 0 | -27.6483 | 0 | 0 |
| Bear | 1 | -48.7036 | -16.2670 | -48.7036 | 0 | 0 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs (M1-M4, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the chosen config.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| -0.0939 | 24.7529 | 41.9116 | 6 |

## 6. Decision

**Verdict: Fail**

- Top-1 rsi21-long65-short40: M5 2 trades, net expectancy -38.175956061613996126470942264, profit factor 0 — fail.
- Top-2 rsi21-long70-short40: M5 2 trades, net expectancy -59.30794165273487886188531806, profit factor 0 — fail.
- Top-3 rsi21-long65-short50: M5 2 trades, net expectancy -12.778890967067828000584028296, profit factor 0 — fail.
- Fail: no top config has M5 net expectancy > 0 and profit factor > 1.

The verdict passes if any frozen top config passes M5. M6-M10 is included above for transparency.

