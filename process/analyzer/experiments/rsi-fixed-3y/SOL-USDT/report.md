# Fixed RSI 1day Backtest — SOL-USDT (3 years, no tuning)

Three equal candle-count thirds of the full period. Each year is a self-contained backtest with 100 warm-up candles; positions open at a year boundary are marked and dropped, so yearly trades do not sum to the full period.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 1day |
| Date range | 2023-09-24T00:00:00Z .. 2026-09-21T00:00:00Z |
| Candles | 1094 |
| SHA-256 | 302cf9212a8e163ae14fafab14a8173939c7224145134e6222b69fd856985a88 |

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
| Base | 6 | -93.3321 | 0.2196 | 61.9890 | 50.6968 | -559.9928 | 0 | 16.6667 |
| Stress | 6 | -94.9203 | 0.2148 | 62.7562 | 50.6751 | -569.5216 | 0 | 16.6667 |

## 4. Yearly breakdown

Three equal candle-count thirds of the full period.

| Year | Dates | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Base | 5 | -82.6903 | 0.2760 | 56.0178 | 60.7073 | -413.4517 | -77.4194 | 20.00 |
| Year 1 | 2023-09-24T00:00:00Z .. 2024-09-21T00:00:00Z | Stress | 5 | -84.2526 | 0.2700 | 56.7671 | 60.6734 | -421.2632 | -79.0331 | 20.00 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Base | 6 | -90.4170 | 0.1228 | 56.0328 | 44.2069 | -542.5022 | 0 | 33.3333 |
| Year 2 | 2024-09-22T00:00:00Z .. 2025-09-21T00:00:00Z | Stress | 6 | -91.9467 | 0.1167 | 56.8500 | 44.2347 | -551.6802 | 0 | 33.3333 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Base | 3 | 19.0133 | 1.3175 | 14.5281 | 109.1065 | 57.0399 | 194.2832 | 33.3333 |
| Year 3 | 2025-09-22T00:00:00Z .. 2026-09-21T00:00:00Z | Stress | 3 | 17.6453 | 1.2899 | 14.7812 | 109.2134 | 52.9360 | 192.4891 | 33.3333 |

## 5. Long vs short

**Full period, base costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 3 | -103.3227 | 0 | -34.4409 | 0.6040 | 33.3333 |
| Short | 3 | -456.6702 | 0 | -152.2234 | 0 | 0 |

**Full period, stress costs**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 3 | -107.6742 | 0 | -35.8914 | 0.5914 | 33.3333 |
| Short | 3 | -461.8474 | 0 | -153.9491 | 0 | 0 |

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

