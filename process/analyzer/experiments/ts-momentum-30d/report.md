# BTC 1-day time-series momentum backtest

> Generated at 2026-10-23T00:00:00Z. This is a cost-aware historical simulation, not evidence of future profitability. The test split was not used for tuning.

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
| Costs (stress) | 20 bps fee + 10 bps slippage per side |

Variants:

- **A. Fixed exposure** — long 1.0 when momentum is positive, flat otherwise; short -1.0 in Short mode.
- **B. Volatility targeting** — exposure is `clamp(target_vol / realized_vol, min, max)` with the momentum direction.
- **C. Volatility targeting + drawdown control** — B plus a configurable 20%/10% drawdown hysteresis overlay. The hysteresis is evaluated on the uncapped variant-B reference equity path so it can recover while the live portfolio is flat.

## Chronological splits

Splits are 60% training, 20% validation, and 20% test by candle count. Each split is simulated independently with its own initial fund and uses earlier candles only for signal warm-up. No parameters are tuned.

| Split | Candles | Start index | End index | First date | Last date |
|---|---:|---:|---:|---|---|
| full | 1824 | 0 | 1824 | 2021-10-24 | 2026-10-21 |
| train | 1094 | 0 | 1094 | 2021-10-24 | 2024-10-21 |
| validation | 365 | 1094 | 1459 | 2024-10-22 | 2025-10-21 |
| test | 365 | 1459 | 1824 | 2025-10-22 | 2026-10-21 |

## base costs (10 bps fee + 5 bps slippage)

| Variant | Mode | Split | Net return | CAGR | Max DD | Sharpe | Sortino | Win rate | Trades | Avg hold (d) | Turnover | Trading costs | Borrow costs | Avg abs exposure |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Fixed exposure | Flat | full | 103.37% | 15.26% | 42.98% | 0.59 | 0.93 | 35.00% | 80 | 11.85 | 234.18 | 351.27 | 0 | 51.97% |
| Fixed exposure | Flat | train | 54.36% | 15.58% | 42.98% | 0.59 | 0.91 | 38.64% | 44 | 12.23 | 98.64 | 147.96 | 0 | 49.18% |
| Fixed exposure | Flat | validation | 37.26% | 37.26% | 31.71% | 1.12 | 1.81 | 35.29% | 17 | 13.06 | 43.36 | 65.04 | 0 | 60.82% |
| Fixed exposure | Flat | test | -4.15% | -4.15% | 29.29% | -0.03 | -0.05 | 30.00% | 20 | 9.40 | 33.01 | 49.52 | 0 | 51.51% |
| Fixed exposure | Short | full | 36.89% | 6.48% | 60.95% | 0.38 | 0.56 | 30.63% | 160 | 11.21 | 422.19 | 633.28 | 0 | 98.30% |
| Fixed exposure | Short | train | 21.67% | 6.76% | 58.88% | 0.39 | 0.58 | 34.09% | 88 | 12.08 | 237.16 | 355.74 | 0 | 97.17% |
| Fixed exposure | Short | validation | 7.46% | 7.46% | 45.07% | 0.38 | 0.56 | 29.41% | 34 | 10.74 | 73.14 | 109.7 | 0 | 100.00% |
| Fixed exposure | Short | test | 4.37% | 4.37% | 35.96% | 0.32 | 0.47 | 25.00% | 40 | 9.12 | 75.05 | 112.58 | 0 | 100.00% |
| Volatility targeting | Flat | full | 68.10% | 10.95% | 22.77% | 0.72 | 1.16 | 32.50% | 80 | 11.85 | 119.22 | 178.83 | 0 | 25.54% |
| Volatility targeting | Flat | train | 44.92% | 13.18% | 20.99% | 0.86 | 1.41 | 36.36% | 44 | 12.23 | 48.70 | 73.04 | 0 | 21.94% |
| Volatility targeting | Flat | validation | 16.89% | 16.89% | 14.94% | 0.97 | 1.49 | 35.29% | 17 | 13.06 | 22.94 | 34.41 | 0 | 33.10% |
| Volatility targeting | Flat | test | -0.84% | -0.84% | 19.65% | 0.02 | 0.04 | 25.00% | 20 | 9.40 | 22.43 | 33.64 | 0 | 28.77% |
| Volatility targeting | Short | full | 63.93% | 10.40% | 31.11% | 0.55 | 0.85 | 30.63% | 160 | 11.21 | 239.90 | 359.85 | 0 | 46.02% |
| Volatility targeting | Short | train | 42.68% | 12.59% | 31.11% | 0.64 | 0.99 | 34.09% | 88 | 12.08 | 110.48 | 165.72 | 0 | 41.73% |
| Volatility targeting | Short | validation | 6.18% | 6.18% | 20.97% | 0.38 | 0.55 | 32.35% | 34 | 10.74 | 40.51 | 60.76 | 0 | 52.68% |
| Volatility targeting | Short | test | 8.04% | 8.04% | 19.50% | 0.46 | 0.73 | 22.50% | 40 | 9.12 | 48.12 | 72.17 | 0 | 52.24% |
| Volatility targeting + drawdown control | Flat | full | 27.37% | 4.96% | 22.62% | 0.40 | 0.61 | 32.89% | 76 | 11.43 | 98.15 | 147.23 | 0 | 23.01% |
| Volatility targeting + drawdown control | Flat | train | 27.91% | 8.56% | 20.78% | 0.61 | 0.97 | 37.21% | 43 | 12.14 | 43.22 | 64.83 | 0 | 20.97% |
| Volatility targeting + drawdown control | Flat | validation | 16.89% | 16.89% | 14.94% | 0.97 | 1.49 | 35.29% | 17 | 13.06 | 22.94 | 34.41 | 0 | 33.10% |
| Volatility targeting + drawdown control | Flat | test | -0.84% | -0.84% | 19.65% | 0.02 | 0.04 | 25.00% | 20 | 9.40 | 22.43 | 33.64 | 0 | 28.77% |
| Volatility targeting + drawdown control | Short | full | 0.90% | 0.18% | 31.24% | 0.09 | 0.14 | 33.33% | 81 | 11.38 | 83.02 | 124.54 | 0 | 21.82% |
| Volatility targeting + drawdown control | Short | train | 14.79% | 4.71% | 31.22% | 0.33 | 0.49 | 36.11% | 72 | 12.50 | 74.17 | 111.26 | 0 | 35.48% |
| Volatility targeting + drawdown control | Short | validation | -6.60% | -6.60% | 26.02% | -0.30 | -0.43 | 28.57% | 21 | 9.76 | 26.08 | 39.12 | 0 | 31.43% |
| Volatility targeting + drawdown control | Short | test | 8.04% | 8.04% | 19.50% | 0.46 | 0.73 | 22.50% | 40 | 9.12 | 48.12 | 72.17 | 0 | 52.24% |

Benchmark: BTC buy-and-hold for the same split, with one entry at the first open, one exit at the last close, and the same per-side rate.

| Split | Net return | CAGR | Max DD | Sharpe | Sortino | Trades | Trading costs | Turnover |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| full | 36.79% | 6.47% | 76.63% | 0.38 | 0.55 | 1 | 3.55 | 2.37 |
| train | 9.59% | 3.10% | 76.63% | 0.33 | 0.48 | 1 | 3.15 | 2.10 |
| validation | 60.29% | 60.29% | 28.11% | 1.29 | 2.01 | 1 | 3.91 | 2.61 |
| test | -22.59% | -22.59% | 48.82% | -0.36 | -0.52 | 1 | 2.66 | 1.78 |

## stress costs (20 bps fee + 10 bps slippage)

| Variant | Mode | Split | Net return | CAGR | Max DD | Sharpe | Sortino | Win rate | Trades | Avg hold (d) | Turnover | Trading costs | Borrow costs | Avg abs exposure |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Fixed exposure | Flat | full | 60.13% | 9.88% | 46.07% | 0.45 | 0.70 | 28.75% | 80 | 11.85 | 204.45 | 613.35 | 0 | 51.97% |
| Fixed exposure | Flat | train | 35.43% | 10.65% | 46.07% | 0.46 | 0.71 | 34.09% | 44 | 12.23 | 91.40 | 274.2 | 0 | 49.18% |
| Fixed exposure | Flat | validation | 30.42% | 30.42% | 33.93% | 0.97 | 1.55 | 29.41% | 17 | 13.06 | 42.30 | 126.91 | 0 | 60.82% |
| Fixed exposure | Flat | test | -9.61% | -9.61% | 32.01% | -0.26 | -0.40 | 20.00% | 20 | 9.40 | 32.14 | 96.43 | 0 | 51.51% |
| Fixed exposure | Short | full | -15.35% | -3.28% | 69.03% | 0.19 | 0.27 | 25.00% | 160 | 11.21 | 333.78 | 1001.34 | 0 | 98.30% |
| Fixed exposure | Short | train | -6.53% | -2.23% | 60.46% | 0.23 | 0.34 | 29.55% | 88 | 12.08 | 205.19 | 615.57 | 0 | 97.17% |
| Fixed exposure | Short | validation | -2.86% | -2.86% | 48.90% | 0.16 | 0.23 | 26.47% | 34 | 10.74 | 69.85 | 209.54 | 0 | 100.00% |
| Fixed exposure | Short | test | -7.34% | -7.34% | 40.42% | 0.05 | 0.07 | 17.50% | 40 | 9.12 | 71.10 | 213.3 | 0 | 100.00% |
| Volatility targeting | Flat | full | 47.26% | 8.05% | 25.75% | 0.56 | 0.89 | 30.00% | 80 | 11.85 | 110.91 | 332.74 | 0 | 25.54% |
| Volatility targeting | Flat | train | 35.68% | 10.72% | 23.07% | 0.72 | 1.18 | 34.09% | 44 | 12.23 | 46.96 | 140.88 | 0 | 21.94% |
| Volatility targeting | Flat | validation | 13.39% | 13.39% | 16.14% | 0.80 | 1.22 | 35.29% | 17 | 13.06 | 22.60 | 67.8 | 0 | 33.10% |
| Volatility targeting | Flat | test | -4.43% | -4.43% | 21.40% | -0.22 | -0.35 | 20.00% | 20 | 9.40 | 22.04 | 66.11 | 0 | 28.77% |
| Volatility targeting | Short | full | 26.16% | 4.76% | 38.52% | 0.32 | 0.48 | 26.88% | 160 | 11.21 | 209.33 | 627.98 | 0 | 46.02% |
| Volatility targeting | Short | train | 24.96% | 7.72% | 32.21% | 0.44 | 0.68 | 31.82% | 88 | 12.08 | 102.84 | 308.52 | 0 | 41.73% |
| Volatility targeting | Short | validation | 0.25% | 0.25% | 23.46% | 0.12 | 0.18 | 29.41% | 34 | 10.74 | 39.40 | 118.21 | 0 | 52.68% |
| Volatility targeting | Short | test | 0.41% | 0.41% | 23.22% | 0.13 | 0.20 | 17.50% | 40 | 9.12 | 46.47 | 139.4 | 0 | 52.24% |
| Volatility targeting + drawdown control | Flat | full | 14.02% | 2.66% | 22.00% | 0.25 | 0.39 | 31.34% | 67 | 11.64 | 79.44 | 238.32 | 0 | 20.30% |
| Volatility targeting + drawdown control | Flat | train | 16.93% | 5.36% | 22.00% | 0.42 | 0.66 | 36.59% | 41 | 12.51 | 39.13 | 117.38 | 0 | 20.50% |
| Volatility targeting + drawdown control | Flat | validation | 13.39% | 13.39% | 16.14% | 0.80 | 1.22 | 35.29% | 17 | 13.06 | 22.60 | 67.8 | 0 | 33.10% |
| Volatility targeting + drawdown control | Flat | test | -15.96% | -15.96% | 22.46% | -1.31 | -1.79 | 21.05% | 19 | 8.42 | 19.77 | 59.32 | 0 | 23.84% |
| Volatility targeting + drawdown control | Short | full | 2.64% | 0.52% | 34.67% | 0.11 | 0.17 | 31.88% | 69 | 12.19 | 66.22 | 198.65 | 0 | 19.91% |
| Volatility targeting + drawdown control | Short | train | 2.64% | 0.87% | 34.67% | 0.14 | 0.21 | 31.88% | 69 | 12.19 | 66.22 | 198.65 | 0 | 33.19% |
| Volatility targeting + drawdown control | Short | validation | -2.66% | -2.66% | 20.55% | -0.16 | -0.24 | 16.67% | 12 | 8.75 | 13.06 | 39.18 | 0 | 12.62% |
| Volatility targeting + drawdown control | Short | test | -23.41% | -23.41% | 31.95% | -1.41 | -1.95 | 14.29% | 35 | 7.31 | 37.07 | 111.2 | 0 | 34.31% |

Benchmark: BTC buy-and-hold for the same split, with one entry at the first open, one exit at the last close, and the same per-side rate.

| Split | Net return | CAGR | Max DD | Sharpe | Sortino | Trades | Trading costs | Turnover |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| full | 36.38% | 6.41% | 76.63% | 0.38 | 0.55 | 1 | 7.1 | 2.37 |
| train | 9.26% | 3.00% | 76.63% | 0.33 | 0.47 | 1 | 6.29 | 2.10 |
| validation | 59.81% | 59.81% | 28.11% | 1.28 | 2.00 | 1 | 7.81 | 2.60 |
| test | -22.82% | -22.82% | 48.82% | -0.37 | -0.53 | 1 | 5.32 | 1.77 |

## Regime summary (full period)

| Scenario | Variant | Mode | Regime | Days | Net return | Sharpe | Max DD | Avg abs exposure |
|---|---|---|---|---:|---:|---:|---:|---:|
| base | Fixed exposure | Flat | Bull | 918 | 481.49% | 1.98 | 25.90% | 68.63% |
| base | Fixed exposure | Flat | Bear | 707 | -52.15% | -1.45 | 58.50% | 38.05% |
| base | Fixed exposure | Flat | Unknown | 199 | -26.90% | -1.97 | 30.06% | 24.62% |
| base | Fixed exposure | Short | Bull | 918 | 153.39% | 1.02 | 41.85% | 100.00% |
| base | Fixed exposure | Short | Bear | 707 | -35.80% | -0.19 | 59.60% | 100.00% |
| base | Fixed exposure | Short | Unknown | 199 | -15.86% | -0.21 | 56.52% | 84.42% |
| base | Volatility targeting | Flat | Bull | 918 | 160.95% | 2.08 | 12.78% | 33.02% |
| base | Volatility targeting | Flat | Bear | 707 | -28.32% | -1.26 | 31.86% | 20.81% |
| base | Volatility targeting | Flat | Unknown | 199 | -10.14% | -2.20 | 11.37% | 7.80% |
| base | Volatility targeting | Short | Bull | 918 | 90.87% | 1.23 | 20.67% | 49.64% |
| base | Volatility targeting | Short | Bear | 707 | -13.44% | -0.22 | 27.60% | 46.38% |
| base | Volatility targeting | Short | Unknown | 199 | -0.78% | 0.03 | 23.65% | 28.07% |
| base | Volatility targeting + drawdown control | Flat | Bull | 918 | 103.53% | 1.66 | 12.78% | 31.14% |
| base | Volatility targeting + drawdown control | Flat | Bear | 707 | -30.37% | -1.48 | 33.52% | 16.73% |
| base | Volatility targeting + drawdown control | Flat | Unknown | 199 | -10.14% | -2.20 | 11.37% | 7.80% |
| base | Volatility targeting + drawdown control | Short | Bull | 918 | 18.60% | 0.49 | 21.19% | 25.86% |
| base | Volatility targeting + drawdown control | Short | Bear | 707 | -10.11% | -0.30 | 18.39% | 16.68% |
| base | Volatility targeting + drawdown control | Short | Unknown | 199 | -5.35% | -0.48 | 20.23% | 21.51% |
| stress | Fixed exposure | Flat | Bull | 918 | 432.13% | 1.89 | 27.45% | 68.63% |
| stress | Fixed exposure | Flat | Bear | 707 | -58.08% | -1.73 | 63.21% | 38.05% |
| stress | Fixed exposure | Flat | Unknown | 199 | -28.21% | -2.09 | 31.21% | 24.62% |
| stress | Fixed exposure | Short | Bull | 918 | 112.11% | 0.87 | 44.42% | 100.00% |
| stress | Fixed exposure | Short | Bear | 707 | -50.75% | -0.45 | 67.77% | 100.00% |
| stress | Fixed exposure | Short | Unknown | 199 | -18.97% | -0.32 | 58.07% | 84.42% |
| stress | Volatility targeting | Flat | Bull | 918 | 146.97% | 1.97 | 13.64% | 33.02% |
| stress | Volatility targeting | Flat | Bear | 707 | -33.23% | -1.55 | 36.13% | 20.81% |
| stress | Volatility targeting | Flat | Unknown | 199 | -10.70% | -2.33 | 11.88% | 7.80% |
| stress | Volatility targeting | Short | Bull | 918 | 72.14% | 1.05 | 22.27% | 49.64% |
| stress | Volatility targeting | Short | Bear | 707 | -25.08% | -0.55 | 35.70% | 46.38% |
| stress | Volatility targeting | Short | Unknown | 199 | -2.18% | -0.09 | 24.60% | 28.07% |
| stress | Volatility targeting + drawdown control | Flat | Bull | 918 | 86.58% | 1.50 | 13.64% | 29.04% |
| stress | Volatility targeting + drawdown control | Flat | Bear | 707 | -31.57% | -1.69 | 34.48% | 12.48% |
| stress | Volatility targeting + drawdown control | Flat | Unknown | 199 | -10.70% | -2.33 | 11.88% | 7.80% |
| stress | Volatility targeting + drawdown control | Short | Bull | 918 | 23.58% | 0.60 | 16.95% | 23.46% |
| stress | Volatility targeting + drawdown control | Short | Bear | 707 | -11.79% | -0.37 | 16.74% | 15.65% |
| stress | Volatility targeting + drawdown control | Short | Unknown | 199 | -5.85% | -0.55 | 20.53% | 18.65% |

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

