# RSI Walk-Forward — 15 Months, 4hour

Expanding training windows M1-M6 through M1-M10, each tested on the following five months. A symbol passes when at least one tested config in any window is successful; the overall experiment passes only when all 5 symbols pass.

**Overall verdict: Fail**

| Symbol | Candles | Winner | Window | OOS trades | OOS expectancy | OOS PF | Successful configs | Verdict |
|---|---:|---|---|---:|---:|---:|---:|---|
| [BTC-USDT](BTC-USDT/report.md) | 2735 | rsi14-long70-short50 | M1-M9 | 13 | 4.9364 | 1.4652 | 3 | Pass |
| [ETH-USDT](ETH-USDT/report.md) | 2735 | none | — | — | — | — | 0 | Fail |
| [SOL-USDT](SOL-USDT/report.md) | 2735 | none | — | — | — | — | 0 | Fail |
| [XRP-USDT](XRP-USDT/report.md) | 2735 | rsi14-long70-short50 | M1-M10 | 16 | 2.5595 | 1.1609 | 2 | Pass |
| [BNB-USDT](BNB-USDT/report.md) | 2735 | none | — | — | — | — | 0 | Fail |

Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`.
