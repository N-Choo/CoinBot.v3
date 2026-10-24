# BTC 1-day time-series momentum walk-forward improvement experiment

> Generated at 2026-10-23T00:00:00Z. This is a cost-aware historical simulation, not evidence of future profitability. The OOS windows were not used to tune the frozen configs.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| Requested range | 2021-10-24 .. 2026-10-23 |
| Effective range | 2021-10-24 .. 2026-10-21 |
| Candles | 1824 |
| Missing candles | 0 |
| Duplicates dropped | 0 |
| Pages | 2 |
| Fetched at | 2026-10-24T01:11:40Z |
| SHA-256 | 652cffe669ec32a0a4ab541f49d10eded5678070d6e5d6243338ad31230bd0fa |

| Train window | Train candles | OOS window | OOS candles | Train range | OOS range |
|---|---:|---|---:|---|---|
| M1-M12 | 364 | M13-M18 | 183 | 2021-10-24T00:00:00Z .. 2022-10-22T00:00:00Z | 2022-10-23T00:00:00Z .. 2023-04-23T00:00:00Z |
| M1-M18 | 547 | M19-M24 | 182 | 2021-10-24T00:00:00Z .. 2023-04-23T00:00:00Z | 2023-04-24T00:00:00Z .. 2023-10-22T00:00:00Z |
| M1-M24 | 729 | M25-M30 | 183 | 2021-10-24T00:00:00Z .. 2023-10-22T00:00:00Z | 2023-10-23T00:00:00Z .. 2024-04-22T00:00:00Z |
| M1-M30 | 912 | M31-M36 | 182 | 2021-10-24T00:00:00Z .. 2024-04-22T00:00:00Z | 2024-04-23T00:00:00Z .. 2024-10-21T00:00:00Z |
| M1-M36 | 1094 | M37-M42 | 182 | 2021-10-24T00:00:00Z .. 2024-10-21T00:00:00Z | 2024-10-22T00:00:00Z .. 2025-04-21T00:00:00Z |
| M1-M42 | 1276 | M43-M48 | 183 | 2021-10-24T00:00:00Z .. 2025-04-21T00:00:00Z | 2025-04-22T00:00:00Z .. 2025-10-21T00:00:00Z |
| M1-M48 | 1459 | M49-M54 | 182 | 2021-10-24T00:00:00Z .. 2025-10-21T00:00:00Z | 2025-10-22T00:00:00Z .. 2026-04-21T00:00:00Z |
| M1-M54 | 1641 | M55-M60 | 183 | 2021-10-24T00:00:00Z .. 2026-04-21T00:00:00Z | 2026-04-22T00:00:00Z .. 2026-10-21T00:00:00Z |

## 2. Experiment and grid

| Field | Value |
|---|---|
| Strategy | BTC time-series momentum |
| Variant | VolTarget |
| Mode | Flat (long/flat only) |
| Volatility lookback | 30 days (fixed) |
| Exposure clamp | 0..1 |
| Grid size | 18 configs |
| Walk-forward windows | 8 |
| Train / OOS | expanding train ending at months 12,18,...,54; next 6 months OOS |
| Ranking metric | train Sharpe |
| Frozen per window | top 3 |
| Base costs | fee 10 bps, slip 5 bps |
| Stress costs | fee 20 bps, slip 10 bps |
| OOS success gates | >= 20 trades, stress net return > BTC buy-and-hold stress return, stress Sharpe > 0 |
| Overall decision rule | at least half of the OOS windows contain a successful frozen top-3 config |

| # | Lookback | Regime filter | Target vol | Label | Role |
|---:|---:|---|---:|---|---|
| 1 | 30 | off | 15% | 30/off/15 | Candidate |
| 2 | 30 | off | 20% | 30/off/20 | Baseline |
| 3 | 30 | off | 25% | 30/off/25 | Candidate |
| 4 | 30 | on | 15% | 30/on/15 | Candidate |
| 5 | 30 | on | 20% | 30/on/20 | Candidate |
| 6 | 30 | on | 25% | 30/on/25 | Candidate |
| 7 | 60 | off | 15% | 60/off/15 | Candidate |
| 8 | 60 | off | 20% | 60/off/20 | Candidate |
| 9 | 60 | off | 25% | 60/off/25 | Candidate |
| 10 | 60 | on | 15% | 60/on/15 | Candidate |
| 11 | 60 | on | 20% | 60/on/20 | Candidate |
| 12 | 60 | on | 25% | 60/on/25 | Candidate |
| 13 | 90 | off | 15% | 90/off/15 | Candidate |
| 14 | 90 | off | 20% | 90/off/20 | Candidate |
| 15 | 90 | off | 25% | 90/off/25 | Candidate |
| 16 | 90 | on | 15% | 90/on/15 | Candidate |
| 17 | 90 | on | 20% | 90/on/20 | Candidate |
| 18 | 90 | on | 25% | 90/on/25 | Candidate |

## 3. Walk-forward results

### Window M1-M12 / OOS M13-M18

Train indices 0..364 (364 candles), OOS indices 364..547 (183 candles).
BTC buy-and-hold stress benchmark: net 42.80%, Sharpe 1.56, max drawdown 25.91%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/off/15 | -1.23 | -7.99% | 12 | 10.51% | 1.28 | 8.92% | 1.11 | 10 | FAIL | OOS trades 10 < 20; stress net return 8.92% <= benchmark 42.80% |
| Top 2 | 30/off/20 | -1.23 | -10.60% | 12 | 13.91% | 1.28 | 11.72% | 1.11 | 10 | FAIL | OOS trades 10 < 20; stress net return 11.72% <= benchmark 42.80% |
| Top 3 | 30/off/25 | -1.23 | -13.18% | 12 | 17.23% | 1.28 | 14.42% | 1.11 | 10 | FAIL | OOS trades 10 < 20; stress net return 14.42% <= benchmark 42.80% |
| Baseline | 30/off/20 | -1.23 | -10.60% | 12 | 13.91% | 1.28 | 11.72% | 1.11 | 10 | FAIL | OOS trades 10 < 20; stress net return 11.72% <= benchmark 42.80% |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 42.80% | 1.56 | 1 | — | — |

### Window M1-M18 / OOS M19-M24

Train indices 0..547 (547 candles), OOS indices 547..729 (182 candles).
BTC buy-and-hold stress benchmark: net 8.06%, Sharpe 0.62, max drawdown 20.00%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.19 | 13.59% | 3 | 6.68% | 1.66 | 6.14% | 1.55 | 4 | FAIL | OOS trades 4 < 20; stress net return 6.14% <= benchmark 8.06% |
| Top 2 | 30/on/20 | 1.19 | 18.31% | 3 | 8.93% | 1.66 | 8.20% | 1.55 | 4 | FAIL | OOS trades 4 < 20 |
| Top 3 | 30/on/25 | 1.19 | 23.11% | 3 | 10.97% | 1.64 | 10.05% | 1.53 | 4 | FAIL | OOS trades 4 < 20 |
| Baseline | 30/off/20 | 0.15 | 1.74% | 22 | 11.17% | 1.69 | 10.15% | 1.56 | 5 | FAIL | OOS trades 5 < 20 |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 8.06% | 0.62 | 1 | — | — |

### Window M1-M24 / OOS M25-M30

Train indices 0..729 (729 candles), OOS indices 729..912 (183 candles).
BTC buy-and-hold stress benchmark: net 121.44%, Sharpe 3.16, max drawdown 16.15%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.31 | 21.18% | 7 | 29.72% | 3.14 | 28.50% | 3.04 | 9 | FAIL | OOS trades 9 < 20; stress net return 28.50% <= benchmark 121.44% |
| Top 2 | 30/on/20 | 1.31 | 28.87% | 7 | 41.02% | 3.14 | 39.25% | 3.04 | 9 | FAIL | OOS trades 9 < 20; stress net return 39.25% <= benchmark 121.44% |
| Top 3 | 30/on/25 | 1.31 | 36.61% | 7 | 53.06% | 3.14 | 50.66% | 3.04 | 9 | FAIL | OOS trades 9 < 20; stress net return 50.66% <= benchmark 121.44% |
| Baseline | 30/off/20 | 0.50 | 13.10% | 27 | 41.02% | 3.14 | 39.25% | 3.04 | 9 | FAIL | OOS trades 9 < 20; stress net return 39.25% <= benchmark 121.44% |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 121.44% | 3.16 | 1 | — | — |

### Window M1-M30 / OOS M31-M36

Train indices 0..912 (912 candles), OOS indices 912..1094 (182 candles).
BTC buy-and-hold stress benchmark: net 0.24%, Sharpe 0.25, max drawdown 24.43%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.83 | 57.31% | 15 | 1.28% | 0.39 | 0.66% | 0.22 | 6 | FAIL | OOS trades 6 < 20 |
| Top 2 | 30/on/20 | 1.83 | 81.91% | 15 | 1.65% | 0.39 | 0.82% | 0.22 | 6 | FAIL | OOS trades 6 < 20 |
| Top 3 | 30/on/25 | 1.82 | 109.36% | 15 | 1.99% | 0.39 | 0.96% | 0.22 | 6 | FAIL | OOS trades 6 < 20 |
| Baseline | 30/off/20 | 1.22 | 59.65% | 35 | -9.28% | -1.45 | -10.41% | -1.65 | 10 | FAIL | OOS trades 10 < 20; stress net return -10.41% <= benchmark 0.24%; stress Sharpe -1.646 <= 0 |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 0.24% | 0.25 | 1 | — | — |

### Window M1-M36 / OOS M37-M42

Train indices 0..1094 (1094 candles), OOS indices 1094..1276 (182 candles).
BTC buy-and-hold stress benchmark: net 29.12%, Sharpe 1.24, max drawdown 28.11%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.64 | 59.39% | 20 | 6.04% | 1.03 | 5.05% | 0.88 | 8 | FAIL | OOS trades 8 < 20; stress net return 5.05% <= benchmark 29.12% |
| Top 2 | 30/on/20 | 1.64 | 85.02% | 20 | 7.96% | 1.03 | 6.62% | 0.88 | 8 | FAIL | OOS trades 8 < 20; stress net return 6.62% <= benchmark 29.12% |
| Top 3 | 30/on/25 | 1.64 | 113.68% | 20 | 9.83% | 1.04 | 8.13% | 0.88 | 8 | FAIL | OOS trades 8 < 20; stress net return 8.13% <= benchmark 29.12% |
| Baseline | 30/off/20 | 0.86 | 44.92% | 44 | 5.85% | 0.78 | 4.04% | 0.56 | 13 | FAIL | OOS trades 13 < 20; stress net return 4.04% <= benchmark 29.12% |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 29.12% | 1.24 | 1 | — | — |

### Window M1-M42 / OOS M43-M48

Train indices 0..1276 (1276 candles), OOS indices 1276..1459 (183 candles).
BTC buy-and-hold stress benchmark: net 23.03%, Sharpe 1.38, max drawdown 14.60%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.53 | 69.11% | 27 | 7.86% | 1.13 | 6.76% | 0.99 | 5 | FAIL | OOS trades 5 < 20; stress net return 6.76% <= benchmark 23.03% |
| Top 2 | 30/on/20 | 1.53 | 99.90% | 27 | 10.37% | 1.13 | 8.87% | 0.99 | 5 | FAIL | OOS trades 5 < 20; stress net return 8.87% <= benchmark 23.03% |
| Top 3 | 30/on/25 | 1.53 | 134.92% | 27 | 12.30% | 1.11 | 10.52% | 0.98 | 5 | FAIL | OOS trades 5 < 20; stress net return 10.52% <= benchmark 23.03% |
| Baseline | 30/off/20 | 0.85 | 53.52% | 56 | 10.37% | 1.13 | 8.87% | 0.99 | 5 | FAIL | OOS trades 5 < 20; stress net return 8.87% <= benchmark 23.03% |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 23.03% | 1.38 | 1 | — | — |

### Window M1-M48 / OOS M49-M54

Train indices 0..1459 (1459 candles), OOS indices 1459..1641 (182 candles).
BTC buy-and-hold stress benchmark: net -29.95%, Sharpe -1.17, max drawdown 45.08%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.46 | 82.41% | 32 | -0.59% | -0.63 | -0.69% | -0.77 | 1 | FAIL | OOS trades 1 < 20; stress Sharpe -0.767 <= 0 |
| Top 2 | 30/on/20 | 1.46 | 120.62% | 32 | -0.78% | -0.63 | -0.93% | -0.77 | 1 | FAIL | OOS trades 1 < 20; stress Sharpe -0.767 <= 0 |
| Top 3 | 30/on/25 | 1.45 | 163.81% | 32 | -0.98% | -0.63 | -1.16% | -0.77 | 1 | FAIL | OOS trades 1 < 20; stress Sharpe -0.768 <= 0 |
| Baseline | 30/off/20 | 0.89 | 69.53% | 60 | -10.83% | -1.72 | -12.89% | -2.09 | 15 | FAIL | OOS trades 15 < 20; stress Sharpe -2.093 <= 0 |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | -29.95% | -1.17 | 1 | — | — |

### Window M1-M54 / OOS M55-M60

Train indices 0..1641 (1641 candles), OOS indices 1641..1824 (183 candles).
BTC buy-and-hold stress benchmark: net 9.51%, Sharpe 0.68, max drawdown 28.69%.

| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| Top 1 | 30/on/15 | 1.36 | 81.34% | 33 | 13.67% | 2.35 | 13.42% | 2.33 | 1 | FAIL | OOS trades 1 < 20 |
| Top 2 | 30/on/20 | 1.36 | 118.89% | 33 | 18.47% | 2.35 | 18.13% | 2.33 | 1 | FAIL | OOS trades 1 < 20 |
| Top 3 | 30/on/25 | 1.35 | 161.21% | 33 | 22.64% | 2.35 | 22.23% | 2.33 | 1 | FAIL | OOS trades 1 < 20 |
| Baseline | 30/off/20 | 0.65 | 51.17% | 75 | 11.12% | 1.28 | 9.54% | 1.12 | 6 | FAIL | OOS trades 6 < 20 |
| Benchmark | BTC buy-and-hold | — | — | — | — | — | 9.51% | 0.68 | 1 | — | — |

## 4. Overall results

| Metric | Value |
|---|---:|
| Windows with at least one successful top-3 run | 0 / 8 |
| Successful frozen top-3 runs | 0 / 24 |
| Baseline 30/off/20 windows passed | 0 / 8 |
| Mean selected-run OOS stress net return | 11.55% |
| Mean baseline OOS stress net return | 7.53% |

| Window | Successful top-3 configs | Baseline 30/off/20 | Baseline OOS stress net | Baseline OOS stress Sharpe | Baseline trades | Benchmark stress net |
|---|---|---|---:|---:|---:|---:|
| M1-M12 | none | FAIL | 11.72% | 1.11 | 10 | 42.80% |
| M1-M18 | none | FAIL | 10.15% | 1.56 | 5 | 8.06% |
| M1-M24 | none | FAIL | 39.25% | 3.04 | 9 | 121.44% |
| M1-M30 | none | FAIL | -10.41% | -1.65 | 10 | 0.24% |
| M1-M36 | none | FAIL | 4.04% | 0.56 | 13 | 29.12% |
| M1-M42 | none | FAIL | 8.87% | 0.99 | 5 | 23.03% |
| M1-M48 | none | FAIL | -12.89% | -2.09 | 15 | -29.95% |
| M1-M54 | none | FAIL | 9.54% | 1.12 | 6 | 9.51% |

| Window | Frozen config | OOS stress net | OOS stress Sharpe | OOS trades | Benchmark stress net |
|---|---|---:|---:|---:|---:|

## 5. Stability and consistency

All statistics below cover the frozen top-3 runs only; the baseline is reported separately.

| Axis | Selections | Successful | Mean train Sharpe | Mean OOS stress net | Mean OOS stress Sharpe |
|---|---:|---:|---:|---:|---:|
| lookback 30 | 24 | 0 | 1.13 | 11.55% | 1.17 |
| lookback 60 | 0 | 0 | — | — | — |
| lookback 90 | 0 | 0 | — | — | — |
| regime off | 3 | 0 | -1.23 | 11.69% | 1.11 |
| regime on | 21 | 0 | 1.47 | 11.53% | 1.17 |
| target vol 15% | 8 | 0 | 1.14 | 8.59% | 1.17 |
| target vol 20% | 8 | 0 | 1.14 | 11.58% | 1.17 |
| target vol 25% | 8 | 0 | 1.13 | 14.48% | 1.16 |

| Config | Windows selected | Successful runs | Mean OOS stress net | Mean OOS stress Sharpe |
|---|---:|---:|---:|---:|
| 30/off/15 | 1 | 0 | 8.92% | 1.11 |
| 30/off/20 | 1 | 0 | 11.72% | 1.11 |
| 30/off/25 | 1 | 0 | 14.42% | 1.11 |
| 30/on/15 | 7 | 0 | 8.55% | 1.18 |
| 30/on/20 | 7 | 0 | 11.56% | 1.18 |
| 30/on/25 | 7 | 0 | 14.48% | 1.17 |

| Window | Top-3 frozen configs |
|---|---|
| M1-M12 | 30/off/15 (FAIL), 30/off/20 (FAIL), 30/off/25 (FAIL) |
| M1-M18 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M24 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M30 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M36 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M42 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M48 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |
| M1-M54 | 30/on/15 (FAIL), 30/on/20 (FAIL), 30/on/25 (FAIL) |

## 6. Decision

**Overall verdict: Fail**

Reasons:

- 0 of 8 OOS windows contained at least one successful frozen top-3 config; the majority rule requires 4.
- 0 of 24 frozen top-3 OOS runs passed all gates (>= 20 trades, stress net return > BTC buy-and-hold, stress Sharpe > 0).
- Baseline 30/off/20 passed in 0 of 8 OOS windows.

**Improvement checks against the 30/off/20 baseline**

- Regime filter: on selected 21 runs (0 passed, mean stress net 11.53%); off selected 3 runs (0 passed, mean stress net 11.69%).
- Longer lookback: 30 selected 24 runs (0 passed, mean stress net 11.55%); 60 selected 0 runs (0 passed, mean stress net —); 90 selected 0 runs (0 passed, mean stress net —).
- Volatility target: 15% selected 8 runs (0 passed, mean stress net 8.59%); 20% selected 8 runs (0 passed, mean stress net 11.58%); 25% selected 8 runs (0 passed, mean stress net 14.48%).
- Baseline 30/off/20 passed 0 of 8 windows (mean stress net 7.53%, mean stress Sharpe 0.58).

