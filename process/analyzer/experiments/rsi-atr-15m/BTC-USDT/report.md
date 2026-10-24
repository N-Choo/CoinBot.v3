# Fixed RSI 15min Backtest — BTC-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 15min |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T23:30:00Z |
| Candles | 105119 |
| SHA-256 | bfb338dfd6e970ef3f9af271bb404abf67c71cb6aa48bd367c0dc48cf7227d1e |

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
| Base | 1909 | -1.5735 | 0.6053 | 307.4849 | 0.2092 | -3003.7850 | -1.3934 | 25.2488 |
| Stress | 1909 | -3.0741 | 0.4026 | 591.5853 | 0.2091 | -5868.4126 | -2.8935 | 20.6915 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 1909 | 1909 | -1.5735 | -3.0741 | 0.6053 | 0.4026 | 307.4849 | 591.5853 | 0.2092 | 0.2091 | -3003.7850 | -5868.4126 | -1.3934 | -2.8935 | 25.2488 | 20.6915 |
| ATR14 1.5x/2.0x | 1910 | 1910 | -1.5533 | -3.0532 | 0.2947 | 0.0758 | 296.3045 | 583.1122 | 0.0676 | 0.0676 | -2966.7381 | -5831.5971 | 0 | 0 | 36.3351 | 17.4869 |
| ATR14 2.0x/3.0x | 1910 | 1910 | -1.5852 | -3.0851 | 0.4197 | 0.1767 | 301.9029 | 588.4515 | 0.0940 | 0.0940 | -3027.8273 | -5892.5631 | 0 | 0 | 36.2827 | 27.0157 |
| ATR14 2.5x/4.0x | 1910 | 1910 | -1.5741 | -3.0742 | 0.4923 | 0.2509 | 300.5187 | 587.1666 | 0.1147 | 0.1147 | -3006.6070 | -5871.6664 | 0 | 0 | 35.0785 | 30.2618 |
| ATR14 2.0x/3.0x high-vol x1.5 | 1910 | 1910 | -1.5852 | -3.0851 | 0.4197 | 0.1767 | 301.9029 | 588.4515 | 0.0940 | 0.0940 | -3027.8273 | -5892.5631 | 0 | 0 | 36.2827 | 27.0157 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Base | 653 | -1.6674 | 0.6097 | 108.8469 | 0.3854 | -1088.7942 | -0.9701 | 25.8806 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Stress | 653 | -3.1685 | 0.4152 | 207.1520 | 0.3853 | -2069.0491 | -2.4709 | 21.7458 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Base | 629 | -1.6743 | 0.5662 | 105.4911 | 0.3215 | -1053.1434 | 11.9023 | 24.9603 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Stress | 629 | -3.1751 | 0.3684 | 199.7152 | 0.3214 | -1997.1516 | 10.4224 | 19.8728 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Base | 627 | -1.4026 | 0.6337 | 98.1792 | 0.3749 | -879.4502 | -1.3934 | 24.7209 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Stress | 627 | -2.9024 | 0.4188 | 185.7922 | 0.3749 | -1819.8351 | -2.8935 | 20.2552 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 954 | -1125.1661 | -1.3934 | -1.1794 | 0.6968 | 26.3103 |
| Short | 955 | -1878.6189 | 0 | -1.9671 | 0.5184 | 24.1885 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 954 | -2556.6256 | -2.8935 | -2.6799 | 0.4673 | 21.5933 |
| Short | 955 | -3311.7871 | 0 | -3.4678 | 0.3407 | 19.7906 |

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

