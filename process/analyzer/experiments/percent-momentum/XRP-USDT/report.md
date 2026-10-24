# Percentage Mean Reversion — XRP-USDT (1day)

Research artifact generated at 2026-09-23T00:00:00Z. This is not a profitability claim.

## Assumptions

- Universe: BTC-USDT, XRP-USDT, SOL-USDT (this report: XRP-USDT).
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
| Symbol | XRP-USDT |
| Timeframe | 1day |
| Candles | 729 |
| Full range | 2024-09-23T00:00:00Z .. 2026-09-21T00:00:00Z |
| Train | 2024-09-23T00:00:00Z .. 2025-09-21T00:00:00Z (364 candles) |
| Test | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z (365 candles) |

## Train tuning (base costs)

Selection rule: highest Sharpe; ties break on total return. The selected row is marked with **selected**.

| Lookback | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| L=5 | -82.3025 | -82.3865 | 89.0860 | -1.2693 | -1.6739 | 29 | 58.6207 | 12.2759 | 58 | 25.7439 | 0.9808 |
| L=6 | -82.8715 | -82.9544 | 88.6299 | -1.3027 | -1.7230 | 29 | 58.6207 | 12.2759 | 58 | 25.7807 | 0.9808 |
| L=7 **selected** | -78.9926 | -79.0825 | 86.0452 | -1.1065 | -1.4671 | 28 | 57.1429 | 12.6071 | 56 | 22.3055 | 0.9725 |

Selected lookback: **L=7**.

## Test (frozen L=7)

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -12.4516 | -12.4516 | 54.3110 | 0.1455 | 0.1996 | 25 | 72.00 | 14.52 | 50 | 79.8883 | 0.9973 |
| Stress | -18.8046 | -18.8046 | 56.3303 | 0.0359 | 0.0489 | 25 | 72.00 | 14.52 | 50 | 154.4254 | 0.9973 |

## Buy-and-hold benchmark (test year)

The benchmark buys at the first test open and sells at the last test close, paying one entry and one exit cost.

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -48.4361 | -48.4361 | 67.3158 | -0.6468 | -0.9632 | 1 | 0 | 364 | 2 | 2.2746 | 1 |
| Stress | -48.5910 | -48.5910 | 67.3158 | -0.6515 | -0.9697 | 1 | 0 | 364 | 2 | 4.5469 | 1 |

Base-cost comparison: strategy -12.4516% vs buy-and-hold -48.4361% (excess 35.9845 percentage points); Sharpe 0.1455 vs -0.6468.

## Limitations

- One fixed threshold (5.00) and three lookbacks; the threshold is not tuned and the selection sample is small.
- One test year per symbol; results are sensitive to the chosen split and market regime.
- Daily closes only; intraday stops, funding, borrow availability, and market impact are not modelled.
- Costs are a flat per-side rate on turnover; the short leg is a paper simulation and has no borrow cost.
- Sharpe and Sortino are annualized with 365 daily bars; CAGR uses the simulated bar count.

