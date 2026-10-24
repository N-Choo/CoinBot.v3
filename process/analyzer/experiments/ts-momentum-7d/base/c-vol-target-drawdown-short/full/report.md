# BTC time-series momentum — Volatility targeting + drawdown control / Short / full / base

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
| Split | full |
| Candles | 1824 |
| Range | 2021-10-24 .. 2026-10-21 |
| Variant | Volatility targeting + drawdown control |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -15.50% | 36.79% |
| CAGR | -3.31% | 6.47% |
| Max drawdown | 21.56% | 76.63% |
| Sharpe | -0.25 | 0.38 |
| Sortino | -0.36 | 0.55 |
| Win rate | 25.86% | 100.00% |
| Trades | 58 | 1 |
| Average holding days | 4.79 | 1824.00 |
| Turnover (initial equity) | 53.32 | 2.37 |
| Trading costs | 79.98 | 3.55 |
| Borrow costs | 0 | — |
| Average exposure | -0.93% | 100.00% |
| Average absolute exposure | 5.97% | 100.00% |
| Days with exposure | 278 | 1824 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 918 | 0.00% | — | — | 0.00% | 0.00% | 0.00% |
| Bear | 707 | -17.14% | -0.99 | -1.26 | 21.56% | -1.15% | 4.34% |
| Unknown | 199 | 1.98% | 0.27 | 0.42 | 13.12% | -4.47% | 39.33% |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2021-11-01 | 2021-11-02 | 1 | -3.24 | no |
| Short | 2021-11-02 | 2021-11-03 | 1 | -15.88 | no |
| Long | 2021-11-03 | 2021-11-06 | 3 | -19.27 | no |
| Short | 2021-11-06 | 2021-11-08 | 2 | -22.91 | no |
| Long | 2021-11-08 | 2021-11-16 | 8 | 9.71 | no |
| Short | 2021-11-16 | 2021-11-26 | 10 | 41.19 | no |
| Long | 2021-11-26 | 2021-11-27 | 1 | -37.06 | no |
| Short | 2021-11-27 | 2021-11-30 | 3 | -18.55 | no |
| Long | 2021-11-30 | 2021-12-01 | 1 | -4.04 | no |
| Short | 2021-12-01 | 2021-12-02 | 1 | -1.71 | no |
| Long | 2021-12-02 | 2021-12-03 | 1 | -3.54 | no |
| Short | 2021-12-03 | 2021-12-12 | 9 | 37.37 | no |
| Long | 2021-12-12 | 2021-12-14 | 2 | -18.91 | no |
| Short | 2021-12-14 | 2021-12-17 | 3 | -6.61 | no |
| Long | 2021-12-17 | 2021-12-18 | 1 | -9.16 | no |
| Short | 2021-12-18 | 2021-12-21 | 3 | -5.15 | no |
| Long | 2021-12-21 | 2021-12-23 | 2 | 15.01 | no |
| Short | 2021-12-23 | 2021-12-24 | 1 | -21.12 | no |
| Long | 2021-12-24 | 2021-12-29 | 5 | -31.33 | no |
| Short | 2021-12-29 | 2022-01-13 | 15 | 26.84 | no |
| Long | 2022-01-13 | 2022-01-14 | 1 | -16.64 | no |
| Short | 2022-01-14 | 2022-01-15 | 1 | -6.15 | no |
| Long | 2022-01-15 | 2022-01-19 | 4 | -10.46 | no |
| Short | 2022-01-19 | 2022-01-29 | 10 | 90.75 | no |
| Long | 2022-01-29 | 2022-02-14 | 16 | 27.3 | no |
| Short | 2022-02-14 | 2022-02-16 | 2 | -46.76 | no |
| Long | 2022-02-16 | 2022-02-17 | 1 | -8.34 | no |
| Short | 2022-02-17 | 2022-03-01 | 12 | -2.28 | no |
| Long | 2022-03-01 | 2022-03-05 | 4 | -17.76 | no |
| Short | 2022-03-05 | 2022-03-06 | 1 | -1.43 | no |
| Long | 2022-03-06 | 2022-03-08 | 2 | -5.91 | no |
| Short | 2022-03-08 | 2022-03-15 | 7 | -23.71 | no |
| Long | 2022-03-15 | 2022-03-17 | 2 | 7.13 | no |
| Short | 2022-03-17 | 2022-03-18 | 1 | 0.47 | no |
| Long | 2022-03-18 | 2022-04-04 | 17 | 61.1 | no |
| Short | 2022-04-04 | 2022-04-19 | 15 | 68.74 | no |
| Long | 2022-04-19 | 2022-04-23 | 4 | -18.7 | no |
| Short | 2022-04-23 | 2022-05-05 | 12 | -1.34 | no |
| Long | 2022-05-05 | 2022-05-06 | 1 | -31.86 | no |
| Short | 2022-05-06 | 2022-05-20 | 14 | 41.59 | no |
| Long | 2022-05-20 | 2022-05-21 | 1 | -10.52 | no |
| Short | 2022-05-21 | 2022-05-26 | 5 | -4.03 | no |
| Long | 2022-05-26 | 2022-05-27 | 1 | -4.86 | no |
| Short | 2022-05-27 | 2022-05-31 | 4 | -47.37 | no |
| Long | 2022-05-31 | 2022-06-07 | 7 | -9.01 | no |
| Short | 2022-06-07 | 2022-06-09 | 2 | 10.35 | no |
| Long | 2022-06-09 | 2022-06-10 | 1 | -2.5 | no |
| Short | 2022-06-10 | 2022-06-24 | 14 | 106.05 | no |
| Long | 2022-06-24 | 2022-06-29 | 5 | -18.58 | no |
| Short | 2022-06-29 | 2022-06-30 | 1 | 1.76 | no |
| Long | 2022-06-30 | 2022-07-01 | 1 | -5.37 | no |
| Short | 2022-07-01 | 2022-07-07 | 6 | -29.45 | no |
| Long | 2022-07-07 | 2022-07-12 | 5 | -12.11 | no |
| Short | 2022-07-12 | 2022-07-19 | 7 | -41.89 | no |
| Long | 2022-07-19 | 2022-07-26 | 7 | -17.9 | no |
| Short | 2022-07-26 | 2022-07-29 | 3 | -50.31 | no |
| Long | 2022-07-29 | 2022-08-04 | 6 | -12.39 | no |
| Short | 2022-08-04 | 2022-08-06 | 2 | -24.24 | no |

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

