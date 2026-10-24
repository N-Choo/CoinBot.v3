# BTC time-series momentum — Volatility targeting + drawdown control / Short / test / base

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
| Split | test |
| Candles | 365 |
| Range | 2025-10-22 .. 2026-10-21 |
| Variant | Volatility targeting + drawdown control |
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -13.23% | -22.59% |
| CAGR | -13.23% | -22.59% |
| Max drawdown | 20.78% | 48.82% |
| Sharpe | -1.29 | -0.36 |
| Sortino | -1.74 | -0.52 |
| Win rate | 15.79% | 0.00% |
| Trades | 19 | 1 |
| Average holding days | 3.79 | 365.00 |
| Turnover (initial equity) | 26.29 | 1.78 |
| Trading costs | 39.44 | 2.66 |
| Borrow costs | 0 | — |
| Average exposure | -2.87% | 100.00% |
| Average absolute exposure | 12.00% | 100.00% |
| Days with exposure | 72 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 74 | -3.38% | -2.12 | -2.60 | 4.50% | 2.34% | 9.44% |
| Bear | 291 | -10.19% | -1.15 | -1.58 | 20.78% | -4.19% | 12.65% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Short | 2025-10-22 | 2025-10-24 | 2 | -13.44 | no |
| Long | 2025-10-24 | 2025-10-31 | 7 | -12.82 | no |
| Short | 2025-10-31 | 2025-11-12 | 12 | 35.46 | no |
| Long | 2025-11-12 | 2025-11-13 | 1 | -8.33 | no |
| Short | 2025-11-13 | 2025-11-28 | 15 | 53.93 | no |
| Long | 2025-11-28 | 2025-12-02 | 4 | -42.7 | no |
| Short | 2025-12-02 | 2025-12-03 | 1 | -28.63 | no |
| Long | 2025-12-03 | 2025-12-06 | 3 | -8.67 | no |
| Short | 2025-12-06 | 2025-12-08 | 2 | -4.82 | no |
| Long | 2025-12-08 | 2025-12-11 | 3 | 4 | no |
| Short | 2025-12-11 | 2025-12-12 | 1 | -5.3 | no |
| Long | 2025-12-12 | 2025-12-15 | 3 | -34.73 | no |
| Short | 2025-12-15 | 2025-12-22 | 7 | -7.06 | no |
| Long | 2025-12-22 | 2025-12-24 | 2 | -10.49 | no |
| Short | 2025-12-24 | 2025-12-25 | 1 | -3.3 | no |
| Long | 2025-12-25 | 2025-12-27 | 2 | -4.86 | no |
| Short | 2025-12-27 | 2025-12-31 | 4 | -14.55 | no |
| Long | 2025-12-31 | 2026-01-01 | 1 | -11.32 | no |
| Short | 2026-01-01 | 2026-01-02 | 1 | -14.63 | no |

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

