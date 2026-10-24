# ATR SL/TP Grid — BTC-USDT (4hour, 365 days, month-12 OOS)

Focused experiment using fixed RSI signals and an ATR SL/TP grid. This is a research artifact, not a profitability claim. Generated at `2026-09-23T00:00:00Z`.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2025-09-23T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | 0ff631ea7544cee16593636b9fdf0d367d6dbcf00586cccdd0e01ef1a0f3e83c |
| As-of | 2026-09-23T00:00:00Z (1790121600000 ms) |
| Requested window | 365 days |

## 2. Split

| Partition | Candle range | Candles | First open | Last open |
|---|---|---:|---|---|
| Train (first 11/12) | 0..2006 | 2006 | 2025-09-23T00:00:00Z | 2026-08-23T04:00:00Z |
| Month 12 OOS (final 1/12) | 2006..2189 | 183 | 2026-08-23T08:00:00Z | 2026-09-22T16:00:00Z |

Both partitions use the existing 100-candle warm-up; only trades opened inside the partition are counted.

## 3. Experiment

| Field | Value |
|---|---|
| Strategy version | rsi-mom-v1 |
| RSI | period 16 / long 60 / short 40 |
| ATR period | 14 |
| ATR grid | SL 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0 × TP 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0 = 64 variants |
| High-vol adjustment | both exits × 1.5 when ATR% > 5 |
| Stop cap | none |
| Baseline | ATR off, same RSI signals |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Initial fund | 1000 |
| Warm-up window | 100 candles |
| Train ranking | Combined PF rank + net expectancy rank; PF None/infinite ranks first; no trade-count gate |
| Drawdown gate | max DD <= 20% when at least 3 variants qualify |

## 4. Train results — first 11/12

Drawdown gate applied: 47 of 64 variants have max DD <= 20%.

### Top 3 ATR variants (base costs)

| Rank | Variant | Trades | Net expectancy | PF | Max DD % | Stderr |
|---|---|---:|---:|---:|---:|---:|
| top-1 | SL5.0/TP3.0 | 34 | -1.4889 | 0.8612 | 12.5533 | 3.7862 |
| top-2 | SL4.5/TP3.0 | 34 | -1.5194 | 0.8588 | 12.6549 | 3.8024 |
| top-3 | SL2.0/TP3.0 | 34 | -1.8142 | 0.7947 | 11.2549 | 2.8734 |

### ATR-off baseline (base costs)

| Trades | Net expectancy | PF | Max DD % | Stderr |
|---:|---:|---:|---:|---:|
| 33 | -1.6850 | 0.8712 | 16.6614 | 6.0627 |

## 5. Month 12 OOS

### Top-1: SL5.0/TP3.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 7 | -13.0299 | 0.1343 | 10.5359 | 5.2439 | -91.2091 | 0 |
| Stress | 7 | -14.5321 | 0.1104 | 11.4351 | 5.2418 | -101.7245 | 0 |

### Top-2: SL4.5/TP3.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 7 | -13.2968 | 0.1320 | 10.7228 | 5.1839 | -93.0779 | 0 |
| Stress | 7 | -14.7974 | 0.1087 | 11.6209 | 5.1802 | -103.5821 | 0 |

### Top-3: SL2.0/TP3.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 7 | -8.7477 | 0.1877 | 7.5384 | 3.8773 | -61.2338 | 0 |
| Stress | 7 | -10.2500 | 0.1496 | 8.4377 | 3.8733 | -71.7501 | 0 |

### ATR-off baseline (month 12)

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 6 | -17.7712 | 0 | 10.6627 | 3.2918 | -106.6274 | 50.5383 |
| Stress | 6 | -19.2702 | 0 | 11.5621 | 3.2948 | -115.6211 | 48.9601 |

## 6. Conclusion

top-1 SL5.0/TP3.0: month-12 base net expectancy -13.0299 vs ATR-off -17.7712, PF 0.1343 vs 0, closed net PnL -91.2091 + open mark 0 = total net PnL -91.2091 vs ATR-off total -56.0891; stress net expectancy -14.5321 vs ATR-off -19.2702.
top-2 SL4.5/TP3.0: month-12 base net expectancy -13.2968 vs ATR-off -17.7712, PF 0.1320 vs 0, closed net PnL -93.0779 + open mark 0 = total net PnL -93.0779 vs ATR-off total -56.0891; stress net expectancy -14.7974 vs ATR-off -19.2702.
top-3 SL2.0/TP3.0: month-12 base net expectancy -8.7477 vs ATR-off -17.7712, PF 0.1877 vs 0, closed net PnL -61.2338 + open mark 0 = total net PnL -61.2338 vs ATR-off total -56.0891; stress net expectancy -10.2500 vs ATR-off -19.2702.

On month-12 base costs, 3 of 3 top-3 ATR configs beat ATR-off on closed-trade net expectancy, 3 of 3 beat it on PF, 3 beat it on both, and 0 beat it on total net PnL including open marks. Did any top-3 ATR config beat ATR-off on month 12? By net expectancy: Yes. By total net PnL: No.

## 7. Execution assumptions

- A signal produced at a candle close fills at the next candle open.
- One position per symbol; opposite signals reverse at the fill open.
- ATR is computed on the signal candle; SL/TP levels use the signal close.
- SL/TP are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched.
- The high-vol adjustment multiplies both exits by 1.5 when ATR% > 5 and has no stop cap.
- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks.

## 8. Notes

- Train ranking uses base costs only. Stress costs are reported on the final month for the frozen top 3 and the ATR-off baseline.
- The month-12 result is not used to select the top 3.
- This report is not a profitability claim.

Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`.
