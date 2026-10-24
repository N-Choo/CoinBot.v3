# Fixed RSI 15min Backtest — XRP-USDT (1095 days, no tuning)

ATR-off yearly results are split into three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period. ATR variants are full-period only.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 15min |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-22T23:30:00Z |
| Candles | 105119 |
| SHA-256 | 773f3547f4242475fbc31f49070dc0c4a3f71b5066e6e52b6c98398fc051d1d5 |

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
| Base | 322 | -3.0599 | 0.4406 | 98.5286 | 0.5832 | -985.2798 | 0 | 25.4658 |
| Stress | 322 | -4.5606 | 0.3111 | 146.8502 | 0.5830 | -1468.5018 | 0 | 20.8075 |

## 4. ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR stop-loss and take-profit are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched. One position per symbol; opposite signals reverse at the fill open. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap.

B = base costs, S = stress costs. ATR is computed at the signal candle and the levels are built from the signal close.

| Variant | Trades B | Trades S | Net expectancy B | Net expectancy S | PF B | PF S | Max DD % B | Max DD % S | Stderr B | Stderr S | Closed net PnL B | Closed net PnL S | Open mark B | Open mark S | Win rate % B | Win rate % S |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ATR off | 322 | 322 | -3.0599 | -4.5606 | 0.4406 | 0.3111 | 98.5286 | 146.8502 | 0.5832 | 0.5830 | -985.2798 | -1468.5018 | 0 | 0 | 25.4658 | 20.8075 |
| ATR14 1.5x/2.0x | 1838 | 1838 | -1.6634 | -3.1633 | 0.4558 | 0.2112 | 306.1958 | 581.4204 | 0.1188 | 0.1188 | -3057.3898 | -5814.2045 | 0 | 0 | 40.9684 | 30.1959 |
| ATR14 2.0x/3.0x | 1838 | 1838 | -1.5010 | -3.0007 | 0.6014 | 0.3599 | 274.4643 | 549.5843 | 0.1646 | 0.1646 | -2758.7610 | -5515.2743 | 0 | 0 | 39.7171 | 36.2894 |
| ATR14 2.5x/4.0x | 1838 | 1838 | -1.6133 | -3.1128 | 0.6319 | 0.4182 | 299.7576 | 571.6796 | 0.1980 | 0.1980 | -2965.2567 | -5721.2501 | 0 | 0 | 36.0174 | 34.3308 |
| ATR14 2.0x/3.0x high-vol x1.5 | 1838 | 1838 | -1.5010 | -3.0007 | 0.6014 | 0.3599 | 274.4643 | 549.5843 | 0.1646 | 0.1646 | -2758.7610 | -5515.2743 | 0 | 0 | 39.7171 | 36.2894 |

## 5. Yearly breakdown (ATR off)

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Base | 322 | -3.0599 | 0.4406 | 98.5286 | 0.5832 | -985.2798 | 0 | 25.4658 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-22T23:30:00Z | Stress | 322 | -4.5606 | 0.3111 | 146.8502 | 0.5830 | -1468.5018 | 0 | 20.8075 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Base | 615 | -1.8639 | 0.7282 | 116.3983 | 0.8330 | -1146.3099 | -4.6198 | 25.8537 |
| Year 2 | 2024-09-22T23:45:00Z .. 2025-09-22T23:30:00Z | Stress | 615 | -3.3665 | 0.5787 | 208.7636 | 0.8325 | -2070.3686 | -6.1151 | 22.7642 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Base | 604 | -0.4769 | 0.8941 | 55.0689 | 0.6187 | -288.0622 | 7.3023 | 29.6358 |
| Year 3 | 2025-09-22T23:45:00Z .. 2026-09-22T23:30:00Z | Stress | 604 | -1.9765 | 0.6469 | 136.9290 | 0.6184 | -1193.7931 | 5.7890 | 24.3377 |

## 6. Long vs short (ATR off)

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 161 | -418.2621 | 0 | -2.5979 | 0.5245 | 29.1925 |
| Short | 161 | -567.0177 | 0 | -3.5218 | 0.3569 | 21.7391 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 161 | -659.4965 | 0 | -4.0963 | 0.3749 | 24.8447 |
| Short | 161 | -809.0053 | 0 | -5.0249 | 0.2487 | 16.7702 |

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

