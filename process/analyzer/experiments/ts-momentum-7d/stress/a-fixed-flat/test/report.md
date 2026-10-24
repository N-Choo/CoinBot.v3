# BTC time-series momentum — Fixed exposure / Flat / test / stress

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
| Mode | Flat |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -28.30% | -22.82% |
| CAGR | -28.30% | -22.82% |
| Max drawdown | 37.30% | 48.82% |
| Sharpe | -1.13 | -0.37 |
| Sortino | -1.74 | -0.53 |
| Win rate | 25.64% | 0.00% |
| Trades | 39 | 1 |
| Average holding days | 4.41 | 365.00 |
| Turnover (initial equity) | 59.92 | 1.77 |
| Trading costs | 179.76 | 5.32 |
| Borrow costs | 0 | — |
| Average exposure | 47.12% | 100.00% |
| Average absolute exposure | 47.12% | 100.00% |
| Days with exposure | 172 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 74 | 9.52% | 1.45 | 3.23 | 11.75% | 56.76% | 56.76% |
| Bear | 291 | -34.53% | -2.13 | -2.88 | 34.53% | 44.67% | 44.67% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2025-10-24 | 2025-10-31 | 7 | -21.83 | no |
| Long | 2025-11-12 | 2025-11-13 | 1 | -19.06 | no |
| Long | 2025-11-28 | 2025-12-02 | 4 | -58.43 | no |
| Long | 2025-12-03 | 2025-12-06 | 3 | -24.46 | no |
| Long | 2025-12-08 | 2025-12-11 | 3 | 10.35 | no |
| Long | 2025-12-12 | 2025-12-15 | 3 | -46.67 | no |
| Long | 2025-12-22 | 2025-12-24 | 2 | -15.95 | no |
| Long | 2025-12-25 | 2025-12-27 | 2 | -7.75 | no |
| Long | 2025-12-31 | 2026-01-01 | 1 | -12.76 | no |
| Long | 2026-01-02 | 2026-01-11 | 9 | 10.19 | no |
| Long | 2026-01-14 | 2026-01-21 | 7 | -64.09 | no |
| Long | 2026-01-28 | 2026-01-29 | 1 | -4.07 | no |
| Long | 2026-02-13 | 2026-02-14 | 1 | 24.41 | no |
| Long | 2026-02-15 | 2026-02-16 | 1 | -15.47 | no |
| Long | 2026-02-20 | 2026-02-21 | 1 | 6.87 | no |
| Long | 2026-02-26 | 2026-02-28 | 2 | -28.08 | no |
| Long | 2026-03-03 | 2026-03-10 | 7 | -8.51 | no |
| Long | 2026-03-11 | 2026-03-12 | 1 | -1.77 | no |
| Long | 2026-03-14 | 2026-03-20 | 6 | -14.52 | no |
| Long | 2026-03-26 | 2026-03-27 | 1 | -29.01 | no |
| Long | 2026-04-04 | 2026-04-30 | 26 | 84.86 | no |
| Long | 2026-05-02 | 2026-05-04 | 2 | -1.28 | no |
| Long | 2026-05-05 | 2026-05-13 | 8 | 1.5 | no |
| Long | 2026-05-15 | 2026-05-16 | 1 | -23.23 | no |
| Long | 2026-05-26 | 2026-05-27 | 1 | -17.72 | no |
| Long | 2026-06-13 | 2026-06-19 | 6 | -11.38 | no |
| Long | 2026-07-03 | 2026-07-14 | 11 | 4.71 | no |
| Long | 2026-07-15 | 2026-07-18 | 3 | -16.43 | no |
| Long | 2026-07-19 | 2026-07-26 | 7 | -9.23 | no |
| Long | 2026-07-27 | 2026-07-28 | 1 | -21.36 | no |
| Long | 2026-08-05 | 2026-08-07 | 2 | -1.68 | no |
| Long | 2026-08-08 | 2026-08-12 | 4 | -17.55 | no |
| Long | 2026-08-18 | 2026-08-29 | 11 | 129.43 | no |
| Long | 2026-08-30 | 2026-08-31 | 1 | -10.2 | no |
| Long | 2026-09-04 | 2026-09-11 | 7 | -48.95 | no |
| Long | 2026-09-19 | 2026-09-29 | 10 | 18.85 | no |
| Long | 2026-10-02 | 2026-10-08 | 6 | -18.04 | no |
| Long | 2026-10-16 | 2026-10-17 | 1 | -9.87 | no |
| Long | 2026-10-20 | 2026-10-21 | 1 | 5.22 | no |

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

