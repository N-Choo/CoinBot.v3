# BTC time-series momentum — Volatility targeting + drawdown control / Short / validation / base

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
| Mode | Short |

## Results

| Metric | Strategy | BTC buy-and-hold |
|---|---:|---:|
| Net cumulative return | -6.60% | 60.29% |
| CAGR | -6.60% | 60.29% |
| Max drawdown | 26.02% | 28.11% |
| Sharpe | -0.30 | 1.29 |
| Sortino | -0.43 | 2.01 |
| Win rate | 28.57% | 100.00% |
| Trades | 21 | 1 |
| Average holding days | 9.76 | 365.00 |
| Turnover (initial equity) | 26.08 | 2.61 |
| Trading costs | 39.12 | 3.91 |
| Borrow costs | 0 | — |
| Average exposure | 9.29% | 100.00% |
| Average absolute exposure | 31.43% | 100.00% |
| Days with exposure | 205 | 365 |

Equity, exposure, and regime for every day are in `equity.csv`.

## Regime results

Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.

| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bull | 328 | -7.00% | -0.35 | -0.49 | 26.02% | 10.62% | 34.69% |
| Bear | 37 | 0.43% | 1.64 | 4.21 | 0.32% | -2.50% | 2.50% |
| Unknown | 0 | — | — | — | — | — | — |

## Trades

A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.

| Side | Entry | Exit | Holding days | PnL | Closed at end |
|---|---|---|---:|---:|---|
| Long | 2024-10-22 | 2024-12-22 | 61 | 178.79 | no |
| Short | 2024-12-22 | 2024-12-25 | 3 | -9.53 | no |
| Long | 2024-12-25 | 2024-12-28 | 3 | -25.25 | no |
| Short | 2024-12-28 | 2025-01-03 | 6 | -16.77 | no |
| Long | 2025-01-03 | 2025-01-04 | 1 | 4.68 | no |
| Short | 2025-01-04 | 2025-01-05 | 1 | -1.78 | no |
| Long | 2025-01-05 | 2025-01-06 | 1 | -0.8 | no |
| Short | 2025-01-06 | 2025-01-07 | 1 | -22.68 | no |
| Long | 2025-01-07 | 2025-01-08 | 1 | -27.31 | no |
| Short | 2025-01-08 | 2025-01-18 | 10 | -37.48 | no |
| Long | 2025-01-18 | 2025-02-03 | 16 | -32.06 | no |
| Short | 2025-02-03 | 2025-02-04 | 1 | -18.8 | no |
| Long | 2025-02-04 | 2025-02-05 | 1 | -16.77 | no |
| Long | 2025-07-15 | 2025-08-16 | 32 | -11.61 | no |
| Short | 2025-08-16 | 2025-09-18 | 33 | 1.69 | no |
| Long | 2025-09-18 | 2025-09-22 | 4 | -9.3 | no |
| Short | 2025-09-22 | 2025-09-25 | 3 | 10.33 | no |
| Long | 2025-09-25 | 2025-09-26 | 1 | -34.69 | no |
| Short | 2025-09-26 | 2025-09-29 | 3 | -23 | no |
| Long | 2025-09-29 | 2025-10-11 | 12 | 11.51 | no |
| Short | 2025-10-11 | 2025-10-21 | 11 | 14.76 | yes |

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

