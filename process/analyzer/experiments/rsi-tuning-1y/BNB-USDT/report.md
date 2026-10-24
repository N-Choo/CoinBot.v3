# RSI Per-Symbol Tuning — BNB-USDT (4hour, 1 year)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2025-10-23T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | 62c18621a0f3bac00ba189bd9049f26fe2519d3cdc4049b004488fb1eb7de5cd |

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
| top-1 | rsi14-long70-short40 (period 14, long 70, short 40) | 2 | 35.8709 | ∞ | 0 |
| top-2 | rsi21-long65-short45 (period 21, long 65, short 45) | 2 | 25.3432 | ∞ | 0 |
| top-3 | rsi21-long70-short45 (period 21, long 70, short 45) | 2 | 19.4810 | ∞ | 0 |

## 3. Validation — M1-M4 development

All 24 candidates, base costs. Frozen configs are marked **top-1**, **top-2**, and **top-3**.

| Label | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Top |
|---|---|---:|---:|---:|---:|---:|---|
| rsi14-long70-short40 | period 14 / long 70 / short 40 | 2 | 35.8709 | ∞ | 0 | 9.2750 | **top-1** |
| rsi21-long65-short45 | period 21 / long 65 / short 45 | 2 | 25.3432 | ∞ | 0 | 10.7699 | **top-2** |
| rsi21-long70-short45 | period 21 / long 70 / short 45 | 2 | 19.4810 | ∞ | 0 | 11.0475 | **top-3** |
| rsi21-long65-short40 | period 21 / long 65 / short 40 | 2 | 9.7172 | ∞ | 0 | 2.1481 |  |
| rsi14-long70-short50 | period 14 / long 70 / short 50 | 4 | 8.5847 | 1.9524 | 3.3683 | 17.8488 |  |
| rsi14-long70-short45 | period 14 / long 70 / short 45 | 4 | 4.0609 | 1.3679 | 4.1633 | 18.3031 |  |
| rsi14-long60-short40 | period 14 / long 60 / short 40 | 11 | 3.8362 | 1.2521 | 8.7975 | 14.7040 |  |
| rsi21-long70-short40 | period 21 / long 70 / short 40 | 2 | 3.7505 | ∞ | 0 | 2.2378 |  |
| rsi14-long55-short45 | period 14 / long 55 / short 45 | 18 | 2.8478 | 1.2848 | 9.4731 | 8.8457 |  |
| rsi14-long60-short45 | period 14 / long 60 / short 45 | 13 | 2.7222 | 1.2068 | 8.6607 | 12.1705 |  |
| rsi21-long65-short50 | period 21 / long 65 / short 50 | 4 | 1.3429 | 1.1209 | 4.2316 | 17.4956 |  |
| rsi21-long60-short45 | period 21 / long 60 / short 45 | 6 | 1.1770 | 1.1402 | 2.4887 | 9.3426 |  |
| rsi14-long60-short50 | period 14 / long 60 / short 50 | 22 | 0.8927 | 1.0927 | 9.7723 | 6.9652 |  |
| rsi14-long55-short40 | period 14 / long 55 / short 40 | 16 | 0.6149 | 1.0457 | 12.8852 | 10.2861 |  |
| rsi14-long55-short50 | period 14 / long 55 / short 50 | 34 | -0.0763 | 0.9899 | 13.7483 | 4.6932 |  |
| rsi21-long55-short50 | period 21 / long 55 / short 50 | 16 | -0.7276 | 0.9037 | 8.2285 | 4.4398 |  |
| baseline | period 21 / long 60 / short 50 | 8 | -1.7339 | 0.8280 | 5.2522 | 8.3786 |  |
| rsi21-long70-short50 | period 21 / long 70 / short 50 | 4 | -1.9872 | 0.8481 | 5.0101 | 16.6331 |  |
| rsi21-long55-short45 | period 21 / long 55 / short 45 | 8 | -2.5241 | 0.8040 | 7.9761 | 9.9366 |  |
| rsi14-long65-short50 | period 14 / long 65 / short 50 | 12 | -4.2301 | 0.6248 | 8.8697 | 5.9913 |  |
| rsi21-long60-short40 | period 21 / long 60 / short 40 | 6 | -7.0752 | 0.3364 | 4.4445 | 7.2828 |  |
| rsi14-long65-short40 | period 14 / long 65 / short 40 | 8 | -8.3054 | 0.5115 | 10.7019 | 10.7577 |  |
| rsi21-long55-short40 | period 21 / long 55 / short 40 | 8 | -8.6903 | 0.3586 | 8.7035 | 7.9650 |  |
| rsi14-long65-short45 | period 14 / long 65 / short 45 | 10 | -11.7968 | 0.3452 | 15.0669 | 8.5800 |  |

## 4. Holdout

M5 is the decision partition; M6-M10 is logged for every frozen config and never changes the decision.

### Top-1: rsi14-long70-short40 (period 14, long 70, short 40) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 2 | -46.5672 | 0 | 9.3134 | 26.1470 | -93.1345 | 5.7332 |
| Stress | 2 | -48.1064 | 0 | 9.6213 | 26.2146 | -96.2127 | 4.2440 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 11 | -8.0704 | 0.4487 | 9.3577 | 7.1776 | -88.7748 | 52.5136 |
| Stress | 11 | -9.5712 | 0.3864 | 10.8677 | 7.1839 | -105.2827 | 50.9324 |

### Top-2: rsi21-long65-short45 (period 21, long 65, short 45) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 2 | -46.5672 | 0 | 9.3134 | 26.1470 | -93.1345 | 5.7332 |
| Stress | 2 | -48.1064 | 0 | 9.6213 | 26.2146 | -96.2127 | 4.2440 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 11 | -6.2509 | 0.4756 | 6.8760 | 6.6649 | -68.7595 | 52.5136 |
| Stress | 11 | -7.7510 | 0.4107 | 8.5261 | 6.6725 | -85.2614 | 50.9324 |

### Top-3: rsi21-long70-short45 (period 21, long 70, short 45) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 0 | 0 | — | 4.3090 | 0 | 0 | -43.0897 |
| Stress | 0 | 0 | — | 4.4652 | 0 | 0 | -44.6520 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -10.5672 | 0.3043 | 7.5950 | 10.0288 | -52.8362 | 52.5136 |
| Stress | 5 | -12.0668 | 0.2641 | 8.1985 | 10.0377 | -60.3338 | 50.9324 |

M6-M10 is logged for context and does not change the decision.

## 5. Robustness

Using rsi14-long70-short40 (period 14, long 70, short 40) (the first top config that passes M5, else top-1).

**Long vs short (M5, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 1 | -20.4202 | 0 | -20.4202 | 0 | 0 |
| Short | 1 | -72.7143 | 5.7332 | -72.7143 | 0 | 0 |

**BTC regime (M5, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 1 | -20.4202 | 5.7332 | -20.4202 | 0 | 0 |
| Bear | 1 | -72.7143 | 0 | -72.7143 | 0 | 0 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs (M1-M4, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the chosen config.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| -11.7968 | -2.1223 | 35.8709 | 4 |

## 6. Decision

**Verdict: Fail**

- Top-1 rsi14-long70-short40: M5 2 trades, net expectancy -46.567234222272017554840574875, profit factor 0 — fail.
- Top-2 rsi21-long65-short45: M5 2 trades, net expectancy -46.567234222272017554840574875, profit factor 0 — fail.
- Top-3 rsi21-long70-short45: M5 0 trades, net expectancy 0, profit factor infinite — fail.
- Fail: no top config has M5 net expectancy > 0 and profit factor > 1.

The verdict passes if any frozen top config passes M5. M6-M10 is included above for transparency.

