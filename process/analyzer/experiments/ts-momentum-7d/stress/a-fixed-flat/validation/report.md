# BTC time-series momentum — Fixed exposure / Flat / validation / stress

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
| Variant | Fixed exposure |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -18.97% | 59.81% |
| CAGR | -18.97% | 59.81% |
| Max drawdown | 43.92% | 28.11% |
| Sharpe | -0.49 | 1.28 |
| Sortino | -0.70 | 2.00 |
| Win rate | 28.12% | 100.00% |
| Trades | 32 | 1 |
| Average holding days | 6.53 | 365.00 |
| Turnover (initial equity) | 57.65 | 2.60 |
| Trading costs | 172.95 | 7.81 |
| Borrow costs | 0 | — |
| Average exposure | 57.26% | 100.00% |
| Average absolute exposure | 57.26% | 100.00% |
| Days with exposure | 209 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 328 | 5.24% | 0.34 | 0.52 | 25.11% | 58.23% | 58.23% |
| Bear | 37 | -23.00% | -7.04 | -6.96 | 25.87% | 48.65% | 48.65% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2024-10-22 | 2024-10-24 | 2 | -16.47 | no |
| Long | 2024-10-25 | 2024-10-26 | 1 | -27.45 | no |
| Long | 2024-10-29 | 2024-11-05 | 7 | -34.31 | no |
| Long | 2024-11-07 | 2024-11-27 | 20 | 193 | no |
| Long | 2024-11-28 | 2024-11-29 | 1 | -9.13 | no |
| Long | 2024-12-03 | 2024-12-19 | 16 | 43.56 | no |
| Long | 2025-01-03 | 2025-01-10 | 7 | -59.13 | no |
| Long | 2025-01-16 | 2025-01-28 | 12 | 10.46 | no |
| Long | 2025-01-30 | 2025-02-01 | 2 | -20.34 | no |
| Long | 2025-02-13 | 2025-02-17 | 4 | -25.54 | no |
| Long | 2025-02-21 | 2025-02-22 | 1 | -28.94 | no |
| Long | 2025-02-24 | 2025-02-25 | 1 | -56.02 | no |
| Long | 2025-03-06 | 2025-03-10 | 4 | -110.91 | no |
| Long | 2025-03-17 | 2025-03-19 | 2 | -3.81 | no |
| Long | 2025-03-20 | 2025-03-23 | 3 | -34.47 | no |
| Long | 2025-03-24 | 2025-03-30 | 6 | -37.51 | no |
| Long | 2025-04-06 | 2025-04-07 | 1 | -52.04 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -30.77 | no |
| Long | 2025-04-13 | 2025-04-20 | 7 | -5.75 | no |
| Long | 2025-04-21 | 2025-05-06 | 15 | 73.18 | no |
| Long | 2025-05-07 | 2025-05-18 | 11 | 44.95 | no |
| Long | 2025-05-19 | 2025-05-29 | 10 | 5.25 | no |
| Long | 2025-06-07 | 2025-06-15 | 8 | 3.76 | no |
| Long | 2025-06-25 | 2025-07-02 | 7 | -8.04 | no |
| Long | 2025-07-03 | 2025-07-21 | 18 | 57.76 | no |
| Long | 2025-07-23 | 2025-07-25 | 2 | -16.94 | no |
| Long | 2025-07-27 | 2025-07-30 | 3 | -4.99 | no |
| Long | 2025-08-08 | 2025-08-18 | 10 | -5.48 | no |
| Long | 2025-08-29 | 2025-08-30 | 1 | -36.4 | no |
| Long | 2025-09-04 | 2025-09-05 | 1 | -11.84 | no |
| Long | 2025-09-06 | 2025-09-20 | 14 | 30.77 | no |
| Long | 2025-09-30 | 2025-10-11 | 11 | -16.11 | no |

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

