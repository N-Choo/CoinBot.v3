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
| Fixed exposure | Flat | full | -46.19% | -11.66% | 61.81% | -0.22 | -0.32 | 26.90% | 171 | 5.42 | 209.07 | 313.6 | 0 | 50.77% |
| Fixed exposure | Flat | train | -25.30% | -9.27% | 61.81% | -0.11 | -0.16 | 23.76% | 101 | 5.40 | 122.42 | 183.63 | 0 | 49.82% |
| Fixed exposure | Flat | validation | -10.79% | -10.79% | 41.69% | -0.20 | -0.28 | 28.12% | 32 | 6.53 | 60.30 | 90.45 | 0 | 57.26% |
| Fixed exposure | Flat | test | -19.38% | -19.38% | 31.07% | -0.69 | -1.08 | 33.33% | 39 | 4.41 | 63.36 | 95.03 | 0 | 47.12% |
| Fixed exposure | Short | full | -90.14% | -37.10% | 91.06% | -0.66 | -0.93 | 27.49% | 342 | 5.31 | 234.58 | 351.87 | 0 | 99.56% |
| Fixed exposure | Short | train | -70.50% | -33.45% | 75.56% | -0.47 | -0.67 | 27.86% | 201 | 5.40 | 193.93 | 290.89 | 0 | 99.27% |
| Fixed exposure | Short | validation | -54.89% | -54.89% | 60.65% | -1.56 | -2.05 | 25.00% | 64 | 5.70 | 78.37 | 117.56 | 0 | 100.00% |
| Fixed exposure | Short | test | -26.13% | -26.13% | 39.68% | -0.47 | -0.70 | 27.85% | 79 | 4.62 | 134.24 | 201.35 | 0 | 100.00% |
| Volatility targeting | Flat | full | -18.47% | -4.00% | 34.41% | -0.13 | -0.20 | 27.49% | 171 | 5.42 | 218.66 | 327.99 | 0 | 29.77% |
| Volatility targeting | Flat | train | 3.87% | 1.28% | 30.99% | 0.16 | 0.25 | 23.76% | 101 | 5.40 | 116.70 | 175.05 | 0 | 27.35% |
| Volatility targeting | Flat | validation | -5.12% | -5.12% | 25.44% | -0.15 | -0.22 | 34.38% | 32 | 6.53 | 51.16 | 76.73 | 0 | 36.22% |
| Volatility targeting | Flat | test | -17.36% | -17.36% | 25.20% | -1.07 | -1.60 | 30.77% | 39 | 4.41 | 50.12 | 75.18 | 0 | 30.57% |
| Volatility targeting | Short | full | -70.07% | -21.45% | 72.22% | -0.74 | -1.07 | 28.95% | 342 | 5.31 | 290.57 | 435.85 | 0 | 57.40% |
| Volatility targeting | Short | train | -34.32% | -13.09% | 51.13% | -0.36 | -0.54 | 29.35% | 201 | 5.40 | 201.83 | 302.74 | 0 | 53.41% |
| Volatility targeting | Short | validation | -39.26% | -39.26% | 44.16% | -1.69 | -2.19 | 29.69% | 64 | 5.70 | 72.38 | 108.57 | 0 | 62.61% |
| Volatility targeting | Short | test | -25.13% | -25.13% | 31.64% | -0.97 | -1.44 | 27.85% | 79 | 4.62 | 104.66 | 156.99 | 0 | 64.15% |
| Volatility targeting + drawdown control | Flat | full | -29.80% | -6.84% | 30.52% | -0.53 | -0.74 | 22.22% | 72 | 5.36 | 77.56 | 116.34 | 0 | 10.57% |
| Volatility targeting + drawdown control | Flat | train | -9.81% | -3.39% | 22.40% | -0.20 | -0.31 | 23.64% | 55 | 5.20 | 54.77 | 82.15 | 0 | 12.52% |
| Volatility targeting + drawdown control | Flat | validation | -17.17% | -17.17% | 24.22% | -1.06 | -1.35 | 20.83% | 24 | 5.83 | 35.80 | 53.7 | 0 | 24.42% |
| Volatility targeting + drawdown control | Flat | test | -18.36% | -18.36% | 20.66% | -2.07 | -2.54 | 25.00% | 20 | 3.15 | 22.36 | 33.53 | 0 | 10.72% |
| Volatility targeting + drawdown control | Short | full | -15.50% | -3.31% | 21.56% | -0.25 | -0.36 | 25.86% | 58 | 4.79 | 53.32 | 79.98 | 0 | 5.97% |
| Volatility targeting + drawdown control | Short | train | -15.50% | -5.46% | 21.56% | -0.32 | -0.47 | 25.86% | 58 | 4.79 | 53.32 | 79.98 | 0 | 9.96% |
| Volatility targeting + drawdown control | Short | validation | -20.11% | -20.11% | 20.11% | -1.43 | -1.75 | 23.53% | 17 | 6.00 | 22.34 | 33.5 | 0 | 14.30% |
| Volatility targeting + drawdown control | Short | test | -13.23% | -13.23% | 20.78% | -1.29 | -1.74 | 15.79% | 19 | 3.79 | 26.29 | 39.44 | 0 | 12.00% |

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
| Fixed exposure | Flat | full | -67.82% | -20.30% | 71.00% | -0.53 | -0.78 | 23.98% | 171 | 5.42 | 164.51 | 493.52 | 0 | 50.77% |
| Fixed exposure | Flat | train | -44.78% | -17.98% | 66.14% | -0.40 | -0.59 | 21.78% | 101 | 5.40 | 106.24 | 318.73 | 0 | 49.82% |
| Fixed exposure | Flat | validation | -18.97% | -18.97% | 43.92% | -0.49 | -0.70 | 28.12% | 32 | 6.53 | 57.65 | 172.95 | 0 | 57.26% |
| Fixed exposure | Flat | test | -28.30% | -28.30% | 37.30% | -1.13 | -1.74 | 25.64% | 39 | 4.41 | 59.92 | 179.76 | 0 | 47.12% |
| Fixed exposure | Short | full | -96.48% | -48.80% | 96.48% | -1.06 | -1.48 | 24.85% | 342 | 5.31 | 174.09 | 522.27 | 0 | 99.56% |
| Fixed exposure | Short | train | -83.88% | -45.60% | 85.32% | -0.84 | -1.18 | 25.37% | 201 | 5.40 | 155.31 | 465.94 | 0 | 99.27% |
| Fixed exposure | Short | validation | -62.75% | -62.75% | 64.94% | -1.97 | -2.55 | 23.44% | 64 | 5.70 | 72.60 | 217.8 | 0 | 100.00% |
| Fixed exposure | Short | test | -41.69% | -41.69% | 49.52% | -1.00 | -1.46 | 24.05% | 79 | 4.62 | 120.11 | 360.33 | 0 | 100.00% |
| Volatility targeting | Flat | full | -43.74% | -10.87% | 47.08% | -0.53 | -0.80 | 23.98% | 171 | 5.42 | 182.29 | 546.87 | 0 | 29.77% |
| Volatility targeting | Flat | train | -15.22% | -5.36% | 38.64% | -0.21 | -0.33 | 21.78% | 101 | 5.40 | 105.35 | 316.06 | 0 | 27.35% |
| Volatility targeting | Flat | validation | -12.53% | -12.53% | 27.21% | -0.55 | -0.77 | 31.25% | 32 | 6.53 | 49.17 | 147.52 | 0 | 36.22% |
| Volatility targeting | Flat | test | -24.29% | -24.29% | 30.23% | -1.59 | -2.34 | 23.08% | 39 | 4.41 | 48.07 | 144.2 | 0 | 30.57% |
| Volatility targeting | Short | full | -85.81% | -32.34% | 85.92% | -1.28 | -1.80 | 25.44% | 342 | 5.31 | 219.46 | 658.39 | 0 | 57.40% |
| Volatility targeting | Short | train | -56.52% | -24.26% | 61.37% | -0.85 | -1.25 | 26.37% | 201 | 5.40 | 168.63 | 505.9 | 0 | 53.41% |
| Volatility targeting | Short | validation | -48.02% | -48.02% | 50.13% | -2.24 | -2.84 | 25.00% | 64 | 5.70 | 67.55 | 202.66 | 0 | 62.61% |
| Volatility targeting | Short | test | -37.46% | -37.46% | 42.29% | -1.65 | -2.36 | 24.05% | 79 | 4.62 | 96.16 | 288.48 | 0 | 64.15% |
| Volatility targeting + drawdown control | Flat | full | -32.35% | -7.52% | 32.81% | -1.07 | -1.39 | 15.79% | 38 | 4.16 | 31.23 | 93.69 | 0 | 3.74% |
| Volatility targeting + drawdown control | Flat | train | -32.35% | -12.22% | 32.81% | -1.39 | -1.79 | 15.79% | 38 | 4.16 | 31.23 | 93.69 | 0 | 6.23% |
| Volatility targeting + drawdown control | Flat | validation | -17.78% | -17.78% | 21.96% | -1.38 | -1.74 | 18.75% | 16 | 5.50 | 22.84 | 68.53 | 0 | 12.62% |
| Volatility targeting + drawdown control | Flat | test | -18.08% | -18.08% | 20.29% | -2.53 | -2.93 | 25.00% | 16 | 3.00 | 18.48 | 55.45 | 0 | 8.66% |
| Volatility targeting + drawdown control | Short | full | -19.64% | -4.28% | 20.26% | -0.34 | -0.50 | 24.56% | 57 | 4.82 | 49.58 | 148.74 | 0 | 5.87% |
| Volatility targeting + drawdown control | Short | train | -19.64% | -7.03% | 20.26% | -0.44 | -0.64 | 24.56% | 57 | 4.82 | 49.58 | 148.74 | 0 | 9.78% |
| Volatility targeting + drawdown control | Short | validation | -20.42% | -20.42% | 20.42% | -1.54 | -1.85 | 14.29% | 14 | 6.14 | 17.82 | 53.45 | 0 | 11.80% |
| Volatility targeting + drawdown control | Short | test | -13.74% | -13.74% | 20.41% | -1.35 | -1.81 | 17.65% | 17 | 4.12 | 22.39 | 67.17 | 0 | 11.45% |

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
| base | Fixed exposure | Flat | Bull | 918 | 176.87% | 1.33 | 29.40% | 58.28% |
| base | Fixed exposure | Flat | Bear | 707 | -71.75% | -2.27 | 71.79% | 44.41% |
| base | Fixed exposure | Flat | Unknown | 199 | -31.21% | -1.68 | 35.60% | 38.69% |
| base | Fixed exposure | Short | Bull | 918 | -46.97% | -0.30 | 58.42% | 100.00% |
| base | Fixed exposure | Short | Bear | 707 | -76.95% | -1.21 | 83.78% | 100.00% |
| base | Fixed exposure | Short | Unknown | 199 | -19.31% | -0.29 | 42.41% | 95.98% |
| base | Volatility targeting | Flat | Bull | 918 | 69.09% | 1.12 | 18.04% | 34.46% |
| base | Volatility targeting | Flat | Bear | 707 | -45.58% | -1.94 | 45.66% | 27.14% |
| base | Volatility targeting | Flat | Unknown | 199 | -11.39% | -1.23 | 16.29% | 17.43% |
| base | Volatility targeting | Short | Bull | 918 | -35.64% | -0.49 | 43.25% | 60.97% |
| base | Volatility targeting | Short | Bear | 707 | -54.40% | -1.38 | 56.83% | 57.85% |
| base | Volatility targeting | Short | Unknown | 199 | 1.98% | 0.27 | 13.12% | 39.33% |
| base | Volatility targeting + drawdown control | Flat | Bull | 918 | 3.27% | 0.16 | 21.10% | 13.95% |
| base | Volatility targeting + drawdown control | Flat | Bear | 707 | -23.28% | -2.12 | 23.28% | 4.25% |
| base | Volatility targeting + drawdown control | Flat | Unknown | 199 | -11.39% | -1.23 | 16.29% | 17.43% |
| base | Volatility targeting + drawdown control | Short | Bull | 918 | 0.00% | — | 0.00% | 0.00% |
| base | Volatility targeting + drawdown control | Short | Bear | 707 | -17.14% | -0.99 | 21.56% | 4.34% |
| base | Volatility targeting + drawdown control | Short | Unknown | 199 | 1.98% | 0.27 | 13.12% | 39.33% |
| stress | Fixed exposure | Flat | Bull | 918 | 119.65% | 1.06 | 31.49% | 58.28% |
| stress | Fixed exposure | Flat | Bear | 707 | -77.39% | -2.69 | 77.39% | 44.41% |
| stress | Fixed exposure | Flat | Unknown | 199 | -35.22% | -1.97 | 38.25% | 38.69% |
| stress | Fixed exposure | Short | Bull | 918 | -66.66% | -0.69 | 73.54% | 100.00% |
| stress | Fixed exposure | Short | Bear | 707 | -85.25% | -1.66 | 88.44% | 100.00% |
| stress | Fixed exposure | Short | Unknown | 199 | -28.36% | -0.63 | 47.62% | 95.98% |
| stress | Volatility targeting | Flat | Bull | 918 | 40.08% | 0.75 | 19.46% | 34.46% |
| stress | Volatility targeting | Flat | Bear | 707 | -53.40% | -2.45 | 53.40% | 27.14% |
| stress | Volatility targeting | Flat | Unknown | 199 | -13.82% | -1.52 | 17.41% | 17.43% |
| stress | Volatility targeting | Short | Bull | 918 | -55.41% | -1.01 | 60.59% | 60.97% |
| stress | Volatility targeting | Short | Bear | 707 | -66.96% | -2.00 | 68.42% | 57.85% |
| stress | Volatility targeting | Short | Unknown | 199 | -3.70% | -0.10 | 15.70% | 39.33% |
| stress | Volatility targeting + drawdown control | Flat | Bull | 918 | -15.58% | -1.24 | 19.46% | 2.87% |
| stress | Volatility targeting + drawdown control | Flat | Bear | 707 | -7.01% | -1.12 | 7.01% | 1.02% |
| stress | Volatility targeting + drawdown control | Flat | Unknown | 199 | -13.82% | -1.52 | 17.41% | 17.43% |
| stress | Volatility targeting + drawdown control | Short | Bull | 918 | 0.00% | — | 0.00% | 0.00% |
| stress | Volatility targeting + drawdown control | Short | Bear | 707 | -16.55% | -0.99 | 20.26% | 4.06% |
| stress | Volatility targeting + drawdown control | Short | Unknown | 199 | -3.70% | -0.10 | 15.70% | 39.33% |

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

