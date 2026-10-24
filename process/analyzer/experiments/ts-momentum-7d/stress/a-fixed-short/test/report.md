# BTC time-series momentum — Fixed exposure / Short / test / stress

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
| Split | test |
| Candles | 365 |
| Range | 2025-10-22 .. 2026-10-21 |
| Variant | Fixed exposure |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -41.69% | -22.82% |
| CAGR | -41.69% | -22.82% |
| Max drawdown | 49.52% | 48.82% |
| Sharpe | -1.00 | -0.37 |
| Sortino | -1.46 | -0.53 |
| Win rate | 24.05% | 0.00% |
| Trades | 79 | 1 |
| Average holding days | 4.62 | 365.00 |
| Turnover (initial equity) | 120.11 | 1.77 |
| Trading costs | 360.33 | 5.32 |
| Borrow costs | 0 | — |
| Average exposure | -5.75% | 100.00% |
| Average absolute exposure | 100.00% | 100.00% |
| Days with exposure | 365 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 74 | -12.53% | -1.35 | -2.14 | 25.37% | 13.51% | 100.00% |
| Bear | 291 | -33.34% | -0.91 | -1.31 | 45.45% | -10.65% | 100.00% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Short | 2025-10-22 | 2025-10-24 | 2 | -22.42 | no |
| Long | 2025-10-24 | 2025-10-31 | 7 | -21.35 | no |
| Short | 2025-10-31 | 2025-11-12 | 12 | 35.9 | no |
| Long | 2025-11-12 | 2025-11-13 | 1 | -19.34 | no |
| Short | 2025-11-13 | 2025-11-28 | 15 | 93.1 | no |
| Long | 2025-11-28 | 2025-12-02 | 4 | -64.95 | no |
| Short | 2025-12-02 | 2025-12-03 | 1 | -63.52 | no |
| Long | 2025-12-03 | 2025-12-06 | 3 | -25.46 | no |
| Short | 2025-12-06 | 2025-12-08 | 2 | -16.26 | no |
| Long | 2025-12-08 | 2025-12-11 | 3 | 10.57 | no |
| Short | 2025-12-11 | 2025-12-12 | 1 | -10.32 | no |
| Long | 2025-12-12 | 2025-12-15 | 3 | -47.17 | no |
| Short | 2025-12-15 | 2025-12-22 | 7 | -11.36 | no |
| Long | 2025-12-22 | 2025-12-24 | 2 | -15.91 | no |
| Short | 2025-12-24 | 2025-12-25 | 1 | -6.61 | no |
| Long | 2025-12-25 | 2025-12-27 | 2 | -7.67 | no |
| Short | 2025-12-27 | 2025-12-31 | 4 | -15.32 | no |
| Long | 2025-12-31 | 2026-01-01 | 1 | -12.39 | no |
| Short | 2026-01-01 | 2026-01-02 | 1 | -15.23 | no |
| Long | 2026-01-02 | 2026-01-11 | 9 | 9.68 | no |
| Short | 2026-01-11 | 2026-01-14 | 3 | -45.77 | no |
| Long | 2026-01-14 | 2026-01-21 | 7 | -57.37 | no |
| Short | 2026-01-21 | 2026-01-28 | 7 | -11.13 | no |
| Long | 2026-01-28 | 2026-01-29 | 1 | -3.59 | no |
| Short | 2026-01-29 | 2026-02-13 | 15 | 182.18 | no |
| Long | 2026-02-13 | 2026-02-14 | 1 | 27.44 | no |
| Short | 2026-02-14 | 2026-02-15 | 1 | -17.3 | no |
| Long | 2026-02-15 | 2026-02-16 | 1 | -17.06 | no |
| Short | 2026-02-16 | 2026-02-20 | 4 | 16.98 | no |
| Long | 2026-02-20 | 2026-02-21 | 1 | 7.72 | no |
| Short | 2026-02-21 | 2026-02-26 | 5 | -9.66 | no |
| Long | 2026-02-26 | 2026-02-28 | 2 | -31.23 | no |
| Short | 2026-02-28 | 2026-03-03 | 3 | -41.81 | no |
| Long | 2026-03-03 | 2026-03-10 | 7 | -8.99 | no |
| Short | 2026-03-10 | 2026-03-11 | 1 | -21.39 | no |
| Long | 2026-03-11 | 2026-03-12 | 1 | -1.82 | no |
| Short | 2026-03-12 | 2026-03-14 | 2 | -12.08 | no |
| Long | 2026-03-14 | 2026-03-20 | 6 | -14.65 | no |
| Short | 2026-03-20 | 2026-03-26 | 6 | -20.31 | no |
| Long | 2026-03-26 | 2026-03-27 | 1 | -28.43 | no |
| Short | 2026-03-27 | 2026-04-04 | 8 | 12.84 | no |
| Long | 2026-04-04 | 2026-04-30 | 26 | 84.75 | no |
| Short | 2026-04-30 | 2026-05-02 | 2 | -28.8 | no |
| Long | 2026-05-02 | 2026-05-04 | 2 | -1.24 | no |
| Short | 2026-05-04 | 2026-05-05 | 1 | -16.38 | no |
| Long | 2026-05-05 | 2026-05-13 | 8 | 1.4 | no |
| Short | 2026-05-13 | 2026-05-15 | 2 | -10.09 | no |
| Long | 2026-05-15 | 2026-05-16 | 1 | -21.53 | no |
| Short | 2026-05-16 | 2026-05-26 | 10 | 10.68 | no |
| Long | 2026-05-26 | 2026-05-27 | 1 | -16.67 | no |
| Short | 2026-05-27 | 2026-06-13 | 17 | 117.3 | no |
| Long | 2026-06-13 | 2026-06-19 | 6 | -12.55 | no |
| Short | 2026-06-19 | 2026-07-03 | 14 | 9.62 | no |
| Long | 2026-07-03 | 2026-07-14 | 11 | 5.25 | no |
| Short | 2026-07-14 | 2026-07-15 | 1 | -39.26 | no |
| Long | 2026-07-15 | 2026-07-18 | 3 | -17.45 | no |
| Short | 2026-07-18 | 2026-07-19 | 1 | -14.94 | no |
| Long | 2026-07-19 | 2026-07-26 | 7 | -9.61 | no |
| Short | 2026-07-26 | 2026-07-27 | 1 | -15.8 | no |
| Long | 2026-07-27 | 2026-07-28 | 1 | -21.73 | no |
| Short | 2026-07-28 | 2026-08-05 | 8 | -8.49 | no |
| Long | 2026-08-05 | 2026-08-07 | 2 | -1.69 | no |
| Short | 2026-08-07 | 2026-08-08 | 1 | -10.18 | no |
| Long | 2026-08-08 | 2026-08-12 | 4 | -17.37 | no |
| Short | 2026-08-12 | 2026-08-18 | 6 | -13.62 | no |
| Long | 2026-08-18 | 2026-08-29 | 11 | 125.35 | no |
| Short | 2026-08-29 | 2026-08-30 | 1 | -8.46 | no |
| Long | 2026-08-30 | 2026-08-31 | 1 | -9.77 | no |
| Short | 2026-08-31 | 2026-09-04 | 4 | -38.83 | no |
| Long | 2026-09-04 | 2026-09-11 | 7 | -44.41 | no |
| Short | 2026-09-11 | 2026-09-19 | 8 | -41.71 | no |
| Long | 2026-09-19 | 2026-09-29 | 10 | 16 | no |
| Short | 2026-09-29 | 2026-10-02 | 3 | -14.15 | no |
| Long | 2026-10-02 | 2026-10-08 | 6 | -14.99 | no |
| Short | 2026-10-08 | 2026-10-16 | 8 | 3.19 | no |
| Long | 2026-10-16 | 2026-10-17 | 1 | -8.24 | no |
| Short | 2026-10-17 | 2026-10-20 | 3 | -6 | no |
| Long | 2026-10-20 | 2026-10-21 | 1 | 4.3 | no |
| Short | 2026-10-21 | 2026-10-21 | 1 | -9.35 | yes |

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

