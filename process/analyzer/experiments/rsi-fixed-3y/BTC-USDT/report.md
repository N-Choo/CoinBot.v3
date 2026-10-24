# Fixed RSI 1day Backtest — BTC-USDT (3 years, no tuning)

Three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 1day |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-21T00:00:00Z |
| Candles | 1094 |
| SHA-256 | 720492dc9788169088d8783c103b8d8e202f4076cfca6007765cf5783e5f80b4 |

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
| Base | 14 | -1.6809 | 0.9540 | 35.4158 | 23.6728 | -23.5325 | 122.9188 | 42.8571 |
| Stress | 14 | -3.2115 | 0.9141 | 36.1515 | 23.6545 | -44.9614 | 121.2319 | 42.8571 |

## 4. Yearly breakdown

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Base | 5 | -43.0155 | 0.3575 | 34.0149 | 41.4606 | -215.0774 | -46.0889 | 20.00 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Stress | 5 | -44.5597 | 0.3463 | 34.7464 | 41.4328 | -222.7986 | -47.6557 | 20.00 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Base | 2 | 94.3836 | 16.9123 | 0.9881 | 106.2466 | 188.7672 | 114.9276 | 50.00 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Stress | 2 | 92.7240 | 14.8616 | 1.1160 | 106.1025 | 185.4480 | 113.2527 | 50.00 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Base | 6 | -10.5414 | 0.6116 | 8.1423 | 20.4008 | -63.2486 | 122.9188 | 50.00 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Stress | 6 | -11.9758 | 0.5700 | 8.5674 | 20.3955 | -71.8549 | 121.2319 | 50.00 |

## 5. Long vs short

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 7 | 131.2022 | 122.9188 | 18.7432 | 1.5103 | 42.8571 |
| Short | 7 | -154.7347 | 0 | -22.1050 | 0.3916 | 42.8571 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 7 | 120.4894 | 121.2319 | 17.2128 | 1.4586 | 42.8571 |
| Short | 7 | -165.4508 | 0 | -23.6358 | 0.3653 | 42.8571 |

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

