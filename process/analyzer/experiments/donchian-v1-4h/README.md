# Donchian Trend-Following — 15 Months, 4hour

Expanding training windows M1-M6 through M1-M10, each tested on the following five months. A symbol passes when any top-3 config in any window clears the stress OOS gates and beats the stress buy-and-hold benchmark; the overall experiment passes only when all 5 symbols pass.

**Overall verdict: Fail**

Generated at: 2026-10-23T00:00:00Z

| Symbol | Candles | Winner | Window | OOS stress trades | OOS stress expectancy | OOS stress PF | OOS stress net PnL | Benchmark stress net PnL | Successful configs | Verdict |
|---|---:|---|---|---:|---:|---:|---:|---:|---:|---|
| [BTC-USDT](BTC-USDT/report.md) | 2735 | none | — | — | — | — | — | — | 0 | Fail |
| [ETH-USDT](ETH-USDT/report.md) | 2735 | none | — | — | — | — | — | — | 0 | Fail |
| [SOL-USDT](SOL-USDT/report.md) | 2735 | entry55-exit55-ema200-atr14x3 | M1-M8 | 5 | 19.4504 | 4.3800 | 97.2520 | 9.9750 | 2 | Pass |
| [BNB-USDT](BNB-USDT/report.md) | 2735 | none | — | — | — | — | — | — | 0 | Fail |
| [XRP-USDT](XRP-USDT/report.md) | 2735 | entry55-exit55-ema200-atr14x2 | M1-M10 | 8 | 4.2796 | 1.7521 | 34.2370 | -18.4042 | 2 | Pass |

Per-symbol artifacts: `report.md` and `results.json`.
