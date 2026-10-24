# Fixed RSI 4hour Backtest — BTC-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T16:00:00Z |
| Candles | 6569 |
| SHA-256 | 77c2540cf7cb47a48807be219dc182596c5126054ec6285fa2972e20ce44a178 |

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
| Base | 121 | 1.8173 | 1.1505 | 20.1466 | 4.0426 | 219.8975 | 50.5383 | 36.3636 |
| Stress | 121 | 0.3083 | 1.0236 | 27.4313 | 4.0390 | 37.3052 | 48.9601 | 31.4050 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 121 | 121 | 1.8173 | 0.3083 | 1.1505 | 1.0236 | 20.1466 | 27.4313 | 4.0426 | 4.0390 | 219.8975 | 37.3052 | 50.5383 | 48.9601 | 36.3636 | 31.4050 |
| ATR14 1.5x/2.0x | 122 | 122 | -1.3329 | -2.8321 | 0.7973 | 0.6172 | 24.3010 | 38.4025 | 1.1255 | 1.1255 | -162.6084 | -345.5205 | 0 | 0 | 45.0820 | 45.0820 |
| ATR14 2.0x/3.0x | 122 | 122 | 0.1447 | -1.3551 | 1.0178 | 0.8486 | 14.1944 | 27.7697 | 1.5910 | 1.5911 | 17.6478 | -165.3210 | 0 | 0 | 45.0820 | 45.0820 |
| ATR14 2.5x/4.0x | 122 | 122 | 0.2662 | -1.2328 | 1.0273 | 0.8840 | 21.6856 | 29.2687 | 1.9706 | 1.9709 | 32.4807 | -150.4071 | 0 | 0 | 42.6230 | 41.8033 |
| ATR14 2.0x/3.0x high-vol x1.5 | 122 | 122 | 0.1447 | -1.3551 | 1.0178 | 0.8486 | 14.1944 | 27.7697 | 1.5910 | 1.5911 | 17.6478 | -165.3210 | 0 | 0 | 45.0820 | 45.0820 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Base | 35 | 11.1696 | 2.0448 | 10.6228 | 10.8385 | 390.9376 | 51.5268 | 45.7143 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T16:00:00Z | Stress | 35 | 9.6491 | 1.8373 | 11.3135 | 10.8244 | 337.7198 | 49.9471 | 40.00 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Base | 45 | -4.9267 | 0.6104 | 27.4049 | 4.3807 | -221.7009 | 9.5514 | 28.8889 |
| Year 2 | 2024-09-22T20:00:00Z .. 2025-09-22T16:00:00Z | Stress | 45 | -6.4375 | 0.5330 | 31.7461 | 4.3762 | -289.6888 | 8.0680 | 22.2222 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Base | 41 | -0.5972 | 0.9535 | 16.2841 | 5.5814 | -24.4855 | 50.5383 | 31.7073 |
| Year 3 | 2025-09-22T20:00:00Z .. 2026-09-22T16:00:00Z | Stress | 41 | -2.0926 | 0.8492 | 18.1089 | 5.5837 | -85.7971 | 48.9601 | 29.2683 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 60 | 474.4844 | 50.5383 | 7.9081 | 1.6712 | 36.6667 |
| Short | 61 | -254.5869 | 0 | -4.1736 | 0.6624 | 36.0656 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 60 | 383.6364 | 48.9601 | 6.3939 | 1.5017 | 33.3333 |
| Short | 61 | -346.3312 | 0 | -5.6776 | 0.5758 | 29.5082 |

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

