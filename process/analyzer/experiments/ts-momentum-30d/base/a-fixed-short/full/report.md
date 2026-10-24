# BTC time-series momentum — Fixed exposure / Short / full / base

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
| Variant | Fixed exposure |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 36.89% | 36.79% |
| CAGR | 6.48% | 6.47% |
| Max drawdown | 60.95% | 76.63% |
| Sharpe | 0.38 | 0.38 |
| Sortino | 0.56 | 0.55 |
| Win rate | 30.63% | 100.00% |
| Trades | 160 | 1 |
| Average holding days | 11.21 | 1824.00 |
| Turnover (initial equity) | 422.19 | 2.37 |
| Trading costs | 633.28 | 3.55 |
| Borrow costs | 0 | — |
| Average exposure | 5.65% | 100.00% |
| Average absolute exposure | 98.30% | 100.00% |
| Days with exposure | 1793 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 153.39% | 1.02 | 1.58 | 41.85% | 37.25% | 100.00% |
| Bear | 707 | -35.80% | -0.19 | -0.27 | 59.60% | -23.90% | 100.00% |
| Unknown | 199 | -15.86% | -0.21 | -0.30 | 56.52% | -35.18% | 84.42% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Short | 2021-11-24 | 2022-02-07 | 75 | 243.56 | no |
| Long | 2022-02-07 | 2022-02-12 | 5 | -4.45 | no |
| Short | 2022-02-12 | 2022-02-16 | 4 | -67.38 | no |
| Long | 2022-02-16 | 2022-02-18 | 2 | -109.85 | no |
| Short | 2022-02-18 | 2022-02-21 | 3 | 53.27 | no |
| Long | 2022-02-21 | 2022-02-28 | 7 | -22.85 | no |
| Short | 2022-02-28 | 2022-03-01 | 1 | -161.24 | no |
| Long | 2022-03-01 | 2022-03-07 | 6 | -105.04 | no |
| Short | 2022-03-07 | 2022-03-20 | 13 | -89.32 | no |
| Long | 2022-03-20 | 2022-04-16 | 27 | -31.07 | no |
| Short | 2022-04-16 | 2022-04-20 | 4 | -19.08 | no |
| Long | 2022-04-20 | 2022-04-22 | 2 | -18.88 | no |
| Short | 2022-04-22 | 2022-06-09 | 48 | 165.67 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -5.18 | no |
| Short | 2022-06-10 | 2022-07-17 | 37 | 263.75 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | 2.71 | no |
| Short | 2022-07-26 | 2022-07-27 | 1 | -0.47 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 96.12 | no |
| Short | 2022-08-19 | 2022-08-25 | 6 | 83.63 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -70.8 | no |
| Short | 2022-08-27 | 2022-10-05 | 39 | -54.89 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -53.9 | no |
| Short | 2022-10-10 | 2022-10-21 | 11 | 18.37 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 6.02 | no |
| Short | 2022-10-23 | 2022-10-24 | 1 | -24.53 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 32.37 | no |
| Short | 2022-11-04 | 2022-11-05 | 1 | -55.43 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -134.31 | no |
| Short | 2022-11-09 | 2022-12-10 | 31 | 34.14 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -2.85 | no |
| Short | 2022-12-11 | 2022-12-12 | 1 | -0.49 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -28.56 | no |
| Short | 2022-12-17 | 2022-12-18 | 1 | -11.1 | no |
| Long | 2022-12-18 | 2022-12-20 | 2 | -21.4 | no |
| Short | 2022-12-20 | 2022-12-21 | 1 | -27.6 | no |
| Long | 2022-12-21 | 2022-12-31 | 10 | -17.63 | no |
| Short | 2022-12-31 | 2023-01-07 | 7 | -20.42 | no |
| Long | 2023-01-07 | 2023-01-08 | 1 | -2.98 | no |
| Short | 2023-01-08 | 2023-01-09 | 1 | -11.61 | no |
| Long | 2023-01-09 | 2023-03-01 | 51 | 286.82 | no |
| Short | 2023-03-01 | 2023-03-02 | 1 | -26.74 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -61.81 | no |
| Short | 2023-03-04 | 2023-03-13 | 9 | 2.45 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 267.42 | no |
| Short | 2023-04-23 | 2023-04-24 | 1 | 6.63 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 18.62 | no |
| Short | 2023-05-02 | 2023-05-03 | 1 | -32.16 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -48.69 | no |
| Short | 2023-05-09 | 2023-06-21 | 43 | -54.72 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 62.91 | no |
| Short | 2023-07-22 | 2023-09-18 | 58 | 130.02 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 21.4 | no |
| Short | 2023-09-29 | 2023-10-01 | 2 | -1.31 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 872.2 | no |
| Short | 2024-01-08 | 2024-01-09 | 1 | -162.49 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -192.62 | no |
| Short | 2024-01-13 | 2024-01-16 | 3 | 4.45 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -59.27 | no |
| Short | 2024-01-19 | 2024-01-30 | 11 | -100.08 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -34.42 | no |
| Short | 2024-02-01 | 2024-02-03 | 2 | -30.06 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -12.44 | no |
| Short | 2024-02-04 | 2024-02-10 | 6 | -156.64 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 602.94 | no |
| Short | 2024-04-04 | 2024-04-05 | 1 | -87.64 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 57.66 | no |
| Short | 2024-04-11 | 2024-04-19 | 8 | 209.18 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | 5.51 | no |
| Short | 2024-04-20 | 2024-04-22 | 2 | -47.59 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 44.49 | no |
| Short | 2024-04-24 | 2024-05-16 | 22 | -44.84 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -12.53 | no |
| Short | 2024-06-15 | 2024-06-16 | 1 | -13.02 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 8.53 | no |
| Short | 2024-06-17 | 2024-07-20 | 33 | -50.61 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -421.6 | no |
| Short | 2024-08-06 | 2024-08-07 | 1 | -70.42 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -31.94 | no |
| Short | 2024-08-08 | 2024-08-09 | 1 | -203.42 | no |
| Long | 2024-08-09 | 2024-08-14 | 5 | -30.53 | no |
| Short | 2024-08-14 | 2024-09-05 | 22 | 39.44 | no |
| Long | 2024-09-05 | 2024-09-07 | 2 | -106.57 | no |
| Short | 2024-09-07 | 2024-09-14 | 7 | -156.5 | no |
| Long | 2024-09-14 | 2024-09-17 | 3 | -49.33 | no |
| Short | 2024-09-17 | 2024-09-18 | 1 | -45.33 | no |
| Long | 2024-09-18 | 2024-09-23 | 5 | 57.24 | no |
| Short | 2024-09-23 | 2024-09-25 | 2 | -16.49 | no |
| Long | 2024-09-25 | 2024-12-22 | 88 | 591.77 | no |
| Short | 2024-12-22 | 2024-12-25 | 3 | -33.29 | no |
| Long | 2024-12-25 | 2024-12-28 | 3 | -81.25 | no |
| Short | 2024-12-28 | 2025-01-03 | 6 | -52.49 | no |
| Long | 2025-01-03 | 2025-01-04 | 1 | 14.65 | no |
| Short | 2025-01-04 | 2025-01-05 | 1 | -5.48 | no |
| Long | 2025-01-05 | 2025-01-06 | 1 | -2.43 | no |
| Short | 2025-01-06 | 2025-01-07 | 1 | -67.2 | no |
| Long | 2025-01-07 | 2025-01-08 | 1 | -82.98 | no |
| Short | 2025-01-08 | 2025-01-18 | 10 | -110.06 | no |
| Long | 2025-01-18 | 2025-02-03 | 16 | -85.64 | no |
| Short | 2025-02-03 | 2025-02-04 | 1 | -50.43 | no |
| Long | 2025-02-04 | 2025-02-05 | 1 | -45.8 | no |
| Short | 2025-02-05 | 2025-02-08 | 3 | 11.34 | no |
| Long | 2025-02-08 | 2025-02-15 | 7 | 8.28 | no |
| Short | 2025-02-15 | 2025-03-29 | 42 | 120.09 | no |
| Long | 2025-03-29 | 2025-03-30 | 1 | -30.88 | no |
| Short | 2025-03-30 | 2025-04-10 | 11 | -20.65 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -48.63 | no |
| Short | 2025-04-11 | 2025-04-13 | 2 | -85.6 | no |
| Long | 2025-04-13 | 2025-04-14 | 1 | -22.81 | no |
| Short | 2025-04-14 | 2025-04-15 | 1 | -13.93 | no |
| Long | 2025-04-15 | 2025-04-19 | 4 | -4.79 | no |
| Short | 2025-04-19 | 2025-04-20 | 1 | -10.91 | no |
| Long | 2025-04-20 | 2025-06-18 | 59 | 237.2 | no |
| Short | 2025-06-18 | 2025-06-29 | 11 | -39.91 | no |
| Long | 2025-06-29 | 2025-08-16 | 48 | 112.95 | no |
| Short | 2025-08-16 | 2025-09-18 | 33 | -4.02 | no |
| Long | 2025-09-18 | 2025-09-22 | 4 | -18.04 | no |
| Short | 2025-09-22 | 2025-09-25 | 3 | 17.97 | no |
| Long | 2025-09-25 | 2025-09-26 | 1 | -55.63 | no |
| Short | 2025-09-26 | 2025-09-29 | 3 | -41.37 | no |
| Long | 2025-09-29 | 2025-10-11 | 12 | 2.89 | no |
| Short | 2025-10-11 | 2025-10-26 | 15 | 2.3 | no |
| Long | 2025-10-26 | 2025-10-30 | 4 | -22.16 | no |
| Short | 2025-10-30 | 2025-12-21 | 52 | 258.12 | no |
| Long | 2025-12-21 | 2025-12-25 | 4 | -16.27 | no |
| Short | 2025-12-25 | 2026-01-01 | 7 | -4.57 | no |
| Long | 2026-01-01 | 2026-01-02 | 1 | 15.69 | no |
| Short | 2026-01-02 | 2026-01-05 | 3 | -49.13 | no |
| Long | 2026-01-05 | 2026-01-09 | 4 | -11.2 | no |
| Short | 2026-01-09 | 2026-01-12 | 3 | -2.91 | no |
| Long | 2026-01-12 | 2026-01-21 | 9 | -44.91 | no |
| Short | 2026-01-21 | 2026-01-22 | 1 | -20.31 | no |
| Long | 2026-01-22 | 2026-01-26 | 4 | -46.66 | no |
| Short | 2026-01-26 | 2026-01-27 | 1 | -29.5 | no |
| Long | 2026-01-27 | 2026-01-30 | 3 | -57.74 | no |
| Short | 2026-01-30 | 2026-03-08 | 37 | 220.42 | no |
| Long | 2026-03-08 | 2026-03-09 | 1 | -32.08 | no |
| Short | 2026-03-09 | 2026-03-12 | 3 | -92.21 | no |
| Long | 2026-03-12 | 2026-03-23 | 11 | -48.19 | no |
| Short | 2026-03-23 | 2026-03-24 | 1 | -61.16 | no |
| Long | 2026-03-24 | 2026-03-28 | 4 | -80.94 | no |
| Short | 2026-03-28 | 2026-03-30 | 2 | 3.3 | no |
| Long | 2026-03-30 | 2026-03-31 | 1 | 10.08 | no |
| Short | 2026-03-31 | 2026-04-01 | 1 | -29.01 | no |
| Long | 2026-04-01 | 2026-04-02 | 1 | -6.04 | no |
| Short | 2026-04-02 | 2026-04-06 | 4 | -19.11 | no |
| Long | 2026-04-06 | 2026-04-13 | 7 | 24.04 | no |
| Short | 2026-04-13 | 2026-04-14 | 1 | -61.55 | no |
| Long | 2026-04-14 | 2026-04-16 | 2 | 2.6 | no |
| Short | 2026-04-16 | 2026-04-17 | 1 | -7.88 | no |
| Long | 2026-04-17 | 2026-05-23 | 36 | 2.39 | no |
| Short | 2026-05-23 | 2026-07-06 | 44 | 170.32 | no |
| Long | 2026-07-06 | 2026-07-09 | 3 | -29.74 | no |
| Short | 2026-07-09 | 2026-07-10 | 1 | -21.66 | no |
| Long | 2026-07-10 | 2026-07-14 | 4 | -20.15 | no |
| Short | 2026-07-14 | 2026-07-19 | 5 | -50.6 | no |
| Long | 2026-07-19 | 2026-08-12 | 24 | -24.51 | no |
| Short | 2026-08-12 | 2026-08-13 | 1 | -1.44 | no |
| Long | 2026-08-13 | 2026-08-14 | 1 | -3.27 | no |
| Short | 2026-08-14 | 2026-08-19 | 5 | -24.61 | no |
| Long | 2026-08-19 | 2026-10-21 | 64 | 313.76 | yes |

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

