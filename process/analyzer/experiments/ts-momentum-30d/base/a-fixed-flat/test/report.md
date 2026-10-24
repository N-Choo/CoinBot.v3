# BTC time-series momentum — Fixed exposure / Flat / test / base

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
| Split | test |
| Candles | 365 |
| Range | 2025-10-22 .. 2026-10-21 |
| Variant | Fixed exposure |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -4.15% | -22.59% |
| CAGR | -4.15% | -22.59% |
| Max drawdown | 29.29% | 48.82% |
| Sharpe | -0.03 | -0.36 |
| Sortino | -0.05 | -0.52 |
| Win rate | 30.00% | 0.00% |
| Trades | 20 | 1 |
| Average holding days | 9.40 | 365.00 |
| Turnover (initial equity) | 33.01 | 1.78 |
| Trading costs | 49.52 | 2.66 |
| Borrow costs | 0 | — |
| Average exposure | 51.51% | 100.00% |
| Average absolute exposure | 51.51% | 100.00% |
| Days with exposure | 188 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 74 | 27.66% | 3.16 | 6.83 | 6.93% | 91.89% | 91.89% |
| Bear | 291 | -24.92% | -1.64 | -2.09 | 29.29% | 41.24% | 41.24% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2025-10-26 | 2025-10-30 | 4 | -17.48 | no |
| Long | 2025-12-21 | 2025-12-25 | 4 | -10.63 | no |
| Long | 2026-01-01 | 2026-01-02 | 1 | 10.29 | no |
| Long | 2026-01-05 | 2026-01-09 | 4 | -7.59 | no |
| Long | 2026-01-12 | 2026-01-21 | 9 | -30.5 | no |
| Long | 2026-01-22 | 2026-01-26 | 4 | -32.15 | no |
| Long | 2026-01-27 | 2026-01-30 | 3 | -40.69 | no |
| Long | 2026-03-08 | 2026-03-09 | 1 | -19.19 | no |
| Long | 2026-03-12 | 2026-03-23 | 11 | -30.82 | no |
| Long | 2026-03-24 | 2026-03-28 | 4 | -54.35 | no |
| Long | 2026-03-30 | 2026-03-31 | 1 | 6.75 | no |
| Long | 2026-04-01 | 2026-04-02 | 1 | -4.15 | no |
| Long | 2026-04-06 | 2026-04-13 | 7 | 16.8 | no |
| Long | 2026-04-14 | 2026-04-16 | 2 | 1.92 | no |
| Long | 2026-04-17 | 2026-05-23 | 36 | 1.78 | no |
| Long | 2026-07-06 | 2026-07-09 | 3 | -19.08 | no |
| Long | 2026-07-10 | 2026-07-14 | 4 | -13.16 | no |
| Long | 2026-07-19 | 2026-08-12 | 24 | -16.75 | no |
| Long | 2026-08-13 | 2026-08-14 | 1 | -2.23 | no |
| Long | 2026-08-19 | 2026-10-21 | 64 | 219.69 | yes |

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

