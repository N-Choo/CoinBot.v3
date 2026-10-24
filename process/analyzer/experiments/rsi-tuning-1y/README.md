# RSI Per-Symbol Tuning — 1 Year, 4hour

Each symbol freezes the top three configs by combined PF/expectancy rank on M1-M4, then judges M5 only. A config passes when M5 net expectancy > 0 and PF > 1; the symbol passes if any top config passes. M6-M10 is logged for context and never changes the decision.

| Symbol | Top | Config | M1-M4 trades | M1-M4 expectancy | M5 trades | M5 expectancy | M5 PF | M6-M10 expectancy | M5 result | Symbol verdict |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---|
| [BTC-USDT](BTC-USDT/report.md) | top-1 | rsi21-long65-short40 | 2 | 20.6234 | 4 | -50.9501 | 0 | -0.9150 | FAIL | Fail |
| [BTC-USDT](BTC-USDT/report.md) | top-2 | rsi21-long70-short40 | 2 | 11.9679 | 0 | 0 | — | -5.1586 | FAIL | Fail |
| [BTC-USDT](BTC-USDT/report.md) | top-3 | rsi14-long70-short50 | 4 | 12.0263 | 4 | -30.6502 | 0 | -5.1096 | FAIL | Fail |
| [BNB-USDT](BNB-USDT/report.md) | top-1 | rsi14-long70-short40 | 2 | 35.8709 | 2 | -46.5672 | 0 | -8.0704 | FAIL | Fail |
| [BNB-USDT](BNB-USDT/report.md) | top-2 | rsi21-long65-short45 | 2 | 25.3432 | 2 | -46.5672 | 0 | -6.2509 | FAIL | Fail |
| [BNB-USDT](BNB-USDT/report.md) | top-3 | rsi21-long70-short45 | 2 | 19.4810 | 0 | 0 | — | -10.5672 | FAIL | Fail |
| [XRP-USDT](XRP-USDT/report.md) | top-1 | rsi21-long65-short40 | 3 | 41.9116 | 2 | -38.1760 | 0 | -19.1461 | FAIL | Fail |
| [XRP-USDT](XRP-USDT/report.md) | top-2 | rsi21-long70-short40 | 3 | 39.5555 | 2 | -59.3079 | 0 | -27.0841 | FAIL | Fail |
| [XRP-USDT](XRP-USDT/report.md) | top-3 | rsi21-long65-short50 | 4 | 39.4613 | 2 | -12.7789 | 0 | -6.2641 | FAIL | Fail |
| [SOL-USDT](SOL-USDT/report.md) | top-1 | rsi21-long65-short50 | 2 | 65.1759 | 4 | -38.0511 | 0 | -8.5331 | FAIL | Fail |
| [SOL-USDT](SOL-USDT/report.md) | top-2 | rsi21-long65-short45 | 2 | 51.5593 | 4 | -47.7326 | 0 | -12.1586 | FAIL | Fail |
| [SOL-USDT](SOL-USDT/report.md) | top-3 | rsi14-long70-short45 | 4 | 64.0439 | 4 | -40.7951 | 0 | -13.4238 | FAIL | Fail |

Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`.
