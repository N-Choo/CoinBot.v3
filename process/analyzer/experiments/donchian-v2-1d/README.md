# Donchian Trend-Following — 5 Years, 1day

Data: 5 years (60 months); expanding training windows M1-M12 through M1-M54, each tested on the next 6 months (OOS M13-M18 through M55-M60). A symbol passes when any top-3 config in any window clears the stress OOS gates (net expectancy > 0, PF > 1, net PnL above buy-and-hold, at least 10 OOS trades) and the parameter-stability rule; the overall experiment passes only when all 5 symbols pass.

**Overall verdict: Fail**

Generated at: 2026-10-23T00:00:00Z

| Symbol | Candles | Winner | Window | OOS stress trades | OOS stress expectancy | OOS stress PF | OOS stress net PnL | Benchmark stress net PnL | Successful configs | Verdict |
|---|---:|---|---|---:|---:|---:|---:|---:|---:|---|
| [BTC-USDT](BTC-USDT/report.md) | 1824 | none | — | — | — | — | — | — | 0 | Fail |
| [ETH-USDT](ETH-USDT/report.md) | 1824 | none | — | — | — | — | — | — | 0 | Fail |
| [SOL-USDT](SOL-USDT/report.md) | 1824 | none | — | — | — | — | — | — | 0 | Fail |
| [BNB-USDT](BNB-USDT/report.md) | 1824 | none | — | — | — | — | — | — | 0 | Fail |
| [XRP-USDT](XRP-USDT/report.md) | 1824 | none | — | — | — | — | — | — | 0 | Fail |

Per-symbol artifacts: `report.md` and `results.json`.
