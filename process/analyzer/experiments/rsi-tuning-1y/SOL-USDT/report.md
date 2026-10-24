# RSI Per-Symbol Tuning — SOL-USDT (4hour, 1 year)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2025-10-23T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | da990bdadf6ffe6a73e95ace204377f81aaf97cbf2c9cf9078cd41845c996c1e |

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
| top-1 | rsi21-long65-short50 (period 21, long 65, short 50) | 2 | 65.1759 | ∞ | 0 |
| top-2 | rsi21-long65-short45 (period 21, long 65, short 45) | 2 | 51.5593 | ∞ | 0 |
| top-3 | rsi14-long70-short45 (period 14, long 70, short 45) | 4 | 64.0439 | 6.9814 | 4.4225 |

## 3. Validation — M1-M4 development

All 24 candidates, base costs. Frozen configs are marked **top-1**, **top-2**, and **top-3**.

| Label | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Top |
|---|---|---:|---:|---:|---:|---:|---|
| rsi21-long65-short50 | period 21 / long 65 / short 50 | 2 | 65.1759 | ∞ | 0 | 32.1612 | **top-1** |
| rsi21-long65-short45 | period 21 / long 65 / short 45 | 2 | 51.5593 | ∞ | 0 | 29.4299 | **top-2** |
| rsi14-long70-short45 | period 14 / long 70 / short 45 | 4 | 64.0439 | 6.9814 | 4.4225 | 46.2505 | **top-3** |
| rsi14-long70-short40 | period 14 / long 70 / short 40 | 4 | 52.3381 | 4.7805 | 6.6791 | 47.1847 |  |
| rsi21-long65-short40 | period 21 / long 65 / short 40 | 2 | 41.8447 | ∞ | 0 | 34.5833 |  |
| rsi14-long70-short50 | period 14 / long 70 / short 50 | 6 | 37.6368 | 4.5403 | 2.7874 | 34.0240 |  |
| rsi21-long70-short50 | period 21 / long 70 / short 50 | 2 | 31.2294 | 14.8880 | 0.4215 | 35.7268 |  |
| rsi14-long65-short45 | period 14 / long 65 / short 45 | 8 | 21.9104 | 2.3550 | 9.4775 | 26.7080 |  |
| rsi21-long70-short45 | period 21 / long 70 / short 45 | 2 | 17.3774 | 3.3776 | 1.3930 | 31.9952 |  |
| baseline | period 21 / long 60 / short 50 | 10 | 15.3579 | 1.9858 | 9.0056 | 20.8778 |  |
| rsi14-long60-short45 | period 14 / long 60 / short 45 | 11 | 13.6457 | 1.9738 | 10.1619 | 20.1538 |  |
| rsi14-long65-short40 | period 14 / long 65 / short 40 | 8 | 11.4701 | 1.5469 | 12.1164 | 26.8777 |  |
| rsi21-long60-short45 | period 21 / long 60 / short 45 | 8 | 11.4135 | 1.5436 | 12.0132 | 26.0330 |  |
| rsi14-long65-short50 | period 14 / long 65 / short 50 | 12 | 10.4638 | 1.7170 | 9.6740 | 18.0445 |  |
| rsi21-long55-short45 | period 21 / long 55 / short 45 | 11 | 8.9874 | 1.5530 | 11.3631 | 19.5193 |  |
| rsi21-long70-short40 | period 21 / long 70 / short 40 | 2 | 8.0129 | 1.5635 | 2.7230 | 36.4537 |  |
| rsi14-long60-short50 | period 14 / long 60 / short 50 | 19 | 7.0202 | 1.6776 | 10.5406 | 11.6181 |  |
| rsi21-long55-short50 | period 21 / long 55 / short 50 | 19 | 6.0198 | 1.5490 | 13.9623 | 11.9125 |  |
| rsi14-long60-short40 | period 14 / long 60 / short 40 | 11 | 4.2953 | 1.2130 | 14.0935 | 20.1404 |  |
| rsi14-long55-short50 | period 14 / long 55 / short 50 | 34 | 2.7900 | 1.3753 | 15.8596 | 5.7677 |  |
| rsi21-long60-short40 | period 21 / long 60 / short 40 | 7 | 1.9045 | 1.0657 | 19.5630 | 30.8959 |  |
| rsi21-long55-short40 | period 21 / long 55 / short 40 | 9 | 1.4611 | 1.0572 | 17.9031 | 24.3624 |  |
| rsi14-long55-short45 | period 14 / long 55 / short 45 | 22 | 0.9156 | 1.0665 | 18.5736 | 9.2561 |  |
| rsi14-long55-short40 | period 14 / long 55 / short 40 | 20 | -6.0192 | 0.7065 | 29.7296 | 10.5005 |  |

## 4. Holdout

M5 is the decision partition; M6-M10 is logged for every frozen config and never changes the decision.

### Top-1: rsi21-long65-short50 (period 21, long 65, short 50) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -38.0511 | 0 | 16.5472 | 7.7081 | -152.2045 | -13.2678 |
| Stress | 4 | -39.5660 | 0 | 17.3050 | 7.7345 | -158.2641 | -14.7855 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 11 | -8.5331 | 0.5701 | 14.5349 | 13.2286 | -93.8644 | 71.5818 |
| Stress | 11 | -10.0306 | 0.5213 | 15.5012 | 13.2454 | -110.3366 | 69.9720 |

### Top-2: rsi21-long65-short45 (period 21, long 65, short 45) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -47.7326 | 0 | 21.4257 | 8.8587 | -190.9304 | -23.3264 |
| Stress | 4 | -49.2467 | 0 | 22.1846 | 8.8887 | -196.9867 | -24.8591 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 11 | -12.1586 | 0.4767 | 15.9020 | 13.5111 | -133.7446 | 71.5818 |
| Stress | 11 | -13.6570 | 0.4388 | 16.9068 | 13.5275 | -150.2269 | 69.9720 |

### Top-3: rsi14-long70-short45 (period 14, long 70, short 45) — M5 **FAIL**

**M5 — first out-of-sample (decision partition)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -40.7951 | 0 | 17.6448 | 8.9288 | -163.1805 | -13.2678 |
| Stress | 4 | -42.3104 | 0 | 18.4027 | 8.9547 | -169.2416 | -14.7855 |

M5 result: **FAIL** (net expectancy > 0 and PF > 1; no trade-count gate).

**M6-M10 — extended OOS log (context only)**

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 13 | -13.4238 | 0.3937 | 22.0800 | 12.2616 | -174.5095 | 71.5818 |
| Stress | 13 | -14.9218 | 0.3627 | 23.3855 | 12.2773 | -193.9839 | 69.9720 |

M6-M10 is logged for context and does not change the decision.

## 5. Robustness

Using rsi21-long65-short50 (period 21, long 65, short 50) (the first top config that passes M5, else top-1).

**Long vs short (M5, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 2 | -56.1261 | 0 | -28.0630 | 0 | 0 |
| Short | 2 | -96.0785 | -13.2678 | -48.0392 | 0 | 0 |

**BTC regime (M5, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 1 | -21.9158 | -13.2678 | -21.9158 | 0 | 0 |
| Bear | 3 | -130.2887 | 0 | -43.4296 | 0 | 0 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs (M1-M4, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the chosen config.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| 11.4135 | 24.3034 | 65.1759 | 6 |

## 6. Decision

**Verdict: Fail**

- Top-1 rsi21-long65-short50: M5 4 trades, net expectancy -38.051133423210315288880380215, profit factor 0 — fail.
- Top-2 rsi21-long65-short45: M5 4 trades, net expectancy -47.732598282564949926643666335, profit factor 0 — fail.
- Top-3 rsi14-long70-short45: M5 4 trades, net expectancy -40.795124420391804921308271312, profit factor 0 — fail.
- Fail: no top config has M5 net expectancy > 0 and profit factor > 1.

The verdict passes if any frozen top config passes M5. M6-M10 is included above for transparency.

