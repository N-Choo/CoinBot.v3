# ATR SL/TP Grid — BNB-USDT (4hour, 365 days, month-12 OOS)

Focused experiment using fixed RSI signals and an ATR SL/TP grid. This is a research artifact, not a profitability claim. Generated at `2026-09-23T00:00:00Z`.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2025-09-23T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | e6c5eb4ccaeb5a7c4c08ccdb469c3c83aa512b94c6bf1d19eb4130556070ab98 |
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

Drawdown gate applied: 7 of 64 variants have max DD <= 20%.

### Top 3 ATR variants (base costs)

| Rank | Variant | Trades | Net expectancy | PF | Max DD % | Stderr |
|---|---|---:|---:|---:|---:|---:|
| top-1 | SL1.5/TP4.5 | 38 | -4.0475 | 0.6286 | 18.5479 | 3.6000 |
| top-2 | SL1.5/TP1.5 | 38 | -3.2483 | 0.5930 | 15.0534 | 2.6497 |
| top-3 | SL1.5/TP5.0 | 38 | -4.0937 | 0.6243 | 17.8744 | 3.7226 |

### ATR-off baseline (base costs)

| Trades | Net expectancy | PF | Max DD % | Stderr |
|---:|---:|---:|---:|---:|
| 37 | -9.6284 | 0.4884 | 35.6250 | 6.2410 |

## 5. Month 12 OOS

### Top-1: SL1.5/TP4.5

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | 2.3089 | 1.3332 | 2.2098 | 8.5239 | 11.5447 | 0 |
| Stress | 5 | 0.7922 | 1.1011 | 2.5092 | 8.5154 | 3.9608 | 0 |

### Top-2: SL1.5/TP1.5

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -4.2549 | 0.3861 | 2.8307 | 4.5291 | -21.2745 | 0 |
| Stress | 5 | -5.7618 | 0.2644 | 3.4329 | 4.5265 | -28.8092 | 0 |

### Top-3: SL1.5/TP5.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | 3.4029 | 1.4910 | 2.2098 | 9.1921 | 17.0146 | 0 |
| Stress | 5 | 1.8845 | 1.2406 | 2.5092 | 9.1827 | 9.4224 | 0 |

### ATR-off baseline (month 12)

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -14.1443 | 0.0643 | 5.6577 | 6.3594 | -56.5772 | 31.0665 |
| Stress | 4 | -15.6528 | 0.0367 | 6.2611 | 6.3600 | -62.6110 | 29.5175 |

## 6. Conclusion

top-1 SL1.5/TP4.5: month-12 base net expectancy 2.3089 vs ATR-off -14.1443, PF 1.3332 vs 0.0643, closed net PnL 11.5447 + open mark 0 = total net PnL 11.5447 vs ATR-off total -25.5108; stress net expectancy 0.7922 vs ATR-off -15.6528.
top-2 SL1.5/TP1.5: month-12 base net expectancy -4.2549 vs ATR-off -14.1443, PF 0.3861 vs 0.0643, closed net PnL -21.2745 + open mark 0 = total net PnL -21.2745 vs ATR-off total -25.5108; stress net expectancy -5.7618 vs ATR-off -15.6528.
top-3 SL1.5/TP5.0: month-12 base net expectancy 3.4029 vs ATR-off -14.1443, PF 1.4910 vs 0.0643, closed net PnL 17.0146 + open mark 0 = total net PnL 17.0146 vs ATR-off total -25.5108; stress net expectancy 1.8845 vs ATR-off -15.6528.

On month-12 base costs, 3 of 3 top-3 ATR configs beat ATR-off on closed-trade net expectancy, 3 of 3 beat it on PF, 3 beat it on both, and 3 beat it on total net PnL including open marks. Did any top-3 ATR config beat ATR-off on month 12? By net expectancy: Yes. By total net PnL: Yes.

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
