# BTC time-series momentum — Volatility targeting / Flat / full / base

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
| Split | full |
| Candles | 1824 |
| Range | 2021-10-24 .. 2026-10-21 |
| Variant | Volatility targeting |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -18.47% | 36.79% |
| CAGR | -4.00% | 6.47% |
| Max drawdown | 34.41% | 76.63% |
| Sharpe | -0.13 | 0.38 |
| Sortino | -0.20 | 0.55 |
| Win rate | 27.49% | 100.00% |
| Trades | 171 | 1 |
| Average holding days | 5.42 | 1824.00 |
| Turnover (initial equity) | 218.66 | 2.37 |
| Trading costs | 327.99 | 3.55 |
| Borrow costs | 0 | — |
| Average exposure | 29.77% | 100.00% |
| Average absolute exposure | 29.77% | 100.00% |
| Days with exposure | 926 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 69.09% | 1.12 | 1.89 | 18.04% | 34.46% | 34.46% |
| Bear | 707 | -45.58% | -1.94 | -2.59 | 45.66% | 27.14% | 27.14% |
| Unknown | 199 | -11.39% | -1.23 | -1.67 | 16.29% | 17.43% | 17.43% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2021-11-01 | 2021-11-02 | 1 | -3.24 | no |
| Long | 2021-11-03 | 2021-11-06 | 3 | -19.58 | no |
| Long | 2021-11-08 | 2021-11-16 | 8 | 10.11 | no |
| Long | 2021-11-26 | 2021-11-27 | 1 | -36.97 | no |
| Long | 2021-11-30 | 2021-12-01 | 1 | -4.11 | no |
| Long | 2021-12-02 | 2021-12-03 | 1 | -3.6 | no |
| Long | 2021-12-12 | 2021-12-14 | 2 | -18.53 | no |
| Long | 2021-12-17 | 2021-12-18 | 1 | -9.04 | no |
| Long | 2021-12-21 | 2021-12-23 | 2 | 14.89 | no |
| Long | 2021-12-24 | 2021-12-29 | 5 | -31.8 | no |
| Long | 2022-01-13 | 2022-01-14 | 1 | -16.39 | no |
| Long | 2022-01-15 | 2022-01-19 | 4 | -10.37 | no |
| Long | 2022-01-29 | 2022-02-14 | 16 | 24.54 | no |
| Long | 2022-02-16 | 2022-02-17 | 1 | -7.86 | no |
| Long | 2022-03-01 | 2022-03-05 | 4 | -16.8 | no |
| Long | 2022-03-06 | 2022-03-08 | 2 | -5.6 | no |
| Long | 2022-03-15 | 2022-03-17 | 2 | 6.93 | no |
| Long | 2022-03-18 | 2022-04-04 | 17 | 59.38 | no |
| Long | 2022-04-19 | 2022-04-23 | 4 | -16.96 | no |
| Long | 2022-05-05 | 2022-05-06 | 1 | -28.93 | no |
| Long | 2022-05-20 | 2022-05-21 | 1 | -9.16 | no |
| Long | 2022-05-26 | 2022-05-27 | 1 | -4.25 | no |
| Long | 2022-05-31 | 2022-06-07 | 7 | -8.27 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -2.27 | no |
| Long | 2022-06-24 | 2022-06-29 | 5 | -15.18 | no |
| Long | 2022-06-30 | 2022-07-01 | 1 | -4.38 | no |
| Long | 2022-07-07 | 2022-07-12 | 5 | -10.17 | no |
| Long | 2022-07-19 | 2022-07-26 | 7 | -15.69 | no |
| Long | 2022-07-29 | 2022-08-04 | 6 | -11.48 | no |
| Long | 2022-08-09 | 2022-08-18 | 9 | -16.07 | no |
| Long | 2022-09-05 | 2022-09-06 | 1 | -6.23 | no |
| Long | 2022-09-10 | 2022-09-17 | 7 | -13.76 | no |
| Long | 2022-09-28 | 2022-10-11 | 13 | 2.08 | no |
| Long | 2022-10-18 | 2022-10-20 | 2 | -18.91 | no |
| Long | 2022-10-23 | 2022-10-25 | 2 | 3.03 | no |
| Long | 2022-10-26 | 2022-11-03 | 8 | 0.27 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -49.69 | no |
| Long | 2022-11-17 | 2022-11-18 | 1 | -0.24 | no |
| Long | 2022-11-28 | 2022-12-08 | 10 | 19.5 | no |
| Long | 2022-12-09 | 2022-12-12 | 3 | -6.67 | no |
| Long | 2022-12-13 | 2022-12-17 | 4 | -13.17 | no |
| Long | 2022-12-24 | 2022-12-28 | 4 | -6.12 | no |
| Long | 2023-01-05 | 2023-01-31 | 26 | 158.21 | no |
| Long | 2023-02-01 | 2023-02-06 | 5 | -4.14 | no |
| Long | 2023-02-08 | 2023-02-09 | 1 | -9.12 | no |
| Long | 2023-02-16 | 2023-02-23 | 7 | -0.92 | no |
| Long | 2023-02-24 | 2023-02-25 | 1 | -14.29 | no |
| Long | 2023-03-14 | 2023-03-28 | 14 | 22.55 | no |
| Long | 2023-03-30 | 2023-03-31 | 1 | -4.81 | no |
| Long | 2023-04-01 | 2023-04-06 | 5 | -4.89 | no |
| Long | 2023-04-07 | 2023-04-08 | 1 | -6.25 | no |
| Long | 2023-04-10 | 2023-04-18 | 8 | 32.65 | no |
| Long | 2023-04-19 | 2023-04-20 | 1 | -27.61 | no |
| Long | 2023-04-28 | 2023-05-05 | 7 | -17.31 | no |
| Long | 2023-05-06 | 2023-05-07 | 1 | -10.84 | no |
| Long | 2023-05-20 | 2023-05-22 | 2 | -6.15 | no |
| Long | 2023-05-24 | 2023-05-25 | 1 | -24.02 | no |
| Long | 2023-05-29 | 2023-06-05 | 7 | -14.77 | no |
| Long | 2023-06-13 | 2023-06-14 | 1 | -0.56 | no |
| Long | 2023-06-18 | 2023-07-01 | 13 | 54.48 | no |
| Long | 2023-07-02 | 2023-07-07 | 5 | -19.54 | no |
| Long | 2023-07-14 | 2023-07-15 | 1 | -24.55 | no |
| Long | 2023-07-17 | 2023-07-18 | 1 | -2.45 | no |
| Long | 2023-08-01 | 2023-08-03 | 2 | -3.69 | no |
| Long | 2023-08-09 | 2023-08-16 | 7 | -17.05 | no |
| Long | 2023-08-26 | 2023-08-27 | 1 | -3.43 | no |
| Long | 2023-08-30 | 2023-09-01 | 2 | -23.34 | no |
| Long | 2023-09-08 | 2023-09-11 | 3 | -13.57 | no |
| Long | 2023-09-13 | 2023-09-23 | 10 | 8.66 | no |
| Long | 2023-09-24 | 2023-09-25 | 1 | -10.9 | no |
| Long | 2023-09-29 | 2023-10-09 | 10 | 20.7 | no |
| Long | 2023-10-10 | 2023-10-11 | 1 | -7.18 | no |
| Long | 2023-10-17 | 2023-11-17 | 31 | 121.96 | no |
| Long | 2023-11-20 | 2023-11-23 | 3 | -1.31 | no |
| Long | 2023-11-24 | 2023-11-28 | 4 | -1.53 | no |
| Long | 2023-11-29 | 2023-12-12 | 13 | 86.38 | no |
| Long | 2023-12-19 | 2023-12-28 | 9 | 3.43 | no |
| Long | 2024-01-02 | 2024-01-04 | 2 | -16.92 | no |
| Long | 2024-01-05 | 2024-01-13 | 8 | -12.77 | no |
| Long | 2024-01-27 | 2024-02-06 | 10 | -0.45 | no |
| Long | 2024-02-07 | 2024-02-23 | 16 | 132.27 | no |
| Long | 2024-02-27 | 2024-03-17 | 19 | 56.7 | no |
| Long | 2024-03-26 | 2024-04-02 | 7 | -6.81 | no |
| Long | 2024-04-09 | 2024-04-13 | 4 | -30.33 | no |
| Long | 2024-04-21 | 2024-04-22 | 1 | -1.18 | no |
| Long | 2024-04-23 | 2024-04-27 | 4 | -25.09 | no |
| Long | 2024-05-05 | 2024-05-07 | 2 | -4.41 | no |
| Long | 2024-05-08 | 2024-05-11 | 3 | -9.4 | no |
| Long | 2024-05-16 | 2024-05-28 | 12 | 17.92 | no |
| Long | 2024-05-31 | 2024-06-01 | 1 | -13.85 | no |
| Long | 2024-06-05 | 2024-06-12 | 7 | -43.96 | no |
| Long | 2024-07-02 | 2024-07-04 | 2 | -27.86 | no |
| Long | 2024-07-12 | 2024-07-30 | 18 | 85.94 | no |
| Long | 2024-07-31 | 2024-08-01 | 1 | -18.54 | no |
| Long | 2024-08-11 | 2024-08-16 | 5 | -12.55 | no |
| Long | 2024-08-20 | 2024-08-21 | 1 | -5.03 | no |
| Long | 2024-08-22 | 2024-08-29 | 7 | -6.7 | no |
| Long | 2024-09-11 | 2024-09-12 | 1 | -3.38 | no |
| Long | 2024-09-13 | 2024-10-01 | 18 | 40.59 | no |
| Long | 2024-10-09 | 2024-10-10 | 1 | -28.24 | no |
| Long | 2024-10-12 | 2024-10-24 | 12 | 27.22 | no |
| Long | 2024-10-25 | 2024-10-26 | 1 | -18.2 | no |
| Long | 2024-10-29 | 2024-11-05 | 7 | -12.36 | no |
| Long | 2024-11-07 | 2024-11-27 | 20 | 54.45 | no |
| Long | 2024-11-28 | 2024-11-29 | 1 | -1.9 | no |
| Long | 2024-12-03 | 2024-12-19 | 16 | 11.41 | no |
| Long | 2025-01-03 | 2025-01-10 | 7 | -16.56 | no |
| Long | 2025-01-16 | 2025-01-28 | 12 | 8.51 | no |
| Long | 2025-01-30 | 2025-02-01 | 2 | -12.8 | no |
| Long | 2025-02-13 | 2025-02-17 | 4 | -19.82 | no |
| Long | 2025-02-21 | 2025-02-22 | 1 | -25 | no |
| Long | 2025-02-24 | 2025-02-25 | 1 | -47.33 | no |
| Long | 2025-03-06 | 2025-03-10 | 4 | -22.11 | no |
| Long | 2025-03-17 | 2025-03-19 | 2 | -0.95 | no |
| Long | 2025-03-20 | 2025-03-23 | 3 | -13.38 | no |
| Long | 2025-03-24 | 2025-03-30 | 6 | -36.34 | no |
| Long | 2025-04-06 | 2025-04-07 | 1 | -33.59 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -8.42 | no |
| Long | 2025-04-13 | 2025-04-20 | 7 | 1.6 | no |
| Long | 2025-04-21 | 2025-05-06 | 15 | 81.03 | no |
| Long | 2025-05-07 | 2025-05-18 | 11 | 36.9 | no |
| Long | 2025-05-19 | 2025-05-29 | 10 | 7.41 | no |
| Long | 2025-06-07 | 2025-06-15 | 8 | 11.84 | no |
| Long | 2025-06-25 | 2025-07-02 | 7 | -11.51 | no |
| Long | 2025-07-03 | 2025-07-21 | 18 | 49.73 | no |
| Long | 2025-07-23 | 2025-07-25 | 2 | -16.65 | no |
| Long | 2025-07-27 | 2025-07-30 | 3 | -2.74 | no |
| Long | 2025-08-08 | 2025-08-18 | 10 | 7.01 | no |
| Long | 2025-08-29 | 2025-08-30 | 1 | -19.61 | no |
| Long | 2025-09-04 | 2025-09-05 | 1 | -6.99 | no |
| Long | 2025-09-06 | 2025-09-20 | 14 | 40.58 | no |
| Long | 2025-09-30 | 2025-10-11 | 11 | -24.87 | no |
| Long | 2025-10-24 | 2025-10-31 | 7 | -12.81 | no |
| Long | 2025-11-12 | 2025-11-13 | 1 | -8.03 | no |
| Long | 2025-11-28 | 2025-12-02 | 4 | -39.09 | no |
| Long | 2025-12-03 | 2025-12-06 | 3 | -8.17 | no |
| Long | 2025-12-08 | 2025-12-11 | 3 | 3.78 | no |
| Long | 2025-12-12 | 2025-12-15 | 3 | -33.06 | no |
| Long | 2025-12-22 | 2025-12-24 | 2 | -10.06 | no |
| Long | 2025-12-25 | 2025-12-27 | 2 | -4.68 | no |
| Long | 2025-12-31 | 2026-01-01 | 1 | -11.08 | no |
| Long | 2026-01-02 | 2026-01-11 | 9 | 15.98 | no |
| Long | 2026-01-14 | 2026-01-21 | 7 | -38.91 | no |
| Long | 2026-01-28 | 2026-01-29 | 1 | -1.49 | no |
| Long | 2026-02-13 | 2026-02-14 | 1 | 6.91 | no |
| Long | 2026-02-15 | 2026-02-16 | 1 | -7.28 | no |
| Long | 2026-02-20 | 2026-02-21 | 1 | 5.57 | no |
| Long | 2026-02-26 | 2026-02-28 | 2 | -10.35 | no |
| Long | 2026-03-03 | 2026-03-10 | 7 | -1.04 | no |
| Long | 2026-03-11 | 2026-03-12 | 1 | 0.14 | no |
| Long | 2026-03-14 | 2026-03-20 | 6 | -12.29 | no |
| Long | 2026-03-26 | 2026-03-27 | 1 | -15.2 | no |
| Long | 2026-04-04 | 2026-04-30 | 26 | 57.58 | no |
| Long | 2026-05-02 | 2026-05-04 | 2 | 0.89 | no |
| Long | 2026-05-05 | 2026-05-13 | 8 | 3.18 | no |
| Long | 2026-05-15 | 2026-05-16 | 1 | -17.98 | no |
| Long | 2026-05-26 | 2026-05-27 | 1 | -14.81 | no |
| Long | 2026-06-13 | 2026-06-19 | 6 | -12.17 | no |
| Long | 2026-07-03 | 2026-07-14 | 11 | -2.96 | no |
| Long | 2026-07-15 | 2026-07-18 | 3 | -8.82 | no |
| Long | 2026-07-19 | 2026-07-26 | 7 | -15.52 | no |
| Long | 2026-07-27 | 2026-07-28 | 1 | -17.72 | no |
| Long | 2026-08-05 | 2026-08-07 | 2 | 0.38 | no |
| Long | 2026-08-08 | 2026-08-12 | 4 | -18.15 | no |
| Long | 2026-08-18 | 2026-08-29 | 11 | 91.24 | no |
| Long | 2026-08-30 | 2026-08-31 | 1 | -6.01 | no |
| Long | 2026-09-04 | 2026-09-11 | 7 | -24.73 | no |
| Long | 2026-09-19 | 2026-09-29 | 10 | 11.49 | no |
| Long | 2026-10-02 | 2026-10-08 | 6 | -17.84 | no |
| Long | 2026-10-16 | 2026-10-17 | 1 | -6.52 | no |
| Long | 2026-10-20 | 2026-10-21 | 1 | 8.36 | no |

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

