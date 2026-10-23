# RSI Momentum Screening (2026-10-23)

Fresh screening run for the new RSI momentum strategy. This directory is separate from the
historical RSI-divergence reports under `experiments/sweep`, `experiments/cost-ladder`,
`experiments/execution`, and `experiments/confirm`.

## Configuration

- Strategy: RSI(21), long on a cross above 60, short/exit on a cross below 50.
- ATR SL/TP: disabled by default (`ATR_ENABLED=false`); enabled candidates still use the
  ATR grid.
- Timeframe: 4hour.
- Symbols: BTC-USDT, ETH-USDT, SOL-USDT.
- History: 12 months ending 2026-10-23T00:00:00Z.
- Data: 2,159 4hour candles per symbol, no gaps or duplicates.
- Costs: base 10 bps fee + 5 bps slippage per side; stress 20 bps + 10 bps per side.
- Candidate grid: 30 sampled from 432 (2 RSI periods x 4 long thresholds x 3 short
  thresholds x 2 ATR states x 3 SL multipliers x 3 TP multipliers).

## Result

**Insufficient evidence — no candidate met the validation criteria.**

Baseline (RSI 21 / 60:50, ATR off) validation:

| Metric | Value |
| --- | --- |
| Trades | 34 |
| Net expectancy | -13.7703 |
| Net PnL | -468.92 |
| Profit factor | 0.0943 |
| Max drawdown | 19.87% |

The sweep report recommends keeping the study baseline rather than selecting a candidate.
No holdout was scored because no candidate was frozen. This is a 12-month, 30-of-432
screening run, not a profitability claim and not a substitute for the larger studies cited
in the strategy README.

## Artifacts

- `sweep/report.md` — generated screening report.
- `sweep/leaderboard.csv` — per-candidate, per-partition metrics.
- `sweep/results.json` — full machine-readable results.
- `replay.html` — deterministic offline replay for BTC/ETH/SOL 4hour.

## Reproduce

```bash
cargo run -p analyzer --features harness --bin analyzer-sweep -- \
  --symbols BTC-USDT,ETH-USDT,SOL-USDT --timeframes 4hour --months 12 \
  --as-of 1792713600000 --generated-at 2026-10-23T00:00:00Z \
  --out-dir process/analyzer/experiments/rsi-momentum/sweep --data-dir data/sweep

cargo run -p analyzer --features harness --bin analyzer-replay -- \
  --timeframe 4hour --as-of 1792713600000 --generated-at 2026-10-23T00:00:00Z \
  --data BTC-USDT=data/sweep/BTC-USDT_4hour.csv \
  --data ETH-USDT=data/sweep/ETH-USDT_4hour.csv \
  --data SOL-USDT=data/sweep/SOL-USDT_4hour.csv \
  --out process/analyzer/experiments/rsi-momentum/replay.html
```
