# Fixed RSI 4hour Backtest — SOL-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 6569 |
| SHA-256 | 27dfd8b12d5ced2b2d3f4413385394541fe4f2c667f212fc41e824d1187bcc77 |

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
| Base | 109 | 9.4465 | 1.4528 | 25.8269 | 8.7681 | 1029.6717 | 63.4951 | 36.6972 |
| Stress | 109 | 7.9265 | 1.3633 | 29.6809 | 8.7596 | 863.9936 | 61.8974 | 35.7798 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 109 | 109 | 9.4465 | 7.9265 | 1.4528 | 1.3633 | 25.8269 | 29.6809 | 8.7681 | 8.7596 | 1029.6717 | 863.9936 | 63.4951 | 61.8974 | 36.6972 | 35.7798 |
| ATR14 1.5x/2.0x | 110 | 110 | -2.7795 | -4.2839 | 0.7657 | 0.6640 | 39.1852 | 51.0406 | 2.0778 | 2.0779 | -305.7442 | -471.2262 | 0 | 0 | 40.9091 | 40.9091 |
| ATR14 2.0x/3.0x | 110 | 110 | 1.5902 | 0.0885 | 1.1129 | 1.0059 | 26.9786 | 34.6884 | 3.0226 | 3.0230 | 174.9207 | 9.7341 | 0 | 0 | 44.5455 | 44.5455 |
| ATR14 2.5x/4.0x | 110 | 110 | 1.0966 | -0.4049 | 1.0648 | 0.9772 | 27.4110 | 34.2689 | 3.5740 | 3.5742 | 120.6222 | -44.5444 | 0 | 0 | 42.7273 | 42.7273 |
| ATR14 2.0x/3.0x high-vol x1.5 | 110 | 110 | 0.9379 | -0.5648 | 1.0666 | 0.9621 | 28.2662 | 36.4210 | 2.9240 | 2.9242 | 103.1710 | -62.1231 | 0 | 0 | 44.5455 | 44.5455 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Base | 29 | 39.6391 | 3.3502 | 4.9470 | 27.4749 | 1149.5353 | 22.0782 | 41.3793 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Stress | 29 | 38.0709 | 3.1446 | 5.4282 | 27.4374 | 1104.0547 | 20.5428 | 41.3793 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Base | 37 | 2.6984 | 1.1248 | 26.2829 | 9.5938 | 99.8418 | 32.2891 | 37.8378 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Stress | 37 | 1.1841 | 1.0525 | 29.2484 | 9.5910 | 43.8117 | 30.8397 | 37.8378 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Base | 43 | -7.7051 | 0.6816 | 48.5281 | 8.5319 | -331.3211 | 63.4951 | 27.9070 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Stress | 43 | -9.1989 | 0.6364 | 52.1794 | 8.5356 | -395.5524 | 61.8974 | 25.5814 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 54 | 1240.7087 | 63.4951 | 22.9761 | 2.1836 | 38.8889 |
| Short | 55 | -211.0370 | 0 | -3.8370 | 0.8278 | 34.5455 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 54 | 1157.7231 | 61.8974 | 21.4393 | 2.0560 | 38.8889 |
| Short | 55 | -293.7295 | 0 | -5.3405 | 0.7709 | 32.7273 |

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

