# BTC time-series momentum — Volatility targeting + drawdown control / Short / full / stress

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
| Variant | Volatility targeting + drawdown control |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 2.64% | 36.38% |
| CAGR | 0.52% | 6.41% |
| Max drawdown | 34.67% | 76.63% |
| Sharpe | 0.11 | 0.38 |
| Sortino | 0.17 | 0.55 |
| Win rate | 31.88% | 100.00% |
| Trades | 69 | 1 |
| Average holding days | 12.19 | 1824.00 |
| Turnover (initial equity) | 66.22 | 2.37 |
| Trading costs | 198.65 | 7.1 |
| Borrow costs | 0 | — |
| Average exposure | 0.14% | 100.00% |
| Average absolute exposure | 19.91% | 100.00% |
| Days with exposure | 841 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 23.58% | 0.60 | 0.94 | 16.95% | 4.58% | 23.46% |
| Bear | 707 | -11.79% | -0.37 | -0.54 | 16.74% | -2.07% | 15.65% |
| Unknown | 199 | -5.85% | -0.55 | -0.77 | 20.53% | -12.51% | 18.65% |

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
| Long | 2022-03-20 | 2022-03-21 | 1 | -6.73 | no |
| Short | 2022-06-17 | 2022-07-17 | 30 | -17.65 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | -2.96 | no |
| Short | 2022-07-26 | 2022-07-27 | 1 | -1.11 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 26.34 | no |
| Short | 2022-08-19 | 2022-08-25 | 6 | 31.15 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -20.61 | no |
| Short | 2022-08-27 | 2022-10-05 | 39 | -16.61 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -14.78 | no |
| Short | 2022-10-10 | 2022-10-21 | 11 | 6.56 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 1.31 | no |
| Short | 2022-10-23 | 2022-10-24 | 1 | -17.72 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 18.95 | no |
| Short | 2022-11-04 | 2022-11-05 | 1 | -35.41 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -74.23 | no |
| Short | 2022-11-09 | 2022-12-10 | 31 | 24.82 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -2.08 | no |
| Short | 2022-12-11 | 2022-12-12 | 1 | -1.76 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -16.87 | no |
| Short | 2022-12-17 | 2022-12-18 | 1 | -6.96 | no |
| Long | 2023-02-16 | 2023-03-01 | 13 | -17.81 | no |
| Short | 2023-03-01 | 2023-03-02 | 1 | -8.62 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -19.52 | no |
| Short | 2023-03-04 | 2023-03-13 | 9 | 1.44 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 48.14 | no |
| Short | 2023-04-23 | 2023-04-24 | 1 | 0.84 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 4.36 | no |
| Short | 2023-05-02 | 2023-05-03 | 1 | -10.95 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -15.2 | no |
| Short | 2023-05-09 | 2023-06-21 | 43 | -18.17 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 15.71 | no |
| Short | 2023-07-22 | 2023-09-18 | 58 | 76.84 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 6.53 | no |
| Short | 2023-09-29 | 2023-10-01 | 2 | -2.5 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 285.44 | no |
| Short | 2024-01-08 | 2024-01-09 | 1 | -40.05 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -47.78 | no |
| Short | 2024-01-13 | 2024-01-16 | 3 | -0.14 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -14.39 | no |
| Short | 2024-01-19 | 2024-01-30 | 11 | -21.73 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -8.59 | no |
| Short | 2024-02-01 | 2024-02-03 | 2 | -8.02 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -4.24 | no |
| Short | 2024-02-04 | 2024-02-10 | 6 | -43.54 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 202.84 | no |
| Short | 2024-04-04 | 2024-04-05 | 1 | -14.3 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 8.79 | no |
| Short | 2024-04-11 | 2024-04-19 | 8 | 32.37 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | -0.23 | no |
| Short | 2024-04-20 | 2024-04-22 | 2 | -10.4 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 7.57 | no |
| Short | 2024-04-24 | 2024-05-16 | 22 | -11.27 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -3.96 | no |
| Short | 2024-06-15 | 2024-06-16 | 1 | -5.52 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 0.51 | no |
| Short | 2024-06-17 | 2024-07-20 | 33 | 19.12 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -103.18 | no |
| Short | 2024-08-06 | 2024-08-07 | 1 | -17.39 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -8.75 | no |
| Short | 2024-08-08 | 2024-08-09 | 1 | -49.77 | no |

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

