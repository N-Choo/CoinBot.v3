# BTC time-series momentum — Volatility targeting + drawdown control / Flat / train / base

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
| Variant | Volatility targeting + drawdown control |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -9.81% | 9.59% |
| CAGR | -3.39% | 3.10% |
| Max drawdown | 22.40% | 76.63% |
| Sharpe | -0.20 | 0.33 |
| Sortino | -0.31 | 0.48 |
| Win rate | 23.64% | 100.00% |
| Trades | 55 | 1 |
| Average holding days | 5.20 | 1094.00 |
| Turnover (initial equity) | 54.77 | 2.10 |
| Trading costs | 82.15 | 3.15 |
| Borrow costs | 0 | — |
| Average exposure | 12.52% | 100.00% |
| Average absolute exposure | 12.52% | 100.00% |
| Days with exposure | 286 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | 22.28% | 1.09 | 1.92 | 18.04% | 14.97% | 14.97% |
| Bear | 379 | -16.76% | -2.23 | -2.66 | 16.96% | 6.60% | 6.60% |
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
| Long | 2022-07-29 | 2022-08-03 | 5 | -9.62 | no |
| Long | 2023-12-05 | 2023-12-12 | 7 | 0.95 | no |
| Long | 2023-12-19 | 2023-12-28 | 9 | 2.98 | no |
| Long | 2024-01-02 | 2024-01-04 | 2 | -14.69 | no |
| Long | 2024-01-05 | 2024-01-13 | 8 | -11.09 | no |
| Long | 2024-01-27 | 2024-02-06 | 10 | -0.39 | no |
| Long | 2024-02-07 | 2024-02-23 | 16 | 114.84 | no |
| Long | 2024-02-27 | 2024-03-17 | 19 | 49.23 | no |
| Long | 2024-03-26 | 2024-04-02 | 7 | -5.91 | no |
| Long | 2024-04-09 | 2024-04-13 | 4 | -26.34 | no |
| Long | 2024-04-21 | 2024-04-22 | 1 | -1.02 | no |
| Long | 2024-04-23 | 2024-04-27 | 4 | -21.78 | no |
| Long | 2024-05-05 | 2024-05-07 | 2 | -3.83 | no |
| Long | 2024-05-08 | 2024-05-11 | 3 | -8.16 | no |
| Long | 2024-05-16 | 2024-05-28 | 12 | 15.56 | no |
| Long | 2024-05-31 | 2024-06-01 | 1 | -12.03 | no |
| Long | 2024-06-05 | 2024-06-12 | 7 | -38.17 | no |
| Long | 2024-07-02 | 2024-07-04 | 2 | -24.19 | no |
| Long | 2024-07-12 | 2024-07-30 | 18 | 74.62 | no |
| Long | 2024-07-31 | 2024-08-01 | 1 | -16.09 | no |
| Long | 2024-08-11 | 2024-08-16 | 5 | -10.9 | no |
| Long | 2024-08-20 | 2024-08-21 | 1 | -4.36 | no |
| Long | 2024-08-22 | 2024-08-29 | 7 | -5.82 | no |
| Long | 2024-09-11 | 2024-09-12 | 1 | -2.94 | no |
| Long | 2024-09-13 | 2024-10-01 | 18 | 35.24 | no |
| Long | 2024-10-09 | 2024-10-10 | 1 | -24.52 | no |
| Long | 2024-10-12 | 2024-10-21 | 10 | 33.65 | yes |

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

