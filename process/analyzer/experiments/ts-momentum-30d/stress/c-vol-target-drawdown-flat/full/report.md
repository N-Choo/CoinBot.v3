# BTC time-series momentum — Volatility targeting + drawdown control / Flat / full / stress

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
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 14.02% | 36.38% |
| CAGR | 2.66% | 6.41% |
| Max drawdown | 22.00% | 76.63% |
| Sharpe | 0.25 | 0.38 |
| Sortino | 0.39 | 0.55 |
| Win rate | 31.34% | 100.00% |
| Trades | 67 | 1 |
| Average holding days | 11.64 | 1824.00 |
| Turnover (initial equity) | 79.44 | 2.37 |
| Trading costs | 238.32 | 7.1 |
| Borrow costs | 0 | — |
| Average exposure | 20.30% | 100.00% |
| Average absolute exposure | 20.30% | 100.00% |
| Days with exposure | 780 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 86.58% | 1.50 | 2.54 | 13.64% | 29.04% | 29.04% |
| Bear | 707 | -31.57% | -1.69 | -2.12 | 34.48% | 12.48% | 12.48% |
| Unknown | 199 | -10.70% | -2.33 | -2.64 | 11.88% | 7.80% | 7.80% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2022-02-07 | 2022-02-12 | 5 | -1.63 | no |
| Long | 2022-02-16 | 2022-02-18 | 2 | -29.44 | no |
| Long | 2022-02-21 | 2022-02-28 | 7 | -6.7 | no |
| Long | 2022-03-01 | 2022-03-07 | 6 | -27.81 | no |
| Long | 2022-03-20 | 2022-04-16 | 27 | -29.74 | no |
| Long | 2022-04-20 | 2022-04-22 | 2 | -11.7 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -2.5 | no |
| Long | 2022-07-17 | 2022-07-26 | 9 | -2.85 | no |
| Long | 2022-07-27 | 2022-08-19 | 23 | 25.42 | no |
| Long | 2022-08-25 | 2022-08-27 | 2 | -19.25 | no |
| Long | 2022-10-05 | 2022-10-10 | 5 | -14.05 | no |
| Long | 2022-10-21 | 2022-10-23 | 2 | 1.24 | no |
| Long | 2022-10-24 | 2022-11-04 | 11 | 18.24 | no |
| Long | 2022-11-05 | 2022-11-09 | 4 | -74.25 | no |
| Long | 2022-12-10 | 2022-12-11 | 1 | -2.02 | no |
| Long | 2022-12-12 | 2022-12-17 | 5 | -16.41 | no |
| Long | 2023-01-21 | 2023-03-01 | 39 | 12.3 | no |
| Long | 2023-03-02 | 2023-03-04 | 2 | -20.09 | no |
| Long | 2023-03-13 | 2023-04-23 | 41 | 49.46 | no |
| Long | 2023-04-24 | 2023-05-02 | 8 | 4.48 | no |
| Long | 2023-05-03 | 2023-05-09 | 6 | -15.81 | no |
| Long | 2023-06-21 | 2023-07-22 | 31 | 16.72 | no |
| Long | 2023-09-18 | 2023-09-29 | 11 | 6.35 | no |
| Long | 2023-10-01 | 2024-01-08 | 99 | 277.99 | no |
| Long | 2024-01-09 | 2024-01-13 | 4 | -48.18 | no |
| Long | 2024-01-16 | 2024-01-19 | 3 | -14.51 | no |
| Long | 2024-01-30 | 2024-02-01 | 2 | -8.84 | no |
| Long | 2024-02-03 | 2024-02-04 | 1 | -4.4 | no |
| Long | 2024-02-10 | 2024-04-04 | 54 | 219.83 | no |
| Long | 2024-04-05 | 2024-04-11 | 6 | 9.64 | no |
| Long | 2024-04-19 | 2024-04-20 | 1 | -0.24 | no |
| Long | 2024-04-22 | 2024-04-24 | 2 | 8.15 | no |
| Long | 2024-05-16 | 2024-06-15 | 30 | -4.3 | no |
| Long | 2024-06-16 | 2024-06-17 | 1 | 0.56 | no |
| Long | 2024-07-20 | 2024-08-06 | 17 | -110.89 | no |
| Long | 2024-08-07 | 2024-08-08 | 1 | -9.56 | no |
| Long | 2024-08-09 | 2024-08-14 | 5 | -8.42 | no |
| Long | 2024-09-05 | 2024-09-07 | 2 | -29.28 | no |
| Long | 2024-09-14 | 2024-09-17 | 3 | -20.89 | no |
| Long | 2024-09-18 | 2024-09-23 | 5 | 21.63 | no |
| Long | 2024-09-25 | 2024-12-22 | 88 | 238.75 | no |
| Long | 2024-12-25 | 2024-12-28 | 3 | -31.57 | no |
| Long | 2025-01-03 | 2025-01-04 | 1 | 3.77 | no |
| Long | 2025-01-05 | 2025-01-06 | 1 | -2.82 | no |
| Long | 2025-01-07 | 2025-01-08 | 1 | -35 | no |
| Long | 2025-01-18 | 2025-02-03 | 16 | -42.35 | no |
| Long | 2025-02-04 | 2025-02-05 | 1 | -23.12 | no |
| Long | 2025-02-08 | 2025-02-15 | 7 | 2.74 | no |
| Long | 2025-03-29 | 2025-03-30 | 1 | -10.13 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -18.7 | no |
| Long | 2025-04-13 | 2025-04-14 | 1 | -10.29 | no |
| Long | 2025-04-15 | 2025-04-19 | 4 | -3.24 | no |
| Long | 2025-04-20 | 2025-06-18 | 59 | 96.57 | no |
| Long | 2025-06-29 | 2025-08-16 | 48 | 80.89 | no |
| Long | 2025-09-18 | 2025-09-22 | 4 | -16.3 | no |
| Long | 2025-09-25 | 2025-09-26 | 1 | -52.61 | no |
| Long | 2025-09-29 | 2025-10-11 | 12 | 13.19 | no |
| Long | 2025-10-26 | 2025-10-30 | 4 | -12.29 | no |
| Long | 2025-12-21 | 2025-12-25 | 4 | -9.13 | no |
| Long | 2026-01-01 | 2026-01-02 | 1 | 5.88 | no |
| Long | 2026-01-05 | 2026-01-09 | 4 | -9.82 | no |
| Long | 2026-01-12 | 2026-01-21 | 9 | -35.3 | no |
| Long | 2026-01-22 | 2026-01-26 | 4 | -31.4 | no |
| Long | 2026-01-27 | 2026-01-30 | 3 | -36.29 | no |
| Long | 2026-03-08 | 2026-03-09 | 1 | -9.51 | no |
| Long | 2026-03-12 | 2026-03-23 | 11 | -18.06 | no |
| Long | 2026-03-24 | 2026-03-28 | 4 | -31.88 | no |

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

