# BTC time-series momentum — Volatility targeting + drawdown control / Short / train / base

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
| Split | train |
| Candles | 1094 |
| Range | 2021-10-24 .. 2024-10-21 |
| Variant | Volatility targeting + drawdown control |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 14.79% | 9.59% |
| CAGR | 4.71% | 3.10% |
| Max drawdown | 31.22% | 76.63% |
| Sharpe | 0.33 | 0.33 |
| Sortino | 0.49 | 0.48 |
| Win rate | 36.11% | 100.00% |
| Trades | 72 | 1 |
| Average holding days | 12.50 | 1094.00 |
| Turnover (initial equity) | 74.17 | 2.10 |
| Trading costs | 111.26 | 3.15 |
| Borrow costs | 0 | — |
| Average exposure | 2.29% | 100.00% |
| Average absolute exposure | 35.48% | 100.00% |
| Days with exposure | 900 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | 34.91% | 1.07 | 1.70 | 16.02% | 10.50% | 44.08% |
| Bear | 379 | -10.11% | -0.41 | -0.60 | 18.39% | -2.62% | 31.11% |
| Unknown | 199 | -5.35% | -0.48 | -0.66 | 20.23% | -9.66% | 21.51% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Short | 2021-11-24 | 2022-02-07 | 75 | 102.91 | no |
| Long | 2022-02-07 | 2022-02-12 | 5 | -0.77 | no |
| Short | 2022-02-12 | 2022-02-16 | 4 | -18.7 | no |
| Long | 2022-02-16 | 2022-02-18 | 2 | -31 | no |
| Short | 2022-02-18 | 2022-02-21 | 3 | 14.73 | no |
| Long | 2022-02-21 | 2022-02-28 | 7 | -6.34 | no |
| Short | 2022-02-28 | 2022-03-01 | 1 | -48.66 | no |
| Long | 2022-03-01 | 2022-03-07 | 6 | -28.51 | no |
| Short | 2022-03-07 | 2022-03-20 | 13 | -26.64 | no |
| Long | 2022-03-20 | 2022-04-09 | 20 | -10.56 | no |
| Short | 2022-06-14 | 2022-07-17 | 33 | 3.34 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | -1.96 | no |
| Short | 2022-07-26 | 2022-07-27 | 1 | -0.14 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 28.5 | no |
| Short | 2022-08-19 | 2022-08-25 | 6 | 33.48 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -20.21 | no |
| Short | 2022-08-27 | 2022-10-05 | 39 | -15.8 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -14.4 | no |
| Short | 2022-10-10 | 2022-10-21 | 11 | 8.5 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 3.23 | no |
| Short | 2022-10-23 | 2022-10-24 | 1 | -16.25 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 22.08 | no |
| Short | 2022-11-04 | 2022-11-05 | 1 | -34.96 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -76.09 | no |
| Short | 2022-11-09 | 2022-12-10 | 31 | 27.29 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -1.08 | no |
| Short | 2022-12-11 | 2022-12-12 | 1 | -0.26 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -16.11 | no |
| Short | 2022-12-17 | 2022-12-18 | 1 | -5.87 | no |
| Long | 2022-12-18 | 2022-12-20 | 2 | -11.35 | no |
| Short | 2022-12-20 | 2022-12-21 | 1 | -14.55 | no |
| Long | 2022-12-21 | 2022-12-29 | 8 | -12.01 | no |
| Long | 2023-01-21 | 2023-03-01 | 39 | 14.33 | no |
| Short | 2023-03-01 | 2023-03-02 | 1 | -8.07 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -19.56 | no |
| Short | 2023-03-04 | 2023-03-13 | 9 | 2.53 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 52.44 | no |
| Short | 2023-04-23 | 2023-04-24 | 1 | 2.15 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 6.06 | no |
| Short | 2023-05-02 | 2023-05-03 | 1 | -10.36 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -14.92 | no |
| Short | 2023-05-09 | 2023-06-21 | 43 | -17.26 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 18.55 | no |
| Short | 2023-07-22 | 2023-09-18 | 58 | 85.77 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 8.76 | no |
| Short | 2023-09-29 | 2023-10-01 | 2 | -0.67 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 312.86 | no |
| Short | 2024-01-08 | 2024-01-09 | 1 | -41.65 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -50.18 | no |
| Short | 2024-01-13 | 2024-01-16 | 3 | 1.2 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -14.26 | no |
| Short | 2024-01-19 | 2024-01-30 | 11 | -22.26 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -8.12 | no |
| Short | 2024-02-01 | 2024-02-03 | 2 | -7.46 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -3.29 | no |
| Short | 2024-02-04 | 2024-02-10 | 6 | -46.12 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 225.12 | no |
| Short | 2024-04-04 | 2024-04-05 | 1 | -14.63 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 10.77 | no |
| Short | 2024-04-11 | 2024-04-19 | 8 | 36.78 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | 1 | no |
| Short | 2024-04-20 | 2024-04-22 | 2 | -9.99 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 9.84 | no |
| Short | 2024-04-24 | 2024-05-16 | 22 | -10.54 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -2.32 | no |
| Short | 2024-06-15 | 2024-06-16 | 1 | -4.03 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 2.67 | no |
| Short | 2024-06-17 | 2024-07-20 | 33 | 24.71 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -113.03 | no |
| Short | 2024-08-06 | 2024-08-07 | 1 | -18.04 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -8.43 | no |
| Short | 2024-08-08 | 2024-08-09 | 1 | -54.32 | no |

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

