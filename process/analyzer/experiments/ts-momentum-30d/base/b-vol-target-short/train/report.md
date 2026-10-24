# BTC time-series momentum — Volatility targeting / Short / train / base

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
| Variant | Volatility targeting |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 42.68% | 9.59% |
| CAGR | 12.59% | 3.10% |
| Max drawdown | 31.11% | 76.63% |
| Sharpe | 0.64 | 0.33 |
| Sortino | 0.99 | 0.48 |
| Win rate | 34.09% | 100.00% |
| Trades | 88 | 1 |
| Average holding days | 12.08 | 1094.00 |
| Turnover (initial equity) | 110.48 | 2.10 |
| Trading costs | 165.72 | 3.15 |
| Borrow costs | 0 | — |
| Average exposure | 2.15% | 100.00% |
| Average absolute exposure | 41.73% | 100.00% |
| Days with exposure | 1063 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | 55.71% | 1.48 | 2.41 | 16.02% | 12.39% | 46.50% |
| Bear | 379 | -7.65% | -0.22 | -0.34 | 22.79% | -4.12% | 42.40% |
| Unknown | 199 | -0.78% | 0.03 | 0.05 | 23.65% | -12.47% | 28.07% |

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
| Long | 2022-03-20 | 2022-04-16 | 27 | -29.09 | no |
| Short | 2022-04-16 | 2022-04-20 | 4 | -10.6 | no |
| Long | 2022-04-20 | 2022-04-22 | 2 | -10.71 | no |
| Short | 2022-04-22 | 2022-06-09 | 48 | 85.07 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -1.88 | no |
| Short | 2022-06-10 | 2022-07-17 | 37 | 89.58 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | -2.22 | no |
| Short | 2022-07-26 | 2022-07-27 | 1 | -0.16 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 32.39 | no |
| Short | 2022-08-19 | 2022-08-25 | 6 | 38.05 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -22.97 | no |
| Short | 2022-08-27 | 2022-10-05 | 39 | -17.95 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -16.36 | no |
| Short | 2022-10-10 | 2022-10-21 | 11 | 9.66 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 3.67 | no |
| Short | 2022-10-23 | 2022-10-24 | 1 | -18.46 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 25.09 | no |
| Short | 2022-11-04 | 2022-11-05 | 1 | -39.73 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -86.47 | no |
| Short | 2022-11-09 | 2022-12-10 | 31 | 31.01 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -1.23 | no |
| Short | 2022-12-11 | 2022-12-12 | 1 | -0.3 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -18.3 | no |
| Short | 2022-12-17 | 2022-12-18 | 1 | -6.67 | no |
| Long | 2022-12-18 | 2022-12-20 | 2 | -12.9 | no |
| Short | 2022-12-20 | 2022-12-21 | 1 | -16.54 | no |
| Long | 2022-12-21 | 2022-12-31 | 10 | -11.7 | no |
| Short | 2022-12-31 | 2023-01-07 | 7 | -16.56 | no |
| Long | 2023-01-07 | 2023-01-08 | 1 | -2.47 | no |
| Short | 2023-01-08 | 2023-01-09 | 1 | -10.09 | no |
| Long | 2023-01-09 | 2023-03-01 | 51 | 204.15 | no |
| Short | 2023-03-01 | 2023-03-02 | 1 | -10.7 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -25.94 | no |
| Short | 2023-03-04 | 2023-03-13 | 9 | 3.36 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 69.56 | no |
| Short | 2023-04-23 | 2023-04-24 | 1 | 2.85 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 8.03 | no |
| Short | 2023-05-02 | 2023-05-03 | 1 | -13.74 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -19.8 | no |
| Short | 2023-05-09 | 2023-06-21 | 43 | -22.9 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 24.6 | no |
| Short | 2023-07-22 | 2023-09-18 | 58 | 113.76 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 11.62 | no |
| Short | 2023-09-29 | 2023-10-01 | 2 | -0.88 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 414.97 | no |
| Short | 2024-01-08 | 2024-01-09 | 1 | -55.25 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -66.55 | no |
| Short | 2024-01-13 | 2024-01-16 | 3 | 1.6 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -18.92 | no |
| Short | 2024-01-19 | 2024-01-30 | 11 | -29.53 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -10.77 | no |
| Short | 2024-02-01 | 2024-02-03 | 2 | -9.9 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -4.36 | no |
| Short | 2024-02-04 | 2024-02-10 | 6 | -61.17 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 298.59 | no |
| Short | 2024-04-04 | 2024-04-05 | 1 | -19.4 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 14.29 | no |
| Short | 2024-04-11 | 2024-04-19 | 8 | 48.79 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | 1.33 | no |
| Short | 2024-04-20 | 2024-04-22 | 2 | -13.24 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 13.05 | no |
| Short | 2024-04-24 | 2024-05-16 | 22 | -13.97 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -3.08 | no |
| Short | 2024-06-15 | 2024-06-16 | 1 | -5.34 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 3.55 | no |
| Short | 2024-06-17 | 2024-07-20 | 33 | 32.78 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -149.92 | no |
| Short | 2024-08-06 | 2024-08-07 | 1 | -23.92 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -11.18 | no |
| Short | 2024-08-08 | 2024-08-09 | 1 | -72.05 | no |
| Long | 2024-08-09 | 2024-08-14 | 5 | -9.53 | no |
| Short | 2024-08-14 | 2024-09-05 | 22 | 11.47 | no |
| Long | 2024-09-05 | 2024-09-07 | 2 | -36.78 | no |
| Short | 2024-09-07 | 2024-09-14 | 7 | -72.78 | no |
| Long | 2024-09-14 | 2024-09-17 | 3 | -24.23 | no |
| Short | 2024-09-17 | 2024-09-18 | 1 | -22.71 | no |
| Long | 2024-09-18 | 2024-09-23 | 5 | 28.27 | no |
| Short | 2024-09-23 | 2024-09-25 | 2 | -9.04 | no |
| Long | 2024-09-25 | 2024-10-21 | 27 | 39.61 | yes |

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

