# RSI Hyperparameter Tuning (2026-10-23)

Screening run for strict-alternation RSI momentum signals on 4hour and 1day candles.
This is a research artifact, not a profitability claim.

## Configuration

- Symbols: BTC-USDT, BNB-USDT, XRP-USDT, SOL-USDT.
- Timeframes: 4hour and 1day.
- Data: exactly the last 1,000 closed candles per symbol/timeframe, trimmed from a
  34-month fetch window ending 2026-10-23T00:00:00Z.
- Candidate grid: all 24 points (RSI periods 14, 21 x long thresholds 55, 60, 65, 70 x
  short thresholds 40, 45, 50), with the pinned baseline (RSI 21 / 60:50) evaluated first.
- Signal rule: strict long/short alternation. The generator (`SignalGenerator` in
  `strategy/generator.rs`) enforces alternation, accepting a signal only when the previous
  accepted signal was the opposite side; repeated same-side signals are ignored before the
  reversal/open logic. An accepted opposite signal closes the current position and opens the
  new side. The experiment result is unchanged because the simulator already enforced the
  same rule.
- Costs: base 10 bps fee + 5 bps slippage per side; stress 20 bps fee + 10 bps slippage per
  side. Selection uses base-cost validation metrics only.
- Simulation window: 100 warm-up candles.

## Data quality

All eight symbol/timeframe combinations have exactly 1,000 candles, with no missing candles
or duplicates in the fetched range.

| Symbol | Timeframe | Candles | Requested | Expected | Missing | Duplicates |
|---|---|---:|---:|---:|---:|---:|
| BTC-USDT | 4hour | 1000 | 6121 | 6119 | 0 | 0 |
| BNB-USDT | 4hour | 1000 | 6121 | 6119 | 0 | 0 |
| XRP-USDT | 4hour | 1000 | 6121 | 6119 | 0 | 0 |
| SOL-USDT | 4hour | 1000 | 6121 | 6119 | 0 | 0 |
| BTC-USDT | 1day | 1000 | 1021 | 1019 | 0 | 0 |
| BNB-USDT | 1day | 1000 | 1021 | 1019 | 0 | 0 |
| XRP-USDT | 1day | 1000 | 1021 | 1019 | 0 | 0 |
| SOL-USDT | 1day | 1000 | 1021 | 1019 | 0 | 0 |

The 1day combinations have exactly 1,000 closed candles after trimming; the 34-month
request returned 1,019 effective candles, so no 1day symbol is short of the target.

## Selection

Selection uses validation only under base costs. Stress results are reported as a
robustness check and never used for selection.

- **4hour: no candidate meets criteria.** Every grid point was rejected, mostly for having
  positive net expectancy in only 1 of 3 folds; three candidates (`rsi14-long70-short40`,
  `rsi21-long60-short40`, `rsi21-long65-short40`) also breach the 20% per-symbol drawdown
  limit on XRP-USDT.
- **1day: no candidate meets criteria.** Every grid point was rejected, mostly for negative
  validation net expectancy or too few positive validation folds; one candidate
  (`rsi14-long55-short40`) also breaches the drawdown limit on SOL-USDT. Very low trade counts
  make the fold check decisive and noisy.

No candidate was frozen, so only the baseline received holdout metrics.

## Baseline metrics

Pinned baseline RSI 21 / 60:50, pooled across the four symbols under base costs.

| Timeframe | Partition | Trades | Net expectancy | Closed net PnL | Open mark | Profit factor | Max DD % | Stderr |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 4hour | Validation | 22 | 3.4116 | 75.0554 | 51.6523 | 1.1927 | 16.53 | 12.9003 |
| 4hour | Holdout | 15 | -6.0843 | -91.2652 | 68.6135 | 0.5151 | 7.73 | — |
| 1day | Validation | 9 | 2.5472 | 22.9250 | 759.2835 | 1.1015 | 7.02 | 23.2634 |
| 1day | Holdout | 16 | -3.8082 | -60.9316 | 111.1126 | 0.8556 | 17.35 | — |

Net expectancy and closed net PnL are realized closed-trade figures after costs. The open mark
is the costed mark-to-market of positions still open at the partition boundary; it is not
realized and is not included in expectancy. Max drawdown is the maximum per-symbol drawdown,
not a portfolio drawdown, and its percentage is measured from the running equity peak (matching
the replay summary) rather than the initial fund. The validation expectancies are positive but
noisy (standard errors are larger than the point estimates on 1day), and both baseline holdout
partitions have negative closed net PnL.

## Recommendation

**Insufficient evidence; do not promote a tuned configuration.** No candidate cleared the
validation criteria on either timeframe, and the baseline itself was rejected. The apparent
top validation rows are small-sample artifacts (for example, 7 trades on 4hour and 1 trade on
1day), so selecting the highest validation expectancy would be overfitting. Keep the current
research baseline unchanged, treat these results as a negative screening result, and require a
larger sample or a different scope before changing live defaults.

## Artifacts

- `report.md` — generated screening report.
- `leaderboard.csv` — per-candidate, per-partition metrics.
- `results.json` — full machine-readable results.

## Reproduce

```bash
cargo run -p analyzer --features harness --bin analyzer-sweep -- \
  --symbols BTC-USDT,BNB-USDT,XRP-USDT,SOL-USDT \
  --timeframes 4hour,1day --months 34 --max-candles 1000 \
  --as-of 1792713600000 --generated-at 2026-10-23T00:00:00Z \
  --out-dir process/analyzer/experiments/rsi-tuning-2026-10-23 \
  --data-dir data/sweep
```
