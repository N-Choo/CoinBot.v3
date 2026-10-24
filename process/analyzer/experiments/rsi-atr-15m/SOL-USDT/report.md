# Fixed RSI 15min Backtest — SOL-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 15min |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T23:30:00Z |
| Candles | 105119 |
| SHA-256 | af8ea3d6dbfcec903ea31ce2c961af8cc582cf61c38219c84058e72a89e8f299 |

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
| Base | 1860 | -1.1394 | 0.8143 | 217.8674 | 0.3788 | -2119.2113 | -2.8882 | 31.1290 |
| Stress | 1860 | -2.6405 | 0.6331 | 481.9946 | 0.3787 | -4911.2894 | -4.3861 | 27.9032 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 1860 | 1860 | -1.1394 | -2.6405 | 0.8143 | 0.6331 | 217.8674 | 481.9946 | 0.3788 | 0.3787 | -2119.2113 | -4911.2894 | -2.8882 | -4.3861 | 31.1290 | 27.9032 |
| ATR14 1.5x/2.0x | 1861 | 1861 | -1.5290 | -3.0288 | 0.5502 | 0.2940 | 283.7593 | 563.6543 | 0.1329 | 0.1329 | -2845.3818 | -5636.5433 | 0 | 0 | 42.2891 | 38.0978 |
| ATR14 2.0x/3.0x | 1860 | 1860 | -1.4266 | -2.9264 | 0.6699 | 0.4416 | 265.4705 | 544.7501 | 0.1857 | 0.1857 | -2653.4568 | -5443.1148 | -2.8882 | -4.3861 | 39.0323 | 37.9570 |
| ATR14 2.5x/4.0x | 1860 | 1860 | -1.4280 | -2.9277 | 0.7172 | 0.5127 | 265.4618 | 544.9896 | 0.2283 | 0.2283 | -2656.1215 | -5445.5097 | -2.8882 | -4.3861 | 36.2903 | 35.8602 |
| ATR14 2.0x/3.0x high-vol x1.5 | 1860 | 1860 | -1.4266 | -2.9264 | 0.6699 | 0.4416 | 265.4705 | 544.7501 | 0.1857 | 0.1857 | -2653.4568 | -5443.1148 | -2.8882 | -4.3861 | 39.0323 | 37.9570 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Base | 599 | -1.0965 | 0.8502 | 72.4365 | 0.7914 | -656.8102 | 6.6353 | 32.2204 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Stress | 599 | -2.5996 | 0.6892 | 156.1975 | 0.7910 | -1557.1432 | 5.1475 | 28.8815 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Base | 629 | -1.5218 | 0.7552 | 98.0881 | 0.6151 | -957.2152 | 37.3419 | 31.3196 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Stress | 629 | -3.0227 | 0.5847 | 192.0465 | 0.6150 | -1901.3000 | 35.9001 | 27.6630 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Base | 631 | -0.8689 | 0.8245 | 70.3622 | 0.5508 | -548.2788 | -2.8882 | 29.7940 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Stress | 631 | -2.3685 | 0.6070 | 158.3462 | 0.5509 | -1494.5059 | -4.3861 | 27.0998 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 930 | -367.4120 | -2.8882 | -0.3951 | 0.9342 | 30.8602 |
| Short | 930 | -1751.7993 | 0 | -1.8837 | 0.6997 | 31.3978 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 930 | -1763.9557 | -4.3861 | -1.8967 | 0.7311 | 28.0645 |
| Short | 930 | -3147.3337 | 0 | -3.3842 | 0.5388 | 27.7419 |

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

