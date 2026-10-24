# BTC time-series momentum — Volatility targeting / Short / validation / stress

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
| Split | validation |
| Candles | 365 |
| Range | 2024-10-22 .. 2025-10-21 |
| Variant | Volatility targeting |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -48.02% | 59.81% |
| CAGR | -48.02% | 59.81% |
| Max drawdown | 50.13% | 28.11% |
| Sharpe | -2.24 | 1.28 |
| Sortino | -2.84 | 2.00 |
| Win rate | 25.00% | 100.00% |
| Trades | 64 | 1 |
| Average holding days | 5.70 | 365.00 |
| Turnover (initial equity) | 67.55 | 2.60 |
| Trading costs | 202.66 | 7.81 |
| Borrow costs | 0 | — |
| Average exposure | 9.83% | 100.00% |
| Average absolute exposure | 62.61% | 100.00% |
| Days with exposure | 365 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 328 | -37.98% | -1.77 | -2.30 | 40.42% | 11.08% | 64.46% |
| Bear | 37 | -16.19% | -6.81 | -7.09 | 18.06% | -1.32% | 46.15% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2024-10-22 | 2024-10-24 | 2 | -14.69 | no |
| Short | 2024-10-24 | 2024-10-25 | 1 | -24.85 | no |
| Long | 2024-10-25 | 2024-10-26 | 1 | -19.03 | no |
| Short | 2024-10-26 | 2024-10-29 | 3 | -32.55 | no |
| Long | 2024-10-29 | 2024-11-05 | 7 | -12.97 | no |
| Short | 2024-11-05 | 2024-11-07 | 2 | -72.83 | no |
| Long | 2024-11-07 | 2024-11-27 | 20 | 42.81 | no |
| Short | 2024-11-27 | 2024-11-28 | 1 | -16.13 | no |
| Long | 2024-11-28 | 2024-11-29 | 1 | -2.42 | no |
| Short | 2024-11-29 | 2024-12-03 | 4 | -3.2 | no |
| Long | 2024-12-03 | 2024-12-19 | 16 | 6.69 | no |
| Short | 2024-12-19 | 2025-01-03 | 15 | -0.98 | no |
| Long | 2025-01-03 | 2025-01-10 | 7 | -15.78 | no |
| Short | 2025-01-10 | 2025-01-16 | 6 | -38.27 | no |
| Long | 2025-01-16 | 2025-01-28 | 12 | 4.5 | no |
| Short | 2025-01-28 | 2025-01-30 | 2 | -19.77 | no |
| Long | 2025-01-30 | 2025-02-01 | 2 | -11.4 | no |
| Short | 2025-02-01 | 2025-02-13 | 12 | 5.96 | no |
| Long | 2025-02-13 | 2025-02-17 | 4 | -16.97 | no |
| Short | 2025-02-17 | 2025-02-21 | 4 | -21.58 | no |
| Long | 2025-02-21 | 2025-02-22 | 1 | -20.22 | no |
| Short | 2025-02-22 | 2025-02-24 | 2 | -4.14 | no |
| Long | 2025-02-24 | 2025-02-25 | 1 | -35.76 | no |
| Short | 2025-02-25 | 2025-03-06 | 9 | -3 | no |
| Long | 2025-03-06 | 2025-03-10 | 4 | -16.13 | no |
| Short | 2025-03-10 | 2025-03-17 | 7 | -8.83 | no |
| Long | 2025-03-17 | 2025-03-19 | 2 | -1.41 | no |
| Short | 2025-03-19 | 2025-03-20 | 1 | -17.55 | no |
| Long | 2025-03-20 | 2025-03-23 | 3 | -9.9 | no |
| Short | 2025-03-23 | 2025-03-24 | 1 | -8.54 | no |
| Long | 2025-03-24 | 2025-03-30 | 6 | -26.05 | no |
| Short | 2025-03-30 | 2025-04-06 | 7 | -11.78 | no |
| Long | 2025-04-06 | 2025-04-07 | 1 | -23.01 | no |
| Short | 2025-04-07 | 2025-04-10 | 3 | -14.25 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -5.77 | no |
| Short | 2025-04-11 | 2025-04-13 | 2 | -9.58 | no |
| Long | 2025-04-13 | 2025-04-20 | 7 | -0.26 | no |
| Short | 2025-04-20 | 2025-04-21 | 1 | -3.78 | no |
| Long | 2025-04-21 | 2025-05-06 | 15 | 47.47 | no |
| Short | 2025-05-06 | 2025-05-07 | 1 | -13.38 | no |
| Long | 2025-05-07 | 2025-05-18 | 11 | 19.84 | no |
| Short | 2025-05-18 | 2025-05-19 | 1 | -21.66 | no |
| Long | 2025-05-19 | 2025-05-29 | 10 | 2.81 | no |
| Short | 2025-05-29 | 2025-06-07 | 9 | 9.2 | no |
| Long | 2025-06-07 | 2025-06-15 | 8 | 5.7 | no |
| Short | 2025-06-15 | 2025-06-25 | 10 | -4.98 | no |
| Long | 2025-06-25 | 2025-07-02 | 7 | -8.34 | no |
| Short | 2025-07-02 | 2025-07-03 | 1 | -19.7 | no |
| Long | 2025-07-03 | 2025-07-21 | 18 | 24.86 | no |
| Short | 2025-07-21 | 2025-07-23 | 2 | -16.06 | no |
| Long | 2025-07-23 | 2025-07-25 | 2 | -10.55 | no |
| Short | 2025-07-25 | 2025-07-27 | 2 | -1.36 | no |
| Long | 2025-07-27 | 2025-07-30 | 3 | -3.06 | no |
| Short | 2025-07-30 | 2025-08-08 | 9 | 0.2 | no |
| Long | 2025-08-08 | 2025-08-18 | 10 | 2.03 | no |
| Short | 2025-08-18 | 2025-08-29 | 11 | 1.83 | no |
| Long | 2025-08-29 | 2025-08-30 | 1 | -11.21 | no |
| Short | 2025-08-30 | 2025-09-04 | 5 | -11.63 | no |
| Long | 2025-09-04 | 2025-09-05 | 1 | -4.57 | no |
| Short | 2025-09-05 | 2025-09-06 | 1 | -1.71 | no |
| Long | 2025-09-06 | 2025-09-20 | 14 | 19.42 | no |
| Short | 2025-09-20 | 2025-09-30 | 10 | 11.13 | no |
| Long | 2025-09-30 | 2025-10-11 | 11 | -14.82 | no |
| Short | 2025-10-11 | 2025-10-21 | 11 | 1.74 | yes |

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

