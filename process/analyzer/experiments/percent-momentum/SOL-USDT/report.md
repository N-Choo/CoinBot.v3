# Percentage Mean Reversion — SOL-USDT (1day)

Research artifact generated at 2026-09-23T00:00:00Z. This is not a profitability claim.

## Assumptions

- Universe: BTC-USDT, XRP-USDT, SOL-USDT (this report: SOL-USDT).
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
| Symbol | SOL-USDT |
| Timeframe | 1day |
| Candles | 729 |
| Full range | 2024-09-23T00:00:00Z .. 2026-09-21T00:00:00Z |
| Train | 2024-09-23T00:00:00Z .. 2025-09-21T00:00:00Z (364 candles) |
| Test | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z (365 candles) |

## Train tuning (base costs)

Selection rule: highest Sharpe; ties break on total return. The selected row is marked with **selected**.

| Lookback | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| L=5 | -61.0934 | -61.1941 | 69.2974 | -0.7022 | -1.0044 | 29 | 48.2759 | 12.3103 | 58 | 66.3742 | 0.9835 |
| L=6 **selected** | -35.9550 | -36.0333 | 71.4002 | -0.1122 | -0.1638 | 27 | 55.5556 | 13.1852 | 54 | 65.6625 | 0.9808 |
| L=7 | -43.4860 | -43.5746 | 62.0697 | -0.2618 | -0.3771 | 29 | 48.2759 | 12.2414 | 58 | 65.8933 | 0.9780 |

Selected lookback: **L=6**.

## Test (frozen L=6)

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -50.6560 | -50.6560 | 60.0670 | -0.6858 | -0.9390 | 24 | 45.8333 | 15.125 | 48 | 57.2587 | 0.9973 |
| Stress | -54.0986 | -54.0986 | 61.6012 | -0.7907 | -1.0772 | 24 | 45.8333 | 15.125 | 48 | 111.0474 | 0.9973 |

## Buy-and-hold benchmark (test year)

The benchmark buys at the first test open and sells at the last test close, paying one entry and one exit cost.

| Costs | Total return % | CAGR % | Max DD % | Sharpe | Sortino | Trades | Win rate % | Avg holding (days) | Turnover | Costs | Avg abs exposure |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | -49.8101 | -49.8101 | 73.6983 | -0.6557 | -0.9191 | 1 | 0 | 364 | 2 | 2.2540 | 1 |
| Stress | -49.9607 | -49.9607 | 73.7378 | -0.6600 | -0.9249 | 1 | 0 | 364 | 2 | 4.5057 | 1 |

Base-cost comparison: strategy -50.6560% vs buy-and-hold -49.8101% (excess -0.8460 percentage points); Sharpe -0.6858 vs -0.6557.

## Limitations

- One fixed threshold (5.00) and three lookbacks; the threshold is not tuned and the selection sample is small.
- One test year per symbol; results are sensitive to the chosen split and market regime.
- Daily closes only; intraday stops, funding, borrow availability, and market impact are not modelled.
- Costs are a flat per-side rate on turnover; the short leg is a paper simulation and has no borrow cost.
- Sharpe and Sortino are annualized with 365 daily bars; CAGR uses the simulated bar count.

