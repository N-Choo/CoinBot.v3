# Fixed RSI 4hour Backtest — XRP-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 6569 |
| SHA-256 | 8ef6f0146ed1ba135117c806564a256dffbccd9ebda9d9ea5468b59e2a6cf5f6 |

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
| Base | 32 | -17.4973 | 0.2142 | 57.8175 | 5.7462 | -559.9150 | 0 | 18.7500 |
| Stress | 32 | -19.0007 | 0.1910 | 62.2591 | 5.7444 | -608.0219 | 0 | 18.7500 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 32 | 32 | -17.4973 | -19.0007 | 0.2142 | 0.1910 | 57.8175 | 62.2591 | 5.7462 | 5.7444 | -559.9150 | -608.0219 | 0 | 0 | 18.7500 | 18.7500 |
| ATR14 1.5x/2.0x | 123 | 123 | 1.2587 | -0.2378 | 1.1454 | 0.9747 | 15.7933 | 24.3097 | 1.8371 | 1.8377 | 154.8216 | -29.2538 | 0 | 0 | 49.5935 | 49.5935 |
| ATR14 2.0x/3.0x | 123 | 123 | 1.6521 | 0.1561 | 1.1433 | 1.0126 | 16.6958 | 22.4356 | 2.4505 | 2.4511 | 203.2085 | 19.2041 | 0 | 0 | 44.7154 | 44.7154 |
| ATR14 2.5x/4.0x | 123 | 123 | -1.8885 | -3.3816 | 0.8741 | 0.7883 | 33.9980 | 46.1741 | 2.8433 | 2.8441 | -232.2892 | -415.9417 | 0 | 0 | 34.9593 | 34.9593 |
| ATR14 2.0x/3.0x high-vol x1.5 | 123 | 123 | 1.3395 | -0.1569 | 1.1131 | 0.9876 | 24.3893 | 23.1368 | 2.5200 | 2.5208 | 164.7583 | -19.3037 | 0 | 0 | 44.7154 | 44.7154 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Base | 32 | -17.4973 | 0.2142 | 57.8175 | 5.7462 | -559.9150 | 0 | 18.7500 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Stress | 32 | -19.0007 | 0.1910 | 62.2591 | 5.7444 | -608.0219 | 0 | 18.7500 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Base | 39 | 34.2718 | 2.6525 | 18.3765 | 41.2550 | 1336.5999 | 22.1367 | 30.7692 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Stress | 39 | 32.7044 | 2.5018 | 19.7823 | 41.1946 | 1275.4732 | 20.6721 | 30.7692 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Base | 41 | -6.6085 | 0.6546 | 28.8914 | 6.6476 | -270.9468 | 61.0341 | 26.8293 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Stress | 41 | -8.0986 | 0.5996 | 34.5681 | 6.6489 | -332.0433 | 59.4401 | 26.8293 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 16 | -243.9456 | 0 | -15.2466 | 0.3403 | 25.00 |
| Short | 16 | -315.9694 | 0 | -19.7481 | 0.0782 | 12.500 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 16 | -267.6152 | 0 | -16.7259 | 0.3089 | 25.00 |
| Short | 16 | -340.4067 | 0 | -21.2754 | 0.0655 | 12.500 |

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

