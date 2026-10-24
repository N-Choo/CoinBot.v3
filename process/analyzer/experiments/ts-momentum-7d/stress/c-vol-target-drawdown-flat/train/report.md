# BTC time-series momentum — Volatility targeting + drawdown control / Flat / train / stress

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
| Split | train |
| Candles | 1094 |
| Range | 2021-10-24 .. 2024-10-21 |
| Variant | Volatility targeting + drawdown control |
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -32.35% | 9.26% |
| CAGR | -12.22% | 3.00% |
| Max drawdown | 32.81% | 76.63% |
| Sharpe | -1.39 | 0.33 |
| Sortino | -1.79 | 0.47 |
| Win rate | 15.79% | 100.00% |
| Trades | 38 | 1 |
| Average holding days | 4.16 | 1094.00 |
| Turnover (initial equity) | 31.23 | 2.10 |
| Trading costs | 93.69 | 6.29 |
| Borrow costs | 0 | — |
| Average exposure | 6.23% | 100.00% |
| Average absolute exposure | 6.23% | 100.00% |
| Days with exposure | 158 | 1094 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 516 | -15.58% | -1.65 | -2.00 | 19.46% | 5.10% | 5.10% |
| Bear | 379 | -7.01% | -1.53 | -1.85 | 7.01% | 1.90% | 1.90% |
| Unknown | 199 | -13.82% | -1.52 | -2.06 | 17.41% | 17.43% | 17.43% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2021-11-01 | 2021-11-02 | 1 | -4.27 | no |
| Long | 2021-11-03 | 2021-11-06 | 3 | -21.07 | no |
| Long | 2021-11-08 | 2021-11-16 | 8 | 8.41 | no |
| Long | 2021-11-26 | 2021-11-27 | 1 | -37.97 | no |
| Long | 2021-11-30 | 2021-12-01 | 1 | -4.79 | no |
| Long | 2021-12-02 | 2021-12-03 | 1 | -4.28 | no |
| Long | 2021-12-12 | 2021-12-14 | 2 | -19.36 | no |
| Long | 2021-12-17 | 2021-12-18 | 1 | -9.75 | no |
| Long | 2021-12-21 | 2021-12-23 | 2 | 13.41 | no |
| Long | 2021-12-24 | 2021-12-29 | 5 | -32.92 | no |
| Long | 2022-01-13 | 2022-01-14 | 1 | -17.61 | no |
| Long | 2022-01-15 | 2022-01-19 | 4 | -11.77 | no |
| Long | 2022-01-29 | 2022-02-14 | 16 | 20.7 | no |
| Long | 2022-02-16 | 2022-02-17 | 1 | -8.94 | no |
| Long | 2022-03-01 | 2022-03-05 | 4 | -16.98 | no |
| Long | 2022-03-06 | 2022-03-08 | 2 | -5.9 | no |
| Long | 2022-03-15 | 2022-03-17 | 2 | 6.18 | no |
| Long | 2022-03-18 | 2022-04-04 | 17 | 56.05 | no |
| Long | 2022-04-19 | 2022-04-23 | 4 | -18.19 | no |
| Long | 2022-05-05 | 2022-05-06 | 1 | -29.17 | no |
| Long | 2022-05-20 | 2022-05-21 | 1 | -9.58 | no |
| Long | 2022-05-26 | 2022-05-27 | 1 | -4.98 | no |
| Long | 2022-05-31 | 2022-06-07 | 7 | -8.95 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -3.27 | no |
| Long | 2022-06-24 | 2022-06-29 | 5 | -15.67 | no |
| Long | 2022-06-30 | 2022-07-01 | 1 | -5.26 | no |
| Long | 2022-07-07 | 2022-07-12 | 5 | -11.18 | no |
| Long | 2024-02-29 | 2024-03-17 | 17 | -9.23 | no |
| Long | 2024-03-26 | 2024-04-02 | 7 | -6.37 | no |
| Long | 2024-04-09 | 2024-04-13 | 4 | -23.15 | no |
| Long | 2024-04-21 | 2024-04-22 | 1 | -1.74 | no |
| Long | 2024-04-23 | 2024-04-27 | 4 | -19.48 | no |
| Long | 2024-05-05 | 2024-05-07 | 2 | -3.84 | no |
| Long | 2024-05-08 | 2024-05-11 | 3 | -7.67 | no |
| Long | 2024-05-16 | 2024-05-28 | 12 | 11.94 | no |
| Long | 2024-05-31 | 2024-06-01 | 1 | -11.95 | no |
| Long | 2024-06-05 | 2024-06-12 | 7 | -33.66 | no |
| Long | 2024-07-02 | 2024-07-04 | 2 | -21.21 | no |

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

