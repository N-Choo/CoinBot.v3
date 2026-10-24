# ATR SL/TP Grid — XRP-USDT (4hour, 365 days, month-12 OOS)

Focused experiment using fixed RSI signals and an ATR SL/TP grid. This is a research artifact, not a profitability claim. Generated at `2026-09-23T00:00:00Z`.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2025-09-23T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | a96c031654cdb27c4a5cae09b2f16ee44fd96e6ad052184530ca477b88b9cd58 |
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

Drawdown gate applied: 48 of 64 variants have max DD <= 20%.

### Top 3 ATR variants (base costs)

| Rank | Variant | Trades | Net expectancy | PF | Max DD % | Stderr |
|---|---|---:|---:|---:|---:|---:|
| top-1 | SL4.0/TP5.0 | 34 | 5.3142 | 1.3955 | 13.4781 | 6.1444 |
| top-2 | SL4.5/TP5.0 | 34 | 5.1462 | 1.3782 | 13.9880 | 6.1721 |
| top-3 | SL5.0/TP5.0 | 34 | 5.0669 | 1.3702 | 14.2287 | 6.1878 |

### ATR-off baseline (base costs)

| Trades | Net expectancy | PF | Max DD % | Stderr |
|---:|---:|---:|---:|---:|
| 33 | -5.0430 | 0.6820 | 24.9349 | 6.3152 |

## 5. Month 12 OOS

### Top-1: SL4.0/TP5.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 6 | -24.8495 | 0.2852 | 20.8599 | 16.9107 | -149.0968 | 0 |
| Stress | 6 | -26.3748 | 0.2679 | 21.6159 | 16.8976 | -158.2486 | 0 |

### Top-2: SL4.5/TP5.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 6 | -25.4604 | 0.2803 | 21.2265 | 17.0220 | -152.7626 | 0 |
| Stress | 6 | -26.9871 | 0.2634 | 21.9833 | 17.0096 | -161.9227 | 0 |

### Top-3: SL5.0/TP5.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 6 | -25.9771 | 0.2763 | 21.5365 | 17.1604 | -155.8626 | 0 |
| Stress | 6 | -27.5022 | 0.2598 | 22.2923 | 17.1468 | -165.0131 | 0 |

### ATR-off baseline (month 12)

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -45.0736 | 0 | 22.5368 | 3.5953 | -225.3680 | 61.0341 |
| Stress | 5 | -46.5824 | 0 | 23.2912 | 3.5785 | -232.9119 | 59.4401 |

## 6. Conclusion

top-1 SL4.0/TP5.0: month-12 base net expectancy -24.8495 vs ATR-off -45.0736, PF 0.2852 vs 0, closed net PnL -149.0968 + open mark 0 = total net PnL -149.0968 vs ATR-off total -164.3340; stress net expectancy -26.3748 vs ATR-off -46.5824.
top-2 SL4.5/TP5.0: month-12 base net expectancy -25.4604 vs ATR-off -45.0736, PF 0.2803 vs 0, closed net PnL -152.7626 + open mark 0 = total net PnL -152.7626 vs ATR-off total -164.3340; stress net expectancy -26.9871 vs ATR-off -46.5824.
top-3 SL5.0/TP5.0: month-12 base net expectancy -25.9771 vs ATR-off -45.0736, PF 0.2763 vs 0, closed net PnL -155.8626 + open mark 0 = total net PnL -155.8626 vs ATR-off total -164.3340; stress net expectancy -27.5022 vs ATR-off -46.5824.

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
