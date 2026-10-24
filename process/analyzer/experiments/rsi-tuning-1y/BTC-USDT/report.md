# RSI Per-Symbol Tuning — BTC-USDT (4hour, 1 year)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2025-10-23T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | 88ea94629ad69275983736e0c136c5deee10e17d5f73bfe5396236f3d3de739c |

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
| top-1 | rsi21-long65-short40 (period 21, long 65, short 40) | 2 | 20.6234 | ∞ | 0 |
| top-2 | rsi21-long70-short40 (period 21, long 70, short 40) | 2 | 11.9679 | 7.1048 | 0.3815 |
| top-3 | rsi14-long70-short50 (period 14, long 70, short 50) | 4 | 12.0263 | 4.6714 | 1.2352 |

## 3. Validation — M1-M4 development

All 24 candidates, base costs. Frozen configs are marked **top-1**, **top-2**, and **top-3**.

| Label | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Top |
|---|---|---:|---:|---:|---:|---:|---|
| rsi21-long65-short40 | period 21 / long 65 / short 40 | 2 | 20.6234 | ∞ | 0 | 15.5841 | **top-1** |
| rsi21-long70-short40 | period 21 / long 70 / short 40 | 2 | 11.9679 | 7.1048 | 0.3815 | 15.8887 | **top-2** |
| rsi14-long70-short50 | period 14 / long 70 / short 50 | 4 | 12.0263 | 4.6714 | 1.2352 | 15.6148 | **top-3** |
| rsi21-long65-short50 | period 21 / long 65 / short 50 | 4 | 8.7453 | 2.5437 | 2.1425 | 16.6743 |  |
| rsi14-long70-short45 | period 14 / long 70 / short 45 | 4 | 6.3587 | 2.0409 | 2.3273 | 14.9426 |  |
| rsi14-long70-short40 | period 14 / long 70 / short 40 | 4 | 3.2476 | 1.3689 | 3.3599 | 15.5835 |  |
| rsi21-long65-short45 | period 21 / long 65 / short 45 | 4 | 2.9148 | 1.2884 | 3.8425 | 16.7942 |  |
| rsi14-long60-short50 | period 14 / long 60 / short 50 | 21 | 2.6729 | 1.3797 | 9.0741 | 6.2205 |  |
| rsi21-long55-short40 | period 21 / long 55 / short 40 | 10 | 2.1013 | 1.1575 | 6.8756 | 11.9361 |  |
| rsi21-long70-short50 | period 21 / long 70 / short 50 | 4 | 1.3737 | 1.1244 | 4.2091 | 16.4183 |  |
| rsi14-long55-short40 | period 14 / long 55 / short 40 | 17 | 0.0040 | 1.0004 | 11.6551 | 7.8855 |  |
| rsi21-long55-short45 | period 21 / long 55 / short 45 | 17 | -0.7578 | 0.9305 | 12.5767 | 7.8231 |  |
| rsi14-long60-short45 | period 14 / long 60 / short 45 | 20 | -0.9703 | 0.9095 | 14.2817 | 6.8382 |  |
| rsi14-long60-short40 | period 14 / long 60 / short 40 | 15 | -1.7483 | 0.8626 | 11.0074 | 8.3746 |  |
| rsi21-long55-short50 | period 21 / long 55 / short 50 | 27 | -1.8091 | 0.8065 | 18.9818 | 5.0545 |  |
| rsi14-long55-short50 | period 14 / long 55 / short 50 | 37 | -1.8180 | 0.7805 | 23.1564 | 4.0281 |  |
| rsi14-long55-short45 | period 14 / long 55 / short 45 | 28 | -1.8683 | 0.8106 | 21.5193 | 5.2946 |  |
| rsi21-long70-short45 | period 21 / long 70 / short 45 | 4 | -4.4587 | 0.7117 | 5.9248 | 16.5187 |  |
| rsi14-long65-short50 | period 14 / long 65 / short 50 | 15 | -4.6317 | 0.5240 | 11.7858 | 5.2131 |  |
| baseline | period 21 / long 60 / short 50 | 12 | -8.8329 | 0.3741 | 15.4059 | 6.9330 |  |
| rsi21-long60-short40 | period 21 / long 60 / short 40 | 6 | -8.8898 | 0.4667 | 9.6608 | 11.2920 |  |
| rsi14-long65-short45 | period 14 / long 65 / short 45 | 16 | -10.9694 | 0.2718 | 19.3079 | 5.4119 |  |
| rsi14-long65-short40 | period 14 / long 65 / short 40 | 11 | -11.8397 | 0.2852 | 15.4016 | 7.3508 |  |
| rsi21-long60-short45 | period 21 / long 60 / short 45 | 10 | -11.8521 | 0.3019 | 16.1496 | 7.9349 |  |

## 4. Holdout

M5 is the decision partition; M6-M10 is logged for every frozen config and never changes the decision.

### Top-1: rsi21-long65-short40 (period 21, long 65, short 40) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -50.9501 | 0 | 21.2142 | 8.8129 | -203.8005 | -8.3418 |
| Stress | 4 | -52.4679 | 0 | 21.9724 | 8.8421 | -209.8717 | -9.8521 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 7 | -0.9150 | 0.9542 | 10.2930 | 17.7742 | -6.4051 | 92.5466 |
| Stress | 7 | -2.4149 | 0.8854 | 10.8770 | 17.7769 | -16.9043 | 90.9054 |

### Top-2: rsi21-long70-short40 (period 21, long 70, short 40) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 0 | 0 | — | 4.7496 | 0 | 0 | -47.4965 |
| Stress | 0 | 0 | — | 4.9065 | 0 | 0 | -49.0654 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 3 | -5.1586 | 0.8172 | 8.4640 | 43.1973 | -15.4757 | 60.6435 |
| Stress | 3 | -6.6708 | 0.7680 | 8.6264 | 43.2612 | -20.0124 | 59.0501 |

### Top-3: rsi14-long70-short50 (period 14, long 70, short 50) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -30.6502 | 0 | 12.2601 | 8.5255 | -122.6007 | 12.5999 |
| Stress | 4 | -32.1658 | 0 | 12.8663 | 8.5374 | -128.6632 | 11.1210 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 15 | -5.1096 | 0.5621 | 10.2814 | 6.9480 | -76.6436 | 97.4716 |
| Stress | 15 | -6.6065 | 0.4819 | 11.4621 | 6.9575 | -99.0979 | 95.8229 |

M6-M10 is logged for context and does not change the decision.

## 5. Robustness

Using rsi21-long65-short40 (period 21, long 65, short 40) (the first top config that passes M5, else top-1).

**Long vs short (M5, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 2 | -78.0062 | 0 | -39.0031 | 0 | 0 |
| Short | 2 | -125.7942 | -8.3418 | -62.8971 | 0 | 0 |

**BTC regime (M5, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 2 | -78.0062 | -8.3418 | -39.0031 | 0 | 0 |
| Bear | 2 | -125.7942 | 0 | -62.8971 | 0 | 0 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs (M1-M4, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the chosen config.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| -11.8521 | -0.7720 | 20.6234 | 6 |

## 6. Decision

**Verdict: Fail**

- Top-1 rsi21-long65-short40: M5 4 trades, net expectancy -50.950116761718470044249264315, profit factor 0 — fail.
- Top-2 rsi21-long70-short40: M5 0 trades, net expectancy 0, profit factor infinite — fail.
- Top-3 rsi14-long70-short50: M5 4 trades, net expectancy -30.650172973396345210714011532, profit factor 0 — fail.
- Fail: no top config has M5 net expectancy > 0 and profit factor > 1.

The verdict passes if any frozen top config passes M5. M6-M10 is included above for transparency.

