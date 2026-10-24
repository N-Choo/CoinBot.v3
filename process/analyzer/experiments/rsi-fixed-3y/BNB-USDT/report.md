# Fixed RSI 1day Backtest — BNB-USDT (3 years, no tuning)

Three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 1day |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-21T00:00:00Z |
| Candles | 1094 |
| SHA-256 | cab19ea4513fe63addaa6fdd01a0a4e178ccf1a6dfe5ff611a347714c5599af1 |

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
| Base | 12 | 16.6308 | 1.4788 | 14.2115 | 36.5116 | 199.5700 | 146.5790 | 33.3333 |
| Stress | 12 | 15.0694 | 1.4217 | 14.8121 | 36.4691 | 180.8331 | 144.8565 | 33.3333 |

## 4. Yearly breakdown

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Base | 1 | 337.3715 | ∞ | 5.6185 | 0 | 337.3715 | -75.1407 | 100 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Stress | 1 | 335.3624 | ∞ | 5.7476 | 0 | 335.3624 | -76.7510 | 100 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Base | 4 | -30.4153 | 0.1201 | 13.6010 | 16.2716 | -121.6614 | 262.2326 | 25.00 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Stress | 4 | -31.9439 | 0.1056 | 14.0733 | 16.2777 | -127.7754 | 260.3364 | 25.00 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Base | 6 | -29.2772 | 0.2968 | 17.5663 | 23.6291 | -175.6634 | 146.5790 | 16.6667 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Stress | 6 | -30.7190 | 0.2830 | 18.4314 | 23.6282 | -184.3143 | 144.8565 | 16.6667 |

## 5. Long vs short

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 6 | 345.2724 | 146.5790 | 57.5454 | 2.7529 | 50.00 |
| Short | 6 | -145.7024 | 0 | -24.2837 | 0.3372 | 16.6667 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 6 | 335.7402 | 144.8565 | 55.9567 | 2.6689 | 50.00 |
| Short | 6 | -154.9072 | 0 | -25.8179 | 0.3195 | 16.6667 |

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

