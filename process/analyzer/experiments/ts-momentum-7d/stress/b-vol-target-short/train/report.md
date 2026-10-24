# BTC time-series momentum — Volatility targeting / Short / train / stress

> BTC time-series momentum. Signals use only closed daily candles and execute at the next open.

## Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| As of | 2026-10-23T00:00:00Z |
| Requested range | 2021-10-24 .. 2026-10-23 |
| Effective range | 2021-10-24 .. 2026-10-21 |
| Closed candles | 1824 |
| Requested / expected candles | 1826 / 1824 |
| Missing candles | 0 |
| Duplicates dropped | 0 |
| Pages fetched | 2 |
| Fetched at | 2026-10-24T01:11:40Z |
| Settle | 5000 ms |
| SHA-256 | `652cffe669ec32a0a4ab541f49d10eded5678070d6e5d6243338ad31230bd0fa` |

## Strategy and configuration

| Field | Value |
|---|---|
| Strategy | btc-ts-momentum-v1 |
| Momentum lookback | 7 days |
| Volatility lookback | 7 days |
| Target annualized volatility | 0.2 |
| Annualization days | 365 |
| Exposure bounds | [0, 1] |
| Negative momentum | Flat and Short run separately |
| Initial fund | 1000 |
| Borrow cost | 0 bps/day on short exposure |
| Drawdown control | halt at 20%, resume below 10% |
| Costs (stress) | 20 bps fee + 10 bps slippage per side |

Variants:

- **A. Fixed exposure** — long 1.0 when momentum is positive, flat otherwise; short -1.0 in Short mode.
- **B. Volatility targeting** — exposure is `clamp(target_vol / realized_vol, min, max)` with the momentum direction.
- **C. Volatility targeting + drawdown control** — B plus a configurable 20%/10% drawdown hysteresis overlay. The hysteresis is evaluated on the uncapped variant-B reference equity path so it can recover while the live portfolio is flat.

## Split

| Field | Value |
|---|---|
| Split | train |
| Candles | 1094 |
| Range | 2021-10-24 .. 2024-10-21 |
| Variant | Volatility targeting |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -56.52% | 9.26% |
| CAGR | -24.26% | 3.00% |
| Max drawdown | 61.37% | 76.63% |
| Sharpe | -0.85 | 0.33 |
| Sortino | -1.25 | 0.47 |
| Win rate | 26.37% | 100.00% |
| Trades | 201 | 1 |
| Average holding days | 5.40 | 1094.00 |
| Turnover (initial equity) | 168.63 | 2.10 |
| Trading costs | 505.9 | 6.29 |
| Borrow costs | 0 | — |
| Average exposure | 1.28% | 100.00% |
| Average absolute exposure | 53.41% | 100.00% |
| Days with exposure | 1086 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | -19.06% | -0.40 | -0.59 | 44.23% | 7.22% | 57.96% |
| Bear | 379 | -44.22% | -1.89 | -2.66 | 46.69% | -3.78% | 54.62% |
| Unknown | 199 | -3.70% | -0.10 | -0.15 | 15.70% | -4.47% | 39.33% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2021-11-01 | 2021-11-02 | 1 | -4.27 | no |
| Short | 2021-11-02 | 2021-11-03 | 1 | -17.01 | no |
| Long | 2021-11-03 | 2021-11-06 | 3 | -20.71 | no |
| Short | 2021-11-06 | 2021-11-08 | 2 | -24.5 | no |
| Long | 2021-11-08 | 2021-11-16 | 8 | 8.06 | no |
| Short | 2021-11-16 | 2021-11-26 | 10 | 38.86 | no |
| Long | 2021-11-26 | 2021-11-27 | 1 | -37.87 | no |
| Short | 2021-11-27 | 2021-11-30 | 3 | -19.08 | no |
| Long | 2021-11-30 | 2021-12-01 | 1 | -4.68 | no |
| Short | 2021-12-01 | 2021-12-02 | 1 | -2.38 | no |
| Long | 2021-12-02 | 2021-12-03 | 1 | -4.17 | no |
| Short | 2021-12-03 | 2021-12-12 | 9 | 35.61 | no |
| Long | 2021-12-12 | 2021-12-14 | 2 | -19.61 | no |
| Short | 2021-12-14 | 2021-12-17 | 3 | -7.28 | no |
| Long | 2021-12-17 | 2021-12-18 | 1 | -9.8 | no |
| Short | 2021-12-18 | 2021-12-21 | 3 | -5.94 | no |
| Long | 2021-12-21 | 2021-12-23 | 2 | 13.39 | no |
| Short | 2021-12-23 | 2021-12-24 | 1 | -21.93 | no |
| Long | 2021-12-24 | 2021-12-29 | 5 | -32.08 | no |
| Short | 2021-12-29 | 2022-01-13 | 15 | 24.01 | no |
| Long | 2022-01-13 | 2022-01-14 | 1 | -17.64 | no |
| Short | 2022-01-14 | 2022-01-15 | 1 | -7.2 | no |
| Long | 2022-01-15 | 2022-01-19 | 4 | -11.69 | no |
| Short | 2022-01-19 | 2022-01-29 | 10 | 85.66 | no |
| Long | 2022-01-29 | 2022-02-14 | 16 | 22.62 | no |
| Short | 2022-02-14 | 2022-02-16 | 2 | -47.28 | no |
| Long | 2022-02-16 | 2022-02-17 | 1 | -9.3 | no |
| Short | 2022-02-17 | 2022-03-01 | 12 | -3.74 | no |
| Long | 2022-03-01 | 2022-03-05 | 4 | -17.57 | no |
| Short | 2022-03-05 | 2022-03-06 | 1 | -1.81 | no |
| Long | 2022-03-06 | 2022-03-08 | 2 | -6.1 | no |
| Short | 2022-03-08 | 2022-03-15 | 7 | -23.7 | no |
| Long | 2022-03-15 | 2022-03-17 | 2 | 6.2 | no |
| Short | 2022-03-17 | 2022-03-18 | 1 | -0.28 | no |
| Long | 2022-03-18 | 2022-04-04 | 17 | 56.29 | no |
| Short | 2022-04-04 | 2022-04-19 | 15 | 63.23 | no |
| Long | 2022-04-19 | 2022-04-23 | 4 | -19.54 | no |
| Short | 2022-04-23 | 2022-05-05 | 12 | -3.58 | no |
| Long | 2022-05-05 | 2022-05-06 | 1 | -31.21 | no |
| Short | 2022-05-06 | 2022-05-20 | 14 | 38.05 | no |
| Long | 2022-05-20 | 2022-05-21 | 1 | -10.67 | no |
| Short | 2022-05-21 | 2022-05-26 | 5 | -4.57 | no |
| Long | 2022-05-26 | 2022-05-27 | 1 | -5.52 | no |
| Short | 2022-05-27 | 2022-05-31 | 4 | -46.02 | no |
| Long | 2022-05-31 | 2022-06-07 | 7 | -9.44 | no |
| Short | 2022-06-07 | 2022-06-09 | 2 | 8.86 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -3.48 | no |
| Short | 2022-06-10 | 2022-06-24 | 14 | 97.82 | no |
| Long | 2022-06-24 | 2022-06-29 | 5 | -18.53 | no |
| Short | 2022-06-29 | 2022-06-30 | 1 | 0.56 | no |
| Long | 2022-06-30 | 2022-07-01 | 1 | -6.23 | no |
| Short | 2022-07-01 | 2022-07-07 | 6 | -30.55 | no |
| Long | 2022-07-07 | 2022-07-12 | 5 | -12.81 | no |
| Short | 2022-07-12 | 2022-07-19 | 7 | -39.98 | no |
| Long | 2022-07-19 | 2022-07-26 | 7 | -17.59 | no |
| Short | 2022-07-26 | 2022-07-29 | 3 | -47.85 | no |
| Long | 2022-07-29 | 2022-08-04 | 6 | -12.28 | no |
| Short | 2022-08-04 | 2022-08-09 | 5 | -36.6 | no |
| Long | 2022-08-09 | 2022-08-18 | 9 | -17.17 | no |
| Short | 2022-08-18 | 2022-09-05 | 18 | 71.87 | no |
| Long | 2022-09-05 | 2022-09-06 | 1 | -7.91 | no |
| Short | 2022-09-06 | 2022-09-10 | 4 | -21.54 | no |
| Long | 2022-09-10 | 2022-09-17 | 7 | -14.48 | no |
| Short | 2022-09-17 | 2022-09-28 | 11 | -1.56 | no |
| Long | 2022-09-28 | 2022-10-11 | 13 | -0.02 | no |
| Short | 2022-10-11 | 2022-10-18 | 7 | -22.85 | no |
| Long | 2022-10-18 | 2022-10-20 | 2 | -20.64 | no |
| Short | 2022-10-20 | 2022-10-23 | 3 | -7.52 | no |
| Long | 2022-10-23 | 2022-10-25 | 2 | 0.73 | no |
| Short | 2022-10-25 | 2022-10-26 | 1 | -31.78 | no |
| Long | 2022-10-26 | 2022-11-03 | 8 | -1.34 | no |
| Short | 2022-11-03 | 2022-11-05 | 2 | -37.5 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -44 | no |
| Short | 2022-11-09 | 2022-11-17 | 8 | 14.19 | no |
| Long | 2022-11-17 | 2022-11-18 | 1 | -0.68 | no |
| Short | 2022-11-18 | 2022-11-28 | 10 | 4.8 | no |
| Long | 2022-11-28 | 2022-12-08 | 10 | 15.52 | no |
| Short | 2022-12-08 | 2022-12-09 | 1 | -18.59 | no |
| Long | 2022-12-09 | 2022-12-12 | 3 | -7.35 | no |
| Short | 2022-12-12 | 2022-12-13 | 1 | -7.31 | no |
| Long | 2022-12-13 | 2022-12-17 | 4 | -13.08 | no |
| Short | 2022-12-17 | 2022-12-24 | 7 | -4.42 | no |
| Long | 2022-12-24 | 2022-12-28 | 4 | -6.9 | no |
| Short | 2022-12-28 | 2023-01-05 | 8 | -8.71 | no |
| Long | 2023-01-05 | 2023-01-31 | 26 | 126.79 | no |
| Short | 2023-01-31 | 2023-02-01 | 1 | -6.84 | no |
| Long | 2023-02-01 | 2023-02-06 | 5 | -4.43 | no |
| Short | 2023-02-06 | 2023-02-08 | 2 | -11.81 | no |
| Long | 2023-02-08 | 2023-02-09 | 1 | -8.69 | no |
| Short | 2023-02-09 | 2023-02-16 | 7 | -12.2 | no |
| Long | 2023-02-16 | 2023-02-23 | 7 | -1.3 | no |
| Short | 2023-02-23 | 2023-02-24 | 1 | 1.17 | no |
| Long | 2023-02-24 | 2023-02-25 | 1 | -12.15 | no |
| Short | 2023-02-25 | 2023-03-14 | 17 | 1.99 | no |
| Long | 2023-03-14 | 2023-03-28 | 14 | 16.65 | no |
| Short | 2023-03-28 | 2023-03-30 | 2 | -14.01 | no |
| Long | 2023-03-30 | 2023-03-31 | 1 | -4.45 | no |
| Short | 2023-03-31 | 2023-04-01 | 1 | -6.25 | no |
| Long | 2023-04-01 | 2023-04-06 | 5 | -4.86 | no |
| Short | 2023-04-06 | 2023-04-07 | 1 | -0.63 | no |
| Long | 2023-04-07 | 2023-04-08 | 1 | -6.62 | no |
| Short | 2023-04-08 | 2023-04-10 | 2 | -13.05 | no |
| Long | 2023-04-10 | 2023-04-18 | 8 | 22.04 | no |
| Short | 2023-04-18 | 2023-04-19 | 1 | -16.84 | no |
| Long | 2023-04-19 | 2023-04-20 | 1 | -20.81 | no |
| Short | 2023-04-20 | 2023-04-28 | 8 | -10.36 | no |
| Long | 2023-04-28 | 2023-05-05 | 7 | -13.34 | no |
| Short | 2023-05-05 | 2023-05-06 | 1 | -9.73 | no |
| Long | 2023-05-06 | 2023-05-07 | 1 | -8.35 | no |
| Short | 2023-05-07 | 2023-05-20 | 13 | 16.43 | no |
| Long | 2023-05-20 | 2023-05-22 | 2 | -6.05 | no |
| Short | 2023-05-22 | 2023-05-24 | 2 | -12.17 | no |
| Long | 2023-05-24 | 2023-05-25 | 1 | -17.96 | no |
| Short | 2023-05-25 | 2023-05-29 | 4 | -25.98 | no |
| Long | 2023-05-29 | 2023-06-05 | 7 | -10.58 | no |
| Short | 2023-06-05 | 2023-06-13 | 8 | 14.52 | no |
| Long | 2023-06-13 | 2023-06-14 | 1 | -0.99 | no |
| Short | 2023-06-14 | 2023-06-18 | 4 | -9.69 | no |
| Long | 2023-06-18 | 2023-07-01 | 13 | 34.31 | no |
| Short | 2023-07-01 | 2023-07-02 | 1 | -5.04 | no |
| Long | 2023-07-02 | 2023-07-07 | 5 | -14.51 | no |
| Short | 2023-07-07 | 2023-07-14 | 7 | -28.3 | no |
| Long | 2023-07-14 | 2023-07-15 | 1 | -16.22 | no |
| Short | 2023-07-15 | 2023-07-17 | 2 | -0.83 | no |
| Long | 2023-07-17 | 2023-07-18 | 1 | -2.26 | no |
| Short | 2023-07-18 | 2023-08-01 | 14 | 8.83 | no |
| Long | 2023-08-01 | 2023-08-03 | 2 | -3.74 | no |
| Short | 2023-08-03 | 2023-08-09 | 6 | -12.49 | no |
| Long | 2023-08-09 | 2023-08-16 | 7 | -11.68 | no |
| Short | 2023-08-16 | 2023-08-26 | 10 | 41.66 | no |
| Long | 2023-08-26 | 2023-08-27 | 1 | -3.74 | no |
| Short | 2023-08-27 | 2023-08-30 | 3 | -34.63 | no |
| Long | 2023-08-30 | 2023-09-01 | 2 | -14.87 | no |
| Short | 2023-09-01 | 2023-09-08 | 7 | -5.37 | no |
| Long | 2023-09-08 | 2023-09-11 | 3 | -9.45 | no |
| Short | 2023-09-11 | 2023-09-13 | 2 | -0.96 | no |
| Long | 2023-09-13 | 2023-09-23 | 10 | 3.85 | no |
| Short | 2023-09-23 | 2023-09-24 | 1 | -2.52 | no |
| Long | 2023-09-24 | 2023-09-25 | 1 | -7.73 | no |
| Short | 2023-09-25 | 2023-09-29 | 4 | -14.68 | no |
| Long | 2023-09-29 | 2023-10-09 | 10 | 10.54 | no |
| Short | 2023-10-09 | 2023-10-10 | 1 | 2.05 | no |
| Long | 2023-10-10 | 2023-10-11 | 1 | -5.29 | no |
| Short | 2023-10-11 | 2023-10-17 | 6 | -20.38 | no |
| Long | 2023-10-17 | 2023-11-17 | 31 | 63.99 | no |
| Short | 2023-11-17 | 2023-11-20 | 3 | -5.8 | no |
| Long | 2023-11-20 | 2023-11-23 | 3 | -1.14 | no |
| Short | 2023-11-23 | 2023-11-24 | 1 | -0.43 | no |
| Long | 2023-11-24 | 2023-11-28 | 4 | -1.37 | no |
| Short | 2023-11-28 | 2023-11-29 | 1 | -3.98 | no |
| Long | 2023-11-29 | 2023-12-12 | 13 | 43.82 | no |
| Short | 2023-12-12 | 2023-12-19 | 7 | -8.88 | no |
| Long | 2023-12-19 | 2023-12-28 | 9 | 0.72 | no |
| Short | 2023-12-28 | 2024-01-02 | 5 | -7.73 | no |
| Long | 2024-01-02 | 2024-01-04 | 2 | -9.28 | no |
| Short | 2024-01-04 | 2024-01-05 | 1 | -6.6 | no |
| Long | 2024-01-05 | 2024-01-13 | 8 | -7.01 | no |
| Short | 2024-01-13 | 2024-01-27 | 14 | 1.6 | no |
| Long | 2024-01-27 | 2024-02-06 | 10 | -1.5 | no |
| Short | 2024-02-06 | 2024-02-07 | 1 | -6.79 | no |
| Long | 2024-02-07 | 2024-02-23 | 16 | 62.02 | no |
| Short | 2024-02-23 | 2024-02-27 | 4 | -33.84 | no |
| Long | 2024-02-27 | 2024-03-17 | 19 | 24.25 | no |
| Short | 2024-03-17 | 2024-03-26 | 9 | -9.01 | no |
| Long | 2024-03-26 | 2024-04-02 | 7 | -3.9 | no |
| Short | 2024-04-02 | 2024-04-09 | 7 | 0.6 | no |
| Long | 2024-04-09 | 2024-04-13 | 4 | -14.19 | no |
| Short | 2024-04-13 | 2024-04-21 | 8 | 3.59 | no |
| Long | 2024-04-21 | 2024-04-22 | 1 | -1.08 | no |
| Short | 2024-04-22 | 2024-04-23 | 1 | -6.81 | no |
| Long | 2024-04-23 | 2024-04-27 | 4 | -11.86 | no |
| Short | 2024-04-27 | 2024-05-05 | 8 | 5.55 | no |
| Long | 2024-05-05 | 2024-05-07 | 2 | -2.37 | no |
| Short | 2024-05-07 | 2024-05-08 | 1 | 1.01 | no |
| Long | 2024-05-08 | 2024-05-11 | 3 | -4.74 | no |
| Short | 2024-05-11 | 2024-05-16 | 5 | -20.41 | no |
| Long | 2024-05-16 | 2024-05-28 | 12 | 7.04 | no |
| Short | 2024-05-28 | 2024-05-31 | 3 | 3.03 | no |
| Long | 2024-05-31 | 2024-06-01 | 1 | -7.1 | no |
| Short | 2024-06-01 | 2024-06-05 | 4 | -20.78 | no |
| Long | 2024-06-05 | 2024-06-12 | 7 | -19.04 | no |
| Short | 2024-06-12 | 2024-07-02 | 20 | 25.75 | no |
| Long | 2024-07-02 | 2024-07-04 | 2 | -12.78 | no |
| Short | 2024-07-04 | 2024-07-12 | 8 | 8.14 | no |
| Long | 2024-07-12 | 2024-07-30 | 18 | 36.1 | no |
| Short | 2024-07-30 | 2024-07-31 | 1 | 0.77 | no |
| Long | 2024-07-31 | 2024-08-01 | 1 | -9.02 | no |
| Short | 2024-08-01 | 2024-08-11 | 10 | 18.6 | no |
| Long | 2024-08-11 | 2024-08-16 | 5 | -6.03 | no |
| Short | 2024-08-16 | 2024-08-20 | 4 | -9.67 | no |
| Long | 2024-08-20 | 2024-08-21 | 1 | -2.9 | no |
| Short | 2024-08-21 | 2024-08-22 | 1 | -10.11 | no |
| Long | 2024-08-22 | 2024-08-29 | 7 | -3.69 | no |
| Short | 2024-08-29 | 2024-09-11 | 13 | 3.75 | no |
| Long | 2024-09-11 | 2024-09-12 | 1 | -2.02 | no |
| Short | 2024-09-12 | 2024-09-13 | 1 | -3.56 | no |
| Long | 2024-09-13 | 2024-10-01 | 18 | 15.93 | no |
| Short | 2024-10-01 | 2024-10-09 | 8 | 1.17 | no |
| Long | 2024-10-09 | 2024-10-10 | 1 | -13.52 | no |
| Short | 2024-10-10 | 2024-10-12 | 2 | -12.19 | no |
| Long | 2024-10-12 | 2024-10-21 | 10 | 15.57 | yes |

## Costs and execution assumptions

- Signals are evaluated at the close of a fully closed candle and executed at the next candle open.
- Overnight gap from the previous close to the next open is applied at the old exposure.
- Costs are `(fee_bps + slip_bps) / 10_000 × |target exposure - old exposure| × equity` at execution.
- After execution the new exposure earns the intraday open-to-close return. No leverage above the configured maximum of 1.0 is used.
- Borrow cost is `borrow_bps_per_day / 10_000 × |short exposure| × equity` each day; it defaults to zero.
- Benchmark uses full equity, one entry at the split's first open, one exit at the split's last close, and the same per-side rate.
- Turnover is total traded notional divided by the split's initial fund. Results are in BTC-USDT, not percentage points of price.

## Limitations

- Spot shorting is not supported by the current trade engine. Short-mode results are a backtest-only simulation.
- Signals require 8 closed daily candles (max of momentum and volatility lookback + 1); earlier days are flat.
- The 200-day BTC regime label needs 200 closes and is reported as unknown before that.
- Splits are chronological reporting partitions. No parameter tuning or model selection is performed on them.
- Fees and slippage are modelled as a single per-side rate on traded notional; market impact is not modelled.
- Open episodes at the end of a split are marked at the final close and counted as closed for win-rate reporting without an extra liquidation cost.
- The drawdown variant evaluates the 20%/10% hysteresis on the uncapped variant-B reference equity path so the control can recover while the live portfolio is flat.

## Data issues

No data issues were recorded.

