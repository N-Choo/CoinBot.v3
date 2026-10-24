# BTC time-series momentum — Volatility targeting / Short / full / stress

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
| Momentum lookback | 30 days |
| Volatility lookback | 30 days |
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
| Split | full |
| Candles | 1824 |
| Range | 2021-10-24 .. 2026-10-21 |
| Variant | Volatility targeting |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 26.16% | 36.38% |
| CAGR | 4.76% | 6.41% |
| Max drawdown | 38.52% | 76.63% |
| Sharpe | 0.32 | 0.38 |
| Sortino | 0.48 | 0.55 |
| Win rate | 26.88% | 100.00% |
| Trades | 160 | 1 |
| Average holding days | 11.21 | 1824.00 |
| Turnover (initial equity) | 209.33 | 2.37 |
| Trading costs | 627.98 | 7.1 |
| Borrow costs | 0 | — |
| Average exposure | 5.05% | 100.00% |
| Average absolute exposure | 46.02% | 100.00% |
| Days with exposure | 1793 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 72.14% | 1.05 | 1.67 | 22.27% | 16.41% | 49.64% |
| Bear | 707 | -25.08% | -0.55 | -0.81 | 35.70% | -4.76% | 46.38% |
| Unknown | 199 | -2.18% | -0.09 | -0.13 | 24.60% | -12.47% | 28.07% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Short | 2021-11-24 | 2022-02-07 | 75 | 100.61 | no |
| Long | 2022-02-07 | 2022-02-12 | 5 | -1.8 | no |
| Short | 2022-02-12 | 2022-02-16 | 4 | -19.66 | no |
| Long | 2022-02-16 | 2022-02-18 | 2 | -31.83 | no |
| Short | 2022-02-18 | 2022-02-21 | 3 | 13.77 | no |
| Long | 2022-02-21 | 2022-02-28 | 7 | -7.33 | no |
| Short | 2022-02-28 | 2022-03-01 | 1 | -49.27 | no |
| Long | 2022-03-01 | 2022-03-07 | 6 | -29.03 | no |
| Short | 2022-03-07 | 2022-03-20 | 13 | -27.18 | no |
| Long | 2022-03-20 | 2022-04-16 | 27 | -30.18 | no |
| Short | 2022-04-16 | 2022-04-20 | 4 | -11.65 | no |
| Long | 2022-04-20 | 2022-04-22 | 2 | -11.72 | no |
| Short | 2022-04-22 | 2022-06-09 | 48 | 82.46 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -2.74 | no |
| Short | 2022-06-10 | 2022-07-17 | 37 | 86.84 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | -3.4 | no |
| Short | 2022-07-26 | 2022-07-27 | 1 | -1.28 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 30.26 | no |
| Short | 2022-08-19 | 2022-08-25 | 6 | 35.78 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -23.67 | no |
| Short | 2022-08-27 | 2022-10-05 | 39 | -19.08 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -16.98 | no |
| Short | 2022-10-10 | 2022-10-21 | 11 | 7.53 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 1.51 | no |
| Short | 2022-10-23 | 2022-10-24 | 1 | -20.35 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 21.77 | no |
| Short | 2022-11-04 | 2022-11-05 | 1 | -40.68 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -85.27 | no |
| Short | 2022-11-09 | 2022-12-10 | 31 | 28.51 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -2.39 | no |
| Short | 2022-12-11 | 2022-12-12 | 1 | -2.02 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -19.38 | no |
| Short | 2022-12-17 | 2022-12-18 | 1 | -8 | no |
| Long | 2022-12-18 | 2022-12-20 | 2 | -13.9 | no |
| Short | 2022-12-20 | 2022-12-21 | 1 | -17.3 | no |
| Long | 2022-12-21 | 2022-12-31 | 10 | -12.86 | no |
| Short | 2022-12-31 | 2023-01-07 | 7 | -17.71 | no |
| Long | 2023-01-07 | 2023-01-08 | 1 | -4.32 | no |
| Short | 2023-01-08 | 2023-01-09 | 1 | -11.6 | no |
| Long | 2023-01-09 | 2023-03-01 | 51 | 189.49 | no |
| Short | 2023-03-01 | 2023-03-02 | 1 | -11.31 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -25.63 | no |
| Short | 2023-03-04 | 2023-03-13 | 9 | 1.89 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 63.21 | no |
| Short | 2023-04-23 | 2023-04-24 | 1 | 1.1 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 5.73 | no |
| Short | 2023-05-02 | 2023-05-03 | 1 | -14.38 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -19.96 | no |
| Short | 2023-05-09 | 2023-06-21 | 43 | -23.86 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 20.63 | no |
| Short | 2023-07-22 | 2023-09-18 | 58 | 100.91 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 8.58 | no |
| Short | 2023-09-29 | 2023-10-01 | 2 | -3.28 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 374.83 | no |
| Short | 2024-01-08 | 2024-01-09 | 1 | -52.59 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -62.74 | no |
| Short | 2024-01-13 | 2024-01-16 | 3 | -0.19 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -18.89 | no |
| Short | 2024-01-19 | 2024-01-30 | 11 | -28.54 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -11.28 | no |
| Short | 2024-02-01 | 2024-02-03 | 2 | -10.53 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -5.57 | no |
| Short | 2024-02-04 | 2024-02-10 | 6 | -57.18 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 266.37 | no |
| Short | 2024-04-04 | 2024-04-05 | 1 | -18.77 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 11.54 | no |
| Short | 2024-04-11 | 2024-04-19 | 8 | 42.51 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | -0.3 | no |
| Short | 2024-04-20 | 2024-04-22 | 2 | -13.66 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 9.94 | no |
| Short | 2024-04-24 | 2024-05-16 | 22 | -14.8 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -5.2 | no |
| Short | 2024-06-15 | 2024-06-16 | 1 | -7.25 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 0.67 | no |
| Short | 2024-06-17 | 2024-07-20 | 33 | 25.11 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -135.49 | no |
| Short | 2024-08-06 | 2024-08-07 | 1 | -22.83 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -11.5 | no |
| Short | 2024-08-08 | 2024-08-09 | 1 | -65.36 | no |
| Long | 2024-08-09 | 2024-08-14 | 5 | -9.66 | no |
| Short | 2024-08-14 | 2024-09-05 | 22 | 8.74 | no |
| Long | 2024-09-05 | 2024-09-07 | 2 | -33.8 | no |
| Short | 2024-09-07 | 2024-09-14 | 7 | -65.9 | no |
| Long | 2024-09-14 | 2024-09-17 | 3 | -22.91 | no |
| Short | 2024-09-17 | 2024-09-18 | 1 | -21.51 | no |
| Long | 2024-09-18 | 2024-09-23 | 5 | 23.3 | no |
| Short | 2024-09-23 | 2024-09-25 | 2 | -9.63 | no |
| Long | 2024-09-25 | 2024-12-22 | 88 | 255.16 | no |
| Short | 2024-12-22 | 2024-12-25 | 3 | -13.84 | no |
| Long | 2024-12-25 | 2024-12-28 | 3 | -33.43 | no |
| Short | 2024-12-28 | 2025-01-03 | 6 | -22.82 | no |
| Long | 2025-01-03 | 2025-01-04 | 1 | 3.92 | no |
| Short | 2025-01-04 | 2025-01-05 | 1 | -4.14 | no |
| Long | 2025-01-05 | 2025-01-06 | 1 | -2.93 | no |
| Short | 2025-01-06 | 2025-01-07 | 1 | -30.04 | no |
| Long | 2025-01-07 | 2025-01-08 | 1 | -35.58 | no |
| Short | 2025-01-08 | 2025-01-18 | 10 | -48.07 | no |
| Long | 2025-01-18 | 2025-02-03 | 16 | -41.49 | no |
| Short | 2025-02-03 | 2025-02-04 | 1 | -24.83 | no |
| Long | 2025-02-04 | 2025-02-05 | 1 | -22.2 | no |
| Short | 2025-02-05 | 2025-02-08 | 3 | 3.1 | no |
| Long | 2025-02-08 | 2025-02-15 | 7 | 2.63 | no |
| Short | 2025-02-15 | 2025-03-29 | 42 | 69.19 | no |
| Long | 2025-03-29 | 2025-03-30 | 1 | -10.32 | no |
| Short | 2025-03-30 | 2025-04-10 | 11 | -7.11 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -18.93 | no |
| Short | 2025-04-11 | 2025-04-13 | 2 | -33.95 | no |
| Long | 2025-04-13 | 2025-04-14 | 1 | -10.13 | no |
| Short | 2025-04-14 | 2025-04-15 | 1 | -6.85 | no |
| Long | 2025-04-15 | 2025-04-19 | 4 | -3.17 | no |
| Short | 2025-04-19 | 2025-04-20 | 1 | -6.04 | no |
| Long | 2025-04-20 | 2025-06-18 | 59 | 94.03 | no |
| Short | 2025-06-18 | 2025-06-29 | 11 | -31.38 | no |
| Long | 2025-06-29 | 2025-08-16 | 48 | 76.81 | no |
| Short | 2025-08-16 | 2025-09-18 | 33 | -1.29 | no |
| Long | 2025-09-18 | 2025-09-22 | 4 | -15.47 | no |
| Short | 2025-09-22 | 2025-09-25 | 3 | 10.75 | no |
| Long | 2025-09-25 | 2025-09-26 | 1 | -50.33 | no |
| Short | 2025-09-26 | 2025-09-29 | 3 | -33.96 | no |
| Long | 2025-09-29 | 2025-10-11 | 12 | 12.27 | no |
| Short | 2025-10-11 | 2025-10-26 | 15 | -0.28 | no |
| Long | 2025-10-26 | 2025-10-30 | 4 | -11.44 | no |
| Short | 2025-10-30 | 2025-12-21 | 52 | 126.98 | no |
| Long | 2025-12-21 | 2025-12-25 | 4 | -9.38 | no |
| Short | 2025-12-25 | 2026-01-01 | 7 | -4.56 | no |
| Long | 2026-01-01 | 2026-01-02 | 1 | 6.01 | no |
| Short | 2026-01-02 | 2026-01-05 | 3 | -35.52 | no |
| Long | 2026-01-05 | 2026-01-09 | 4 | -9.8 | no |
| Short | 2026-01-09 | 2026-01-12 | 3 | -5.03 | no |
| Long | 2026-01-12 | 2026-01-21 | 9 | -35.05 | no |
| Short | 2026-01-21 | 2026-01-22 | 1 | -14.75 | no |
| Long | 2026-01-22 | 2026-01-26 | 4 | -30.81 | no |
| Short | 2026-01-26 | 2026-01-27 | 1 | -19.38 | no |
| Long | 2026-01-27 | 2026-01-30 | 3 | -35.03 | no |
| Short | 2026-01-30 | 2026-03-08 | 37 | 119.04 | no |
| Long | 2026-03-08 | 2026-03-09 | 1 | -10.12 | no |
| Short | 2026-03-09 | 2026-03-12 | 3 | -34.79 | no |
| Long | 2026-03-12 | 2026-03-23 | 11 | -18.69 | no |
| Short | 2026-03-23 | 2026-03-24 | 1 | -24.08 | no |
| Long | 2026-03-24 | 2026-03-28 | 4 | -32.35 | no |
| Short | 2026-03-28 | 2026-03-30 | 2 | -0.05 | no |
| Long | 2026-03-30 | 2026-03-31 | 1 | 2.74 | no |
| Short | 2026-03-31 | 2026-04-01 | 1 | -13.35 | no |
| Long | 2026-04-01 | 2026-04-02 | 1 | -3.92 | no |
| Short | 2026-04-02 | 2026-04-06 | 4 | -12.33 | no |
| Long | 2026-04-06 | 2026-04-13 | 7 | 11.54 | no |
| Short | 2026-04-13 | 2026-04-14 | 1 | -32.29 | no |
| Long | 2026-04-14 | 2026-04-16 | 2 | -0.25 | no |
| Short | 2026-04-16 | 2026-04-17 | 1 | -5.39 | no |
| Long | 2026-04-17 | 2026-05-23 | 36 | -15.85 | no |
| Short | 2026-05-23 | 2026-07-06 | 44 | 123.27 | no |
| Long | 2026-07-06 | 2026-07-09 | 3 | -20.57 | no |
| Short | 2026-07-09 | 2026-07-10 | 1 | -15.59 | no |
| Long | 2026-07-10 | 2026-07-14 | 4 | -16.13 | no |
| Short | 2026-07-14 | 2026-07-19 | 5 | -35.73 | no |
| Long | 2026-07-19 | 2026-08-12 | 24 | -20.45 | no |
| Short | 2026-08-12 | 2026-08-13 | 1 | -3.45 | no |
| Long | 2026-08-13 | 2026-08-14 | 1 | -5 | no |
| Short | 2026-08-14 | 2026-08-19 | 5 | -26.14 | no |
| Long | 2026-08-19 | 2026-10-21 | 64 | 193.58 | yes |

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
- Signals require 31 closed daily candles (max of momentum and volatility lookback + 1); earlier days are flat.
- The 200-day BTC regime label needs 200 closes and is reported as unknown before that.
- Splits are chronological reporting partitions. No parameter tuning or model selection is performed on them.
- Fees and slippage are modelled as a single per-side rate on traded notional; market impact is not modelled.
- Open episodes at the end of a split are marked at the final close and counted as closed for win-rate reporting without an extra liquidation cost.
- The drawdown variant evaluates the 20%/10% hysteresis on the uncapped variant-B reference equity path so the control can recover while the live portfolio is flat.

## Data issues

No data issues were recorded.

