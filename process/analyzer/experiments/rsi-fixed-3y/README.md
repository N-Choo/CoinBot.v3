# Fixed RSI 1day Backtest — 3 Years, No Tuning

One fixed configuration across all symbols: RSI 21, long 60, short 40, no ATR/SL/TP, no tuning. Generated at `2026-09-23T00:00:00Z`.

| Symbol | Candles | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % | Verdict | Notes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| [BTC-USDT](BTC-USDT/report.md) | 1094 | 14 | -1.6809 | 0.9540 | 35.4158 | 23.6728 | -23.5325 | 122.9188 | 42.8571 | no base edge | stress expectancy -3.2115 |
| [BNB-USDT](BNB-USDT/report.md) | 1094 | 12 | 16.6308 | 1.4788 | 14.2115 | 36.5116 | 199.5700 | 146.5790 | 33.3333 | positive base expectancy | stress expectancy 15.0694 |
| [SOL-USDT](SOL-USDT/report.md) | 1094 | 6 | -93.3321 | 0.2196 | 61.9890 | 50.6968 | -559.9928 | 0 | 16.6667 | no base edge | stress expectancy -94.9203 |
| [XRP-USDT](XRP-USDT/report.md) | 1094 | 13 | 60.5882 | 2.1050 | 19.1343 | 110.2200 | 787.6468 | 103.7625 | 23.0769 | positive base expectancy | stress expectancy 58.9499 |

## Notes

- Full-period metrics are net of base costs (10 bps fee + 5 bps slippage per side); stress costs are 20 + 10.
- The 3-year window ends one month before the generated-at date.
- The configuration is fixed and is not tuned on this data.
- This report is not a profitability claim.

Per-symbol artifacts: `report.md`, `results.json`.
