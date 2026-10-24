# Fixed RSI 4hour Backtest — BNB-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 6569 |
| SHA-256 | 08db2a3d76e027a657dbcf215cd6e1d186ef4330f64d4223b333c7b59871b8f6 |

## 2. Fixed strategy

| Field | Value |
|---|---|
| Strategy version | rsi-mom-v1 |
| RSI period | 16 |
| Long threshold | 60 |
| Short threshold | 40 |
| ATR / stop-loss / take-profit | off in the baseline; variants compared below |
| Tuning | none |
| Warm-up window | 100 candles |

## 3. Full-period performance (ATR off)

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | 111 | -0.6762 | 0.9542 | 29.0109 | 3.6894 | -75.0538 | 31.0665 | 36.9369 |
| Stress | 111 | -2.1870 | 0.8610 | 36.6759 | 3.6885 | -242.7580 | 29.5175 | 36.0360 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 111 | 111 | -0.6762 | -2.1870 | 0.9542 | 0.8610 | 29.0109 | 36.6759 | 3.6894 | 3.6885 | -75.0538 | -242.7580 | 31.0665 | 29.5175 | 36.9369 | 36.0360 |
| ATR14 1.5x/2.0x | 112 | 112 | -2.8203 | -4.3205 | 0.6634 | 0.5343 | 34.8182 | 49.3102 | 1.4356 | 1.4361 | -315.8690 | -483.9008 | 0 | 0 | 40.1786 | 40.1786 |
| ATR14 2.0x/3.0x | 112 | 112 | -4.8588 | -6.3607 | 0.5806 | 0.4937 | 55.9698 | 72.6386 | 1.8506 | 1.8510 | -544.1824 | -712.3959 | 0 | 0 | 34.8214 | 34.8214 |
| ATR14 2.5x/4.0x | 112 | 112 | -2.2708 | -3.7763 | 0.8163 | 0.7156 | 28.2383 | 44.4342 | 2.3623 | 2.3624 | -254.3350 | -422.9497 | 0 | 0 | 39.2857 | 39.2857 |
| ATR14 2.0x/3.0x high-vol x1.5 | 112 | 112 | -5.1129 | -6.6152 | 0.5681 | 0.4839 | 58.8164 | 75.4895 | 1.9325 | 1.9331 | -572.6482 | -740.9043 | 0 | 0 | 34.8214 | 34.8214 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Base | 37 | 1.0282 | 1.0648 | 17.0702 | 7.2400 | 38.0422 | 61.5422 | 35.1351 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Stress | 37 | -0.4933 | 0.9707 | 18.5254 | 7.2351 | -18.2522 | 59.9475 | 35.1351 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Base | 30 | 1.3587 | 1.1292 | 13.6755 | 5.3210 | 40.7613 | 64.6001 | 40.00 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Stress | 30 | -0.1513 | 0.9868 | 16.2301 | 5.3177 | -4.5391 | 63.0008 | 40.00 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Base | 43 | -6.6638 | 0.6249 | 35.0467 | 5.9947 | -286.5418 | 31.0665 | 32.5581 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Stress | 43 | -8.1616 | 0.5658 | 40.5915 | 5.9976 | -350.9497 | 29.5175 | 30.2326 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 55 | 364.5605 | 31.0665 | 6.6284 | 1.5346 | 47.2727 |
| Short | 56 | -439.6142 | 0 | -7.8503 | 0.5412 | 26.7857 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 55 | 281.3889 | 29.5175 | 5.1162 | 1.3879 | 45.4545 |
| Short | 56 | -524.1469 | 0 | -9.3598 | 0.4866 | 26.7857 |

## 7. Execution assumptions and costs

- A signal produced at a candle close fills at the next candle open.
- One position per symbol; opposite signals reverse at the fill open.
- ATR off exits only on opposite signals. ATR variants can also exit on stop-loss or take-profit, including on the entry candle.
- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks.
- Base costs: 10 bps fee + 5 bps slippage per side.
- Stress costs: 20 bps fee + 10 bps slippage per side.

## 7. Notes

- The requested window spans 1095 days ending at as_of = 1790121600000 ms.
- The configuration is fixed at RSI 16/60/40 and is not tuned on this data.
- This report is not a profitability claim.

