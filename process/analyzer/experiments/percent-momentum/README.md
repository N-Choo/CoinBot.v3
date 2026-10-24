# Percentage Mean Reversion — 1day, 2-year train/test

Generated at 2026-09-23T00:00:00Z; data as of 2026-09-23T00:00:00Z (1790121600000 ms).

Assumptions: symbols BTC-USDT, XRP-USDT, SOL-USDT; 1day candles; first half of the retained history is training and the second half is the untouched test year. The mean-reversion rule sets C2 to the current close and C1 to the close L days earlier: if C1 / C2 - 1 is above +5% (the earlier price was above C2, so price fell) the target is +1 unit, below -5% (price rose) the target is -1 unit, otherwise the position is held. Signals fill at the next open. Base costs are 10 bps fee + 5 bps slippage per side, stress costs are 20 + 10, and borrow is free. One unit is 100% of strategy equity with no leverage beyond 1x; shorts are a backtest simulation only.

Selection: run L=5, 6, 7 on the training half, pick the highest Sharpe (tie: total return), freeze it, then evaluate the test half once.

| Symbol | Candles | Train L=5 Sharpe | Train L=6 Sharpe | Train L=7 Sharpe | Selected L | Test return % (base) | Test Sharpe (base) | Test return % (stress) | B&H return % (base) | Excess (pp) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | 729 | -0.0725 | -0.0186 | -0.4888 | L=6 | -37.7964 | -0.8290 | -41.0803 | -25.0706 | -12.7257 |
| [XRP-USDT](XRP-USDT/report.md) | 729 | -1.2693 | -1.3027 | -1.1065 | L=7 | -12.4516 | 0.1455 | -18.8046 | -48.4361 | 35.9845 |
| [SOL-USDT](SOL-USDT/report.md) | 729 | -0.7022 | -0.1122 | -0.2618 | L=6 | -50.6560 | -0.6858 | -54.0986 | -49.8101 | -0.8460 |

Per-symbol artifacts: `report.md` and `results.json`. Turnover is the sum of absolute position changes; avg abs exposure is a fraction (1.0 = fully invested). Sharpe and Sortino are annualized with 365 daily bars. This is a research artifact, not a profitability claim.
