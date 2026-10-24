# BTC time-series momentum — Volatility targeting + drawdown control / Flat / validation / base

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
| Split | validation |
| Candles | 365 |
| Range | 2024-10-22 .. 2025-10-21 |
| Variant | Volatility targeting + drawdown control |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | 16.89% | 60.29% |
| CAGR | 16.89% | 60.29% |
| Max drawdown | 14.94% | 28.11% |
| Sharpe | 0.97 | 1.29 |
| Sortino | 1.49 | 2.01 |
| Win rate | 35.29% | 100.00% |
| Trades | 17 | 1 |
| Average holding days | 13.06 | 365.00 |
| Turnover (initial equity) | 22.94 | 2.61 |
| Trading costs | 34.41 | 3.91 |
| Borrow costs | 0 | — |
| Average exposure | 33.10% | 100.00% |
| Average absolute exposure | 33.10% | 100.00% |
| Days with exposure | 222 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 328 | 19.22% | 1.15 | 1.77 | 12.78% | 35.84% | 35.84% |
| Bear | 37 | -1.95% | -2.92 | -3.52 | 3.28% | 8.82% | 8.82% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2024-10-22 | 2024-12-22 | 61 | 178.79 | no |
| Long | 2024-12-25 | 2024-12-28 | 3 | -25.45 | no |
| Long | 2025-01-03 | 2025-01-04 | 1 | 4.79 | no |
| Long | 2025-01-05 | 2025-01-06 | 1 | -0.81 | no |
| Long | 2025-01-07 | 2025-01-08 | 1 | -28.55 | no |
| Long | 2025-01-18 | 2025-02-03 | 16 | -34.73 | no |
| Long | 2025-02-04 | 2025-02-05 | 1 | -18.51 | no |
| Long | 2025-02-08 | 2025-02-15 | 7 | 4.05 | no |
| Long | 2025-03-29 | 2025-03-30 | 1 | -7.79 | no |
| Long | 2025-04-10 | 2025-04-11 | 1 | -15.05 | no |
| Long | 2025-04-13 | 2025-04-14 | 1 | -7.79 | no |
| Long | 2025-04-15 | 2025-04-19 | 4 | -1.65 | no |
| Long | 2025-04-20 | 2025-06-18 | 59 | 86.86 | no |
| Long | 2025-06-29 | 2025-08-16 | 48 | 74.78 | no |
| Long | 2025-09-18 | 2025-09-22 | 4 | -11.66 | no |
| Long | 2025-09-25 | 2025-09-26 | 1 | -43.02 | no |
| Long | 2025-09-29 | 2025-10-11 | 12 | 14.64 | no |

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

