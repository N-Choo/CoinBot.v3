# Fixed RSI 15min Backtest — BNB-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 15min |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T23:30:00Z |
| Candles | 105119 |
| SHA-256 | c1ff677e816febdc3ec82dc9b0cca8f90258ac8a96830b73184872d0f6b92fbc |

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
| Base | 1903 | -1.5296 | 0.6394 | 293.9022 | 0.2328 | -2910.8914 | -2.5385 | 27.1151 |
| Stress | 1903 | -3.0303 | 0.4358 | 577.0723 | 0.2328 | -5766.6865 | -4.0369 | 22.5434 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 1903 | 1903 | -1.5296 | -3.0303 | 0.6394 | 0.4358 | 293.9022 | 577.0723 | 0.2328 | 0.2328 | -2910.8914 | -5766.6865 | -2.5385 | -4.0369 | 27.1151 | 22.5434 |
| ATR14 1.5x/2.0x | 1904 | 1904 | -1.5408 | -3.0407 | 0.3463 | 0.1087 | 293.5967 | 578.9498 | 0.0780 | 0.0780 | -2933.6962 | -5789.4980 | 0 | 0 | 39.0231 | 20.6408 |
| ATR14 2.0x/3.0x | 1903 | 1903 | -1.4795 | -2.9795 | 0.4888 | 0.2271 | 282.9327 | 567.6979 | 0.1081 | 0.1081 | -2815.5611 | -5669.9260 | -2.5385 | -4.0369 | 39.4640 | 30.7409 |
| ATR14 2.5x/4.0x | 1903 | 1903 | -1.4549 | -2.9551 | 0.5630 | 0.3108 | 277.8843 | 562.9360 | 0.1307 | 0.1307 | -2768.7336 | -5623.5242 | -2.5385 | -4.0369 | 37.3095 | 34.1040 |
| ATR14 2.0x/3.0x high-vol x1.5 | 1903 | 1903 | -1.4795 | -2.9795 | 0.4888 | 0.2271 | 282.9327 | 567.6979 | 0.1081 | 0.1081 | -2815.5611 | -5669.9260 | -2.5385 | -4.0369 | 39.4640 | 30.7409 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Base | 631 | -1.7763 | 0.6194 | 112.7459 | 0.4322 | -1120.8228 | 1.9490 | 26.3074 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Stress | 631 | -3.2777 | 0.4352 | 207.1827 | 0.4321 | -2068.2080 | 0.4438 | 22.6624 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Base | 633 | -1.4770 | 0.6312 | 97.6101 | 0.3610 | -934.9282 | 26.1449 | 28.7520 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Stress | 633 | -2.9778 | 0.4181 | 191.2823 | 0.3609 | -1884.9336 | 24.6863 | 23.2227 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Base | 639 | -1.4031 | 0.6552 | 93.1793 | 0.4109 | -896.5874 | -2.5385 | 26.1346 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Stress | 639 | -2.9030 | 0.4427 | 180.5209 | 0.4109 | -1855.0272 | -4.0369 | 21.5962 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 951 | -1022.9530 | -2.5385 | -1.0757 | 0.7331 | 30.2839 |
| Short | 952 | -1887.9384 | 0 | -1.9831 | 0.5548 | 23.9496 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 951 | -2450.0592 | -4.0369 | -2.5763 | 0.4965 | 24.5005 |
| Short | 952 | -3316.6272 | 0 | -3.4839 | 0.3807 | 20.5882 |

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

