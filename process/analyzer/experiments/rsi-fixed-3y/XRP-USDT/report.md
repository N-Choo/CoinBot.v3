# Fixed RSI 1day Backtest — XRP-USDT (3 years, no tuning)

Three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 1day |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-21T00:00:00Z |
| Candles | 1094 |
| SHA-256 | 0cf8b22719d62aad4406ebbccd6e58f3e2a1bbf8d995c292db9486bc730eeaca |

## 2. Fixed strategy

| Field | Value |
|---|---|
| Strategy version | rsi-mom-v1 |
| RSI period | 21 |
| Long threshold | 60 |
| Short threshold | 40 |
| ATR / stop-loss / take-profit | off (current code has no ATR/SL/TP) |
| Tuning | none |
| Warm-up window | 100 candles |

## 3. Full-period performance

| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Base | 13 | 60.5882 | 2.1050 | 19.1343 | 110.2200 | 787.6468 | 103.7625 | 23.0769 |
| Stress | 13 | 58.9499 | 2.0530 | 19.6755 | 110.0640 | 766.3484 | 102.1043 | 23.0769 |

## 4. Yearly breakdown

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Base | 3 | -65.5582 | 0 | 19.6675 | 20.9747 | -196.6746 | 54.7953 | 0 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Stress | 3 | -67.0524 | 0 | 20.1157 | 20.9147 | -201.1572 | 53.2107 | 0 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Base | 6 | 152.1533 | 3.0228 | 11.7278 | 242.4963 | 912.9201 | 116.1413 | 16.6667 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Stress | 6 | 150.3005 | 2.9652 | 11.9760 | 242.1559 | 901.8029 | 114.4645 | 16.6667 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Base | 3 | 1.5011 | 1.0342 | 13.1322 | 76.4686 | 4.5032 | 103.7625 | 66.6667 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Stress | 3 | 0.1358 | 1.0031 | 13.2820 | 76.4697 | 0.4073 | 102.1043 | 66.6667 |

## 5. Long vs short

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 6 | 993.4244 | 103.7625 | 165.5707 | 3.6790 | 16.6667 |
| Short | 7 | -205.7776 | 0 | -29.3968 | 0.3983 | 28.5714 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 6 | 982.9185 | 102.1043 | 163.8197 | 3.6019 | 16.6667 |
| Short | 7 | -216.5700 | 0 | -30.9386 | 0.3812 | 28.5714 |

## 6. Execution assumptions and costs

- A signal produced at a candle close fills at the next candle open.
- One position per symbol, reversal-only exits.
- Exits occur only on opposite signals; there is no ATR, stop-loss, or take-profit.
- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks.
- Base costs: 10 bps fee + 5 bps slippage per side.
- Stress costs: 20 bps fee + 10 bps slippage per side.

## 7. Notes

- The 3-year window ends one month before the generated-at date (as_of = 1790121600000 ms).
- The configuration is fixed at RSI 21/60/40 and is not tuned on this data.
- This report is not a profitability claim.

