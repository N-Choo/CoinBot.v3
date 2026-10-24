# BTC time-series momentum — Fixed exposure / Short / train / base

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
| Costs (base) | 10 bps fee + 5 bps slippage per side |

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
| Variant | Fixed exposure |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -70.50% | 9.59% |
| CAGR | -33.45% | 3.10% |
| Max drawdown | 75.56% | 76.63% |
| Sharpe | -0.47 | 0.33 |
| Sortino | -0.67 | 0.48 |
| Win rate | 27.86% | 100.00% |
| Trades | 201 | 1 |
| Average holding days | 5.40 | 1094.00 |
| Turnover (initial equity) | 193.93 | 2.10 |
| Trading costs | 290.89 | 3.15 |
| Borrow costs | 0 | — |
| Average exposure | 0.37% | 100.00% |
| Average absolute exposure | 99.27% | 100.00% |
| Days with exposure | 1086 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | -12.08% | 0.07 | 0.10 | 52.16% | 17.05% | 100.00% |
| Bear | 379 | -58.41% | -1.23 | -1.71 | 68.27% | -12.40% | 100.00% |
| Unknown | 199 | -19.31% | -0.29 | -0.42 | 42.41% | -18.59% | 95.98% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2021-11-01 | 2021-11-02 | 1 | -9.44 | no |
| Short | 2021-11-02 | 2021-11-03 | 1 | -40.66 | no |
| Long | 2021-11-03 | 2021-11-06 | 3 | -36.92 | no |
| Short | 2021-11-06 | 2021-11-08 | 2 | -36.93 | no |
| Long | 2021-11-08 | 2021-11-16 | 8 | 2.1 | no |
| Short | 2021-11-16 | 2021-11-26 | 10 | 55.93 | no |
| Long | 2021-11-26 | 2021-11-27 | 1 | -85.51 | no |
| Short | 2021-11-27 | 2021-11-30 | 3 | -64.01 | no |
| Long | 2021-11-30 | 2021-12-01 | 1 | -13.72 | no |
| Short | 2021-12-01 | 2021-12-02 | 1 | -5.68 | no |
| Long | 2021-12-02 | 2021-12-03 | 1 | -11.67 | no |
| Short | 2021-12-03 | 2021-12-12 | 9 | 92.26 | no |
| Long | 2021-12-12 | 2021-12-14 | 2 | -48.57 | no |
| Short | 2021-12-14 | 2021-12-17 | 3 | -19.48 | no |
| Long | 2021-12-17 | 2021-12-18 | 1 | -26.4 | no |
| Short | 2021-12-18 | 2021-12-21 | 3 | -14.44 | no |
| Long | 2021-12-21 | 2021-12-23 | 2 | 24.17 | no |
| Short | 2021-12-23 | 2021-12-24 | 1 | -37.49 | no |
| Long | 2021-12-24 | 2021-12-29 | 5 | -48.9 | no |
| Short | 2021-12-29 | 2022-01-13 | 15 | 47.7 | no |
| Long | 2022-01-13 | 2022-01-14 | 1 | -24.26 | no |
| Short | 2022-01-14 | 2022-01-15 | 1 | -10.23 | no |
| Long | 2022-01-15 | 2022-01-19 | 4 | -13.5 | no |
| Short | 2022-01-19 | 2022-01-29 | 10 | 69.49 | no |
| Long | 2022-01-29 | 2022-02-14 | 16 | 83.12 | no |
| Short | 2022-02-14 | 2022-02-16 | 2 | -50.74 | no |
| Long | 2022-02-16 | 2022-02-17 | 1 | -14.43 | no |
| Short | 2022-02-17 | 2022-03-01 | 12 | -16.53 | no |
| Long | 2022-03-01 | 2022-03-05 | 4 | -71.61 | no |
| Short | 2022-03-05 | 2022-03-06 | 1 | -6.37 | no |
| Long | 2022-03-06 | 2022-03-08 | 2 | -25.76 | no |
| Short | 2022-03-08 | 2022-03-15 | 7 | -37.87 | no |
| Long | 2022-03-15 | 2022-03-17 | 2 | 20.18 | no |
| Short | 2022-03-17 | 2022-03-18 | 1 | 1.15 | no |
| Long | 2022-03-18 | 2022-04-04 | 17 | 81.73 | no |
| Short | 2022-04-04 | 2022-04-19 | 15 | 85.83 | no |
| Long | 2022-04-19 | 2022-04-23 | 4 | -23.51 | no |
| Short | 2022-04-23 | 2022-05-05 | 12 | -9.74 | no |
| Long | 2022-05-05 | 2022-05-06 | 1 | -62.16 | no |
| Short | 2022-05-06 | 2022-05-20 | 14 | 114.88 | no |
| Long | 2022-05-20 | 2022-05-21 | 1 | -31.99 | no |
| Short | 2022-05-21 | 2022-05-26 | 5 | -13.47 | no |
| Long | 2022-05-26 | 2022-05-27 | 1 | -11.21 | no |
| Short | 2022-05-27 | 2022-05-31 | 4 | -67.28 | no |
| Long | 2022-05-31 | 2022-06-07 | 7 | -10.08 | no |
| Short | 2022-06-07 | 2022-06-09 | 2 | 23.4 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -4.36 | no |
| Short | 2022-06-10 | 2022-06-24 | 14 | 239.33 | no |
| Long | 2022-06-24 | 2022-06-29 | 5 | -39.55 | no |
| Short | 2022-06-29 | 2022-06-30 | 1 | 4.08 | no |
| Long | 2022-06-30 | 2022-07-01 | 1 | -11.12 | no |
| Short | 2022-07-01 | 2022-07-07 | 6 | -32.85 | no |
| Long | 2022-07-07 | 2022-07-12 | 5 | -27.54 | no |
| Short | 2022-07-12 | 2022-07-19 | 7 | -101.42 | no |
| Long | 2022-07-19 | 2022-07-26 | 7 | -38.67 | no |
| Short | 2022-07-26 | 2022-07-29 | 3 | -80.02 | no |
| Long | 2022-07-29 | 2022-08-04 | 6 | -27.9 | no |
| Short | 2022-08-04 | 2022-08-09 | 5 | -26.97 | no |
| Long | 2022-08-09 | 2022-08-18 | 9 | -12.68 | no |
| Short | 2022-08-18 | 2022-09-05 | 18 | 77.21 | no |
| Long | 2022-09-05 | 2022-09-06 | 1 | -8.3 | no |
| Short | 2022-09-06 | 2022-09-10 | 4 | -54.61 | no |
| Long | 2022-09-10 | 2022-09-17 | 7 | -42.1 | no |
| Short | 2022-09-17 | 2022-09-28 | 11 | 14.34 | no |
| Long | 2022-09-28 | 2022-10-11 | 13 | -0.11 | no |
| Short | 2022-10-11 | 2022-10-18 | 7 | -13.06 | no |
| Long | 2022-10-18 | 2022-10-20 | 2 | -12.63 | no |
| Short | 2022-10-20 | 2022-10-23 | 3 | -3.6 | no |
| Long | 2022-10-23 | 2022-10-25 | 2 | 1.75 | no |
| Short | 2022-10-25 | 2022-10-26 | 1 | -20.86 | no |
| Long | 2022-10-26 | 2022-11-03 | 8 | 0.29 | no |
| Short | 2022-11-03 | 2022-11-05 | 2 | -24.9 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -57.07 | no |
| Short | 2022-11-09 | 2022-11-17 | 8 | 28.94 | no |
| Long | 2022-11-17 | 2022-11-18 | 1 | -0.6 | no |
| Short | 2022-11-18 | 2022-11-28 | 10 | 4 | no |
| Long | 2022-11-28 | 2022-12-08 | 10 | 9.22 | no |
| Short | 2022-12-08 | 2022-12-09 | 1 | -11.5 | no |
| Long | 2022-12-09 | 2022-12-12 | 3 | -4.76 | no |
| Short | 2022-12-12 | 2022-12-13 | 1 | -4.19 | no |
| Long | 2022-12-13 | 2022-12-17 | 4 | -15.17 | no |
| Short | 2022-12-17 | 2022-12-24 | 7 | -5.27 | no |
| Long | 2022-12-24 | 2022-12-28 | 4 | -2.91 | no |
| Short | 2022-12-28 | 2023-01-05 | 8 | -4.71 | no |
| Long | 2023-01-05 | 2023-01-31 | 26 | 137.04 | no |
| Short | 2023-01-31 | 2023-02-01 | 1 | -8.56 | no |
| Long | 2023-02-01 | 2023-02-06 | 5 | -6.08 | no |
| Short | 2023-02-06 | 2023-02-08 | 2 | -8.77 | no |
| Long | 2023-02-08 | 2023-02-09 | 1 | -7.69 | no |
| Short | 2023-02-09 | 2023-02-16 | 7 | -35.11 | no |
| Long | 2023-02-16 | 2023-02-23 | 7 | -4.13 | no |
| Short | 2023-02-23 | 2023-02-24 | 1 | 3.28 | no |
| Long | 2023-02-24 | 2023-02-25 | 1 | -15.77 | no |
| Short | 2023-02-25 | 2023-03-14 | 17 | -27.79 | no |
| Long | 2023-03-14 | 2023-03-28 | 14 | 51.01 | no |
| Short | 2023-03-28 | 2023-03-30 | 2 | -22.27 | no |
| Long | 2023-03-30 | 2023-03-31 | 1 | -6.37 | no |
| Short | 2023-03-31 | 2023-04-01 | 1 | -8.2 | no |
| Long | 2023-04-01 | 2023-04-06 | 5 | -5.76 | no |
| Short | 2023-04-06 | 2023-04-07 | 1 | 0.85 | no |
| Long | 2023-04-07 | 2023-04-08 | 1 | -3.17 | no |
| Short | 2023-04-08 | 2023-04-10 | 2 | -7.53 | no |
| Long | 2023-04-10 | 2023-04-18 | 8 | 14.95 | no |
| Short | 2023-04-18 | 2023-04-19 | 1 | -15.22 | no |
| Long | 2023-04-19 | 2023-04-20 | 1 | -22.76 | no |
| Short | 2023-04-20 | 2023-04-28 | 8 | -11.74 | no |
| Long | 2023-04-28 | 2023-05-05 | 7 | -9.35 | no |
| Short | 2023-05-05 | 2023-05-06 | 1 | -9.69 | no |
| Long | 2023-05-06 | 2023-05-07 | 1 | -9.15 | no |
| Short | 2023-05-07 | 2023-05-20 | 13 | 23.82 | no |
| Long | 2023-05-20 | 2023-05-22 | 2 | -3.04 | no |
| Short | 2023-05-22 | 2023-05-24 | 2 | -7.7 | no |
| Long | 2023-05-24 | 2023-05-25 | 1 | -13.05 | no |
| Short | 2023-05-25 | 2023-05-29 | 4 | -23.59 | no |
| Long | 2023-05-29 | 2023-06-05 | 7 | -12.09 | no |
| Short | 2023-06-05 | 2023-06-13 | 8 | 11.3 | no |
| Long | 2023-06-13 | 2023-06-14 | 1 | -0.61 | no |
| Short | 2023-06-14 | 2023-06-18 | 4 | -8.88 | no |
| Long | 2023-06-18 | 2023-07-01 | 13 | 46.54 | no |
| Short | 2023-07-01 | 2023-07-02 | 1 | -2.42 | no |
| Long | 2023-07-02 | 2023-07-07 | 5 | -9.27 | no |
| Short | 2023-07-07 | 2023-07-14 | 7 | -19.1 | no |
| Long | 2023-07-14 | 2023-07-15 | 1 | -13.1 | no |
| Short | 2023-07-15 | 2023-07-17 | 2 | -0.09 | no |
| Long | 2023-07-17 | 2023-07-18 | 1 | -1.92 | no |
| Short | 2023-07-18 | 2023-08-01 | 14 | 8.54 | no |
| Long | 2023-08-01 | 2023-08-03 | 2 | -1.57 | no |
| Short | 2023-08-03 | 2023-08-09 | 6 | -7.57 | no |
| Long | 2023-08-09 | 2023-08-16 | 7 | -7.06 | no |
| Short | 2023-08-16 | 2023-08-26 | 10 | 34.36 | no |
| Long | 2023-08-26 | 2023-08-27 | 1 | -1.58 | no |
| Short | 2023-08-27 | 2023-08-30 | 3 | -23.3 | no |
| Long | 2023-08-30 | 2023-09-01 | 2 | -21.51 | no |
| Short | 2023-09-01 | 2023-09-08 | 7 | -4.6 | no |
| Long | 2023-09-08 | 2023-09-11 | 3 | -5.52 | no |
| Short | 2023-09-11 | 2023-09-13 | 2 | -1.25 | no |
| Long | 2023-09-13 | 2023-09-23 | 10 | 7.37 | no |
| Short | 2023-09-23 | 2023-09-24 | 1 | -0.87 | no |
| Long | 2023-09-24 | 2023-09-25 | 1 | -4.5 | no |
| Short | 2023-09-25 | 2023-09-29 | 4 | -9.33 | no |
| Long | 2023-09-29 | 2023-10-09 | 10 | 8.49 | no |
| Short | 2023-10-09 | 2023-10-10 | 1 | 2.53 | no |
| Long | 2023-10-10 | 2023-10-11 | 1 | -3.02 | no |
| Short | 2023-10-11 | 2023-10-17 | 6 | -12.98 | no |
| Long | 2023-10-17 | 2023-11-17 | 31 | 73.32 | no |
| Short | 2023-11-17 | 2023-11-20 | 3 | -12.39 | no |
| Long | 2023-11-20 | 2023-11-23 | 3 | -0.56 | no |
| Short | 2023-11-23 | 2023-11-24 | 1 | 0.04 | no |
| Long | 2023-11-24 | 2023-11-28 | 4 | -1.44 | no |
| Short | 2023-11-28 | 2023-11-29 | 1 | -6.18 | no |
| Long | 2023-11-29 | 2023-12-12 | 13 | 28.8 | no |
| Short | 2023-12-12 | 2023-12-19 | 7 | -13.98 | no |
| Long | 2023-12-19 | 2023-12-28 | 9 | 5.22 | no |
| Short | 2023-12-28 | 2024-01-02 | 5 | -7.77 | no |
| Long | 2024-01-02 | 2024-01-04 | 2 | -11.31 | no |
| Short | 2024-01-04 | 2024-01-05 | 1 | -11.08 | no |
| Long | 2024-01-05 | 2024-01-13 | 8 | -10.82 | no |
| Short | 2024-01-13 | 2024-01-27 | 14 | 3.93 | no |
| Long | 2024-01-27 | 2024-02-06 | 10 | 5.66 | no |
| Short | 2024-02-06 | 2024-02-07 | 1 | -3.86 | no |
| Long | 2024-02-07 | 2024-02-23 | 16 | 58.59 | no |
| Short | 2024-02-23 | 2024-02-27 | 4 | -23.95 | no |
| Long | 2024-02-27 | 2024-03-17 | 19 | 68.13 | no |
| Short | 2024-03-17 | 2024-03-26 | 9 | -37.82 | no |
| Long | 2024-03-26 | 2024-04-02 | 7 | -2.31 | no |
| Short | 2024-04-02 | 2024-04-09 | 7 | -13.88 | no |
| Long | 2024-04-09 | 2024-04-13 | 4 | -23.89 | no |
| Short | 2024-04-13 | 2024-04-21 | 8 | 7.75 | no |
| Long | 2024-04-21 | 2024-04-22 | 1 | -1 | no |
| Short | 2024-04-22 | 2024-04-23 | 1 | -10.96 | no |
| Long | 2024-04-23 | 2024-04-27 | 4 | -16.2 | no |
| Short | 2024-04-27 | 2024-05-05 | 8 | -4.33 | no |
| Long | 2024-05-05 | 2024-05-07 | 2 | -4.5 | no |
| Short | 2024-05-07 | 2024-05-08 | 1 | 3.24 | no |
| Long | 2024-05-08 | 2024-05-11 | 3 | -8.52 | no |
| Short | 2024-05-11 | 2024-05-16 | 5 | -27.62 | no |
| Long | 2024-05-16 | 2024-05-28 | 12 | 12.59 | no |
| Short | 2024-05-28 | 2024-05-31 | 3 | 3.55 | no |
| Long | 2024-05-31 | 2024-06-01 | 1 | -4.36 | no |
| Short | 2024-06-01 | 2024-06-05 | 4 | -13.34 | no |
| Long | 2024-06-05 | 2024-06-12 | 7 | -13.39 | no |
| Short | 2024-06-12 | 2024-07-02 | 20 | 15.79 | no |
| Long | 2024-07-02 | 2024-07-04 | 2 | -12.66 | no |
| Short | 2024-07-04 | 2024-07-12 | 8 | 10.75 | no |
| Long | 2024-07-12 | 2024-07-30 | 18 | 44.46 | no |
| Short | 2024-07-30 | 2024-07-31 | 1 | 1.87 | no |
| Long | 2024-07-31 | 2024-08-01 | 1 | -8.55 | no |
| Short | 2024-08-01 | 2024-08-11 | 10 | 9.32 | no |
| Long | 2024-08-11 | 2024-08-16 | 5 | -18.85 | no |
| Short | 2024-08-16 | 2024-08-20 | 4 | -10.94 | no |
| Long | 2024-08-20 | 2024-08-21 | 1 | -2.95 | no |
| Short | 2024-08-21 | 2024-08-22 | 1 | -11.34 | no |
| Long | 2024-08-22 | 2024-08-29 | 7 | -10.45 | no |
| Short | 2024-08-29 | 2024-09-11 | 13 | 3.73 | no |
| Long | 2024-09-11 | 2024-09-12 | 1 | -2.24 | no |
| Short | 2024-09-12 | 2024-09-13 | 1 | -4.53 | no |
| Long | 2024-09-13 | 2024-10-01 | 18 | 22.87 | no |
| Short | 2024-10-01 | 2024-10-09 | 8 | 3.85 | no |
| Long | 2024-10-09 | 2024-10-10 | 1 | -8.02 | no |
| Short | 2024-10-10 | 2024-10-12 | 2 | -9.82 | no |
| Long | 2024-10-12 | 2024-10-21 | 10 | 20.79 | yes |

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

