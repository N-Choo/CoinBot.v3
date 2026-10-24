# ATR SL/TP Grid — SOL-USDT (4hour, 365 days, month-12 OOS)

Focused experiment using fixed RSI signals and an ATR SL/TP grid. This is a research artifact, not a profitability claim. Generated at `2026-09-23T00:00:00Z`.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2025-09-23T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 2189 |
| SHA-256 | 1bf2ffca0a67cd3067d9c3ad0b059f24d61c27cd36bad8ec4a3f9001a99f14f2 |
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

Drawdown gate failed: only 0 of 64 variants have max DD <= 20%; top 3 selected from all 64 variants without the gate.

### Top 3 ATR variants (base costs)

| Rank | Variant | Trades | Net expectancy | PF | Max DD % | Stderr |
|---|---|---:|---:|---:|---:|---:|
| top-1 | SL2.5/TP3.0 | 38 | -4.5990 | 0.7164 | 33.0220 | 4.6478 |
| top-2 | SL2.5/TP1.5 | 38 | -4.2177 | 0.6499 | 23.7496 | 3.5014 |
| top-3 | SL2.5/TP4.5 | 38 | -6.2968 | 0.6529 | 35.8078 | 5.1622 |

### ATR-off baseline (base costs)

| Trades | Net expectancy | PF | Max DD % | Stderr |
|---:|---:|---:|---:|---:|
| 37 | -10.1546 | 0.5883 | 46.8295 | 8.5855 |

## 5. Month 12 OOS

### Top-1: SL2.5/TP3.0

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -2.6212 | 0.8270 | 7.2993 | 14.0210 | -13.1062 | 0 |
| Stress | 5 | -4.1473 | 0.7417 | 7.7476 | 14.0108 | -20.7365 | 0 |

### Top-2: SL2.5/TP1.5

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -9.2151 | 0.3918 | 7.4405 | 9.8895 | -46.0753 | 0 |
| Stress | 5 | -10.7312 | 0.3317 | 7.8974 | 9.8855 | -53.6561 | 0 |

### Top-3: SL2.5/TP4.5

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 5 | -10.0215 | 0.4317 | 8.8166 | 12.2840 | -50.1074 | 0 |
| Stress | 5 | -11.5364 | 0.3875 | 9.4181 | 12.2738 | -57.6821 | 0 |

### ATR-off baseline (month 12)

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |
|---|---:|---:|---:|---:|---:|---:|---:|
| Base | 4 | -23.5043 | 0 | 9.4017 | 3.7259 | -94.0173 | 63.4951 |
| Stress | 4 | -25.0097 | 0 | 10.0039 | 3.7330 | -100.0389 | 61.8974 |

## 6. Conclusion

top-1 SL2.5/TP3.0: month-12 base net expectancy -2.6212 vs ATR-off -23.5043, PF 0.8270 vs 0, closed net PnL -13.1062 + open mark 0 = total net PnL -13.1062 vs ATR-off total -30.5222; stress net expectancy -4.1473 vs ATR-off -25.0097.
top-2 SL2.5/TP1.5: month-12 base net expectancy -9.2151 vs ATR-off -23.5043, PF 0.3918 vs 0, closed net PnL -46.0753 + open mark 0 = total net PnL -46.0753 vs ATR-off total -30.5222; stress net expectancy -10.7312 vs ATR-off -25.0097.
top-3 SL2.5/TP4.5: month-12 base net expectancy -10.0215 vs ATR-off -23.5043, PF 0.4317 vs 0, closed net PnL -50.1074 + open mark 0 = total net PnL -50.1074 vs ATR-off total -30.5222; stress net expectancy -11.5364 vs ATR-off -25.0097.

On month-12 base costs, 3 of 3 top-3 ATR configs beat ATR-off on closed-trade net expectancy, 3 of 3 beat it on PF, 3 beat it on both, and 1 beat it on total net PnL including open marks. Did any top-3 ATR config beat ATR-off on month 12? By net expectancy: Yes. By total net PnL: Yes.

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
