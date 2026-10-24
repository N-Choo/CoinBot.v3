# BTC time-series momentum — Fixed exposure / Short / validation / base

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
| Split | validation |
| Candles | 365 |
| Range | 2024-10-22 .. 2025-10-21 |
| Variant | Fixed exposure |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -54.89% | 60.29% |
| CAGR | -54.89% | 60.29% |
| Max drawdown | 60.65% | 28.11% |
| Sharpe | -1.56 | 1.29 |
| Sortino | -2.05 | 2.01 |
| Win rate | 25.00% | 100.00% |
| Trades | 64 | 1 |
| Average holding days | 5.70 | 365.00 |
| Turnover (initial equity) | 78.37 | 2.61 |
| Trading costs | 117.56 | 3.91 |
| Borrow costs | 0 | — |
| Average exposure | 14.52% | 100.00% |
| Average absolute exposure | 100.00% | 100.00% |
| Days with exposure | 365 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 328 | -34.20% | -0.87 | -1.19 | 40.52% | 16.46% | 100.00% |
| Bear | 37 | -31.45% | -6.52 | -6.85 | 34.52% | -2.70% | 100.00% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2024-10-22 | 2024-10-24 | 2 | -13.51 | no |
| Short | 2024-10-24 | 2024-10-25 | 1 | -25.61 | no |
| Long | 2024-10-25 | 2024-10-26 | 1 | -24.01 | no |
| Short | 2024-10-26 | 2024-10-29 | 3 | -47.33 | no |
| Long | 2024-10-29 | 2024-11-05 | 7 | -29.35 | no |
| Short | 2024-11-05 | 2024-11-07 | 2 | -96.78 | no |
| Long | 2024-11-07 | 2024-11-27 | 20 | 162.63 | no |
| Short | 2024-11-27 | 2024-11-28 | 1 | -41.89 | no |
| Long | 2024-11-28 | 2024-11-29 | 1 | -4.6 | no |
| Short | 2024-11-29 | 2024-12-03 | 4 | -4.98 | no |
| Long | 2024-12-03 | 2024-12-19 | 16 | 37.2 | no |
| Short | 2024-12-19 | 2025-01-03 | 15 | 21.98 | no |
| Long | 2025-01-03 | 2025-01-10 | 7 | -45.38 | no |
| Short | 2025-01-10 | 2025-01-16 | 6 | -74.89 | no |
| Long | 2025-01-16 | 2025-01-28 | 12 | 10.28 | no |
| Short | 2025-01-28 | 2025-01-30 | 2 | -16.19 | no |
| Long | 2025-01-30 | 2025-02-01 | 2 | -12.54 | no |
| Short | 2025-02-01 | 2025-02-13 | 12 | 30.7 | no |
| Long | 2025-02-13 | 2025-02-17 | 4 | -17.1 | no |
| Short | 2025-02-17 | 2025-02-21 | 4 | -20.61 | no |
| Long | 2025-02-21 | 2025-02-22 | 1 | -19.32 | no |
| Short | 2025-02-22 | 2025-02-24 | 2 | -2.95 | no |
| Long | 2025-02-24 | 2025-02-25 | 1 | -39.65 | no |
| Short | 2025-02-25 | 2025-03-06 | 9 | -10.61 | no |
| Long | 2025-03-06 | 2025-03-10 | 4 | -79.93 | no |
| Short | 2025-03-10 | 2025-03-17 | 7 | -20.1 | no |
| Long | 2025-03-17 | 2025-03-19 | 2 | -0.89 | no |
| Short | 2025-03-19 | 2025-03-20 | 1 | -32.46 | no |
| Long | 2025-03-20 | 2025-03-23 | 3 | -21.79 | no |
| Short | 2025-03-23 | 2025-03-24 | 1 | -16.62 | no |
| Long | 2025-03-24 | 2025-03-30 | 6 | -23.29 | no |
| Short | 2025-03-30 | 2025-04-06 | 7 | -8.13 | no |
| Long | 2025-04-06 | 2025-04-07 | 1 | -32.61 | no |
| Short | 2025-04-07 | 2025-04-10 | 3 | -29.19 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -17.67 | no |
| Short | 2025-04-11 | 2025-04-13 | 2 | -31.1 | no |
| Long | 2025-04-13 | 2025-04-20 | 7 | -2.1 | no |
| Short | 2025-04-20 | 2025-04-21 | 1 | -1.71 | no |
| Long | 2025-04-21 | 2025-05-06 | 15 | 43.26 | no |
| Short | 2025-05-06 | 2025-05-07 | 1 | -11.09 | no |
| Long | 2025-05-07 | 2025-05-18 | 11 | 26.55 | no |
| Short | 2025-05-18 | 2025-05-19 | 1 | -16.01 | no |
| Long | 2025-05-19 | 2025-05-29 | 10 | 4.18 | no |
| Short | 2025-05-29 | 2025-06-07 | 9 | 12.28 | no |
| Long | 2025-06-07 | 2025-06-15 | 8 | 3.48 | no |
| Short | 2025-06-15 | 2025-06-25 | 10 | -5.66 | no |
| Long | 2025-06-25 | 2025-07-02 | 7 | -3.1 | no |
| Short | 2025-07-02 | 2025-07-03 | 1 | -14.77 | no |
| Long | 2025-07-03 | 2025-07-21 | 18 | 32.44 | no |
| Short | 2025-07-21 | 2025-07-23 | 2 | -12.12 | no |
| Long | 2025-07-23 | 2025-07-25 | 2 | -7.54 | no |
| Short | 2025-07-25 | 2025-07-27 | 2 | 0.2 | no |
| Long | 2025-07-27 | 2025-07-30 | 3 | -1.28 | no |
| Short | 2025-07-30 | 2025-08-08 | 9 | -0.28 | no |
| Long | 2025-08-08 | 2025-08-18 | 10 | -1.55 | no |
| Short | 2025-08-18 | 2025-08-29 | 11 | 15.66 | no |
| Long | 2025-08-29 | 2025-08-30 | 1 | -18.62 | no |
| Short | 2025-08-30 | 2025-09-04 | 5 | -14.75 | no |
| Long | 2025-09-04 | 2025-09-05 | 1 | -5.02 | no |
| Short | 2025-09-05 | 2025-09-06 | 1 | -1.02 | no |
| Long | 2025-09-06 | 2025-09-20 | 14 | 17.69 | no |
| Short | 2025-09-20 | 2025-09-30 | 10 | 2.39 | no |
| Long | 2025-09-30 | 2025-10-11 | 11 | -7.33 | no |
| Short | 2025-10-11 | 2025-10-21 | 11 | 15.2 | yes |

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

