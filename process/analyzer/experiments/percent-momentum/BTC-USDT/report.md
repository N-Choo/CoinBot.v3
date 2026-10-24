# Percentage Mean Reversion — BTC-USDT (1day)

Research artifact generated at 2026-09-23T00:00:00Z. This is not a profitability claim.

## Assumptions

- Universe: BTC-USDT, XRP-USDT, SOL-USDT (this report: BTC-USDT).
- Timeframe: 1day.
- Data: 2 years ending 2026-09-23T00:00:00Z (as_of 1790121600000), 729 closed candles retained.
- Split: first 364 candles for training, last 365 for testing. Tuning never reads the test half.
- Rule (mean reversion): C2 is the close on day t and C1 is the close L days earlier (C1 = close[t - L]). If C1 / C2 - 1 > +5.00% (the earlier price was above C2, so price fell) -> target +1 unit (buy); if C1 / C2 - 1 < -5.00% (price rose) -> target -1 unit (sell); otherwise hold the current position.
- Position: one unit is 100% of strategy equity, long or short; no volatility targeting and no leverage beyond 1x. Short selling is a backtest simulation only.
- Execution: C2 is read at the close of day t-1 and filled at the open of day t; no look-ahead.
- Costs: base 10 bps fee + 5 bps slippage per side; stress 20 bps fee + 10 bps slippage per side; borrow cost 0.
- Tuning: L in 5, 6, 7 on the training half; select the highest Sharpe, breaking ties by total return; freeze L before evaluating the test half.
- Metrics: turnover is the sum of absolute position changes in units; avg abs exposure is a fraction (1.0 = fully invested); costs are in quote currency.

## Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| Candles | 729 |
| Full range | 2024-09-23T00:00:00Z .. 2026-09-21T00:00:00Z |
| Train | 2024-09-23T00:00:00Z .. 2025-09-21T00:00:00Z (364 candles) |
| Test | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z (365 candles) |

## Train tuning (base costs)

Selection rule: highest Sharpe; ties break on total return. The selected row is marked with **selected**.

| Lookback | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| L=5 | -11.7087 | -11.7389 | 34.7634 | -0.0725 | -0.1028 | 19 | 63.1579 | 18.6316 | 38 | 57.0646 | 0.9753 |
| L=6 **selected** | -9.6906 | -9.7159 | 31.4469 | -0.0186 | -0.0258 | 18 | 72.2222 | 19.6111 | 36 | 52.6243 | 0.9725 |
| L=7 | -26.2715 | -26.3332 | 40.7798 | -0.4888 | -0.6613 | 15 | 73.3333 | 23.60 | 30 | 38.5170 | 0.9753 |

Selected lookback: **L=6**.

## Test (frozen L=6)

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -37.7964 | -37.7964 | 50.1421 | -0.8290 | -1.0822 | 18 | 66.6667 | 20 | 36 | 54.3926 | 0.9890 |
| Stress | -41.0803 | -41.0803 | 51.5490 | -0.9481 | -1.2335 | 18 | 66.6667 | 20 | 36 | 106.2237 | 0.9890 |

## Buy-and-hold benchmark (test year)

The benchmark buys at the first test open and sells at the last test close, paying one entry and one exit cost.

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -25.0706 | -25.0706 | 52.9630 | -0.4128 | -0.5816 | 1 | 0 | 364 | 2 | 2.6256 | 1 |
| Stress | -25.2956 | -25.2956 | 52.9630 | -0.4197 | -0.5910 | 1 | 0 | 364 | 2 | 5.2479 | 1 |

Base-cost comparison: strategy -37.7964% vs buy-and-hold -25.0706% (excess -12.7257 percentage points); Sharpe -0.8290 vs -0.4128.

## Limitations

- One fixed threshold (5.00) and three lookbacks; the threshold is not tuned and the selection sample is small.
- One test year per symbol; results are sensitive to the chosen split and market regime.
- Daily closes only; intraday stops, funding, borrow availability, and market impact are not modelled.
- Costs are a flat per-side rate on turnover; the short leg is a paper simulation and has no borrow cost.
- Sharpe and Sortino are annualized with 365 daily bars; CAGR uses the simulated bar count.

