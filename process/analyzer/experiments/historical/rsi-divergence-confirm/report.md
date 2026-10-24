# Analyzer Fresh Confirmation

Pre-registered one-shot confirmation. No tuning is permitted after this run.

## 1. Freeze

| Field | Value |
|---|---|
| Generated at | 2026-10-22T00:00:00Z |
| Lock path | ./freeze/confirmation-lock.json |
| Repository root | . |
| Frozen commit | 91e79e1 |
| Frozen-file SHA-256 (verified) | d3699b82fe34ec6277f877317637c0f81baf0194ee397fb7e3dd4d79c99bc80f |
| Lock SHA-256 (verified) | b4c6e24d0e0c298b3c713d93e937af5dbc869bc6dc964af0611ef8083c849063 |
| Lock created at | 2026-10-22T00:00:00Z |

## 2. Dataset

| Field | Value |
|---|---|
| Requested range | 2022-11-02T00:00:00Z (1667347200000) → 2024-10-22T00:00:00Z (1729555200000) |
| Timeframe | 4hour |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT |
| Data dir | data/confirm |
| Base URL | https://api.kucoin.com |
| Settle (ms) | 5000 |
| Initial fund per symbol | 1000 (CLI-configurable; the decision is scale-invariant) |
| Warm-up window | 100 |

| Symbol | First open | Last open | Candles | Requested | Expected | Missing | Duplicates | Content SHA-256 | Cache |
|---|---|---|---|---|---|---|---|---|---|
| BTC-USDT | 2022-11-02T00:00:00Z | 2024-10-21T20:00:00Z | 4320 | 4321 | 4320 | 0 | 0 | 5ec7d58fc2f47fea001bc9bc6d44046d659758b06e999e9eeb7e1718e2127e8a | fetched |
| ETH-USDT | 2022-11-02T00:00:00Z | 2024-10-21T20:00:00Z | 4320 | 4321 | 4320 | 0 | 0 | 474941444f09c17e5b0738301f0b4ecbb5d73712dc876faa542c1f92d0cc7608 | fetched |
| SOL-USDT | 2022-11-02T00:00:00Z | 2024-10-21T20:00:00Z | 4320 | 4321 | 4320 | 0 | 0 | 5a53a8fa7d7aa54d6b04cf0e622c5113f1f1786b11b1b992532f1920b9e06e90 | fetched |

## 3. Frozen config

| Field | Value |
|---|---|
| RSI_PERIOD | 14 |
| RSI_DIVERGENCE_ORDER | 5 |
| ANALYZER_MAX_DIVERGENCE_AGE_CANDLES | 4 |
| SMA_ENABLED | true |
| SMA_PERIOD | 20 |
| ATR_PERIOD | 14 |
| ATR_SL_MULTIPLIER | 1.5 |
| ATR_TP_MULTIPLIER | 2.0 |
| PRICE_DIFF_THRESHOLD | 0.05 |
| ATR_PCT_HIGH | 5.0 |
| ATR_HIGH_ADJUST | 1.5 |
| ATR_SL_MAX_PCT | 0.0 |
| ANALYZER_WINDOW_CANDLES | 100 |

## 4. Execution model

| Field | Value |
|---|---|
| Label | retest_2c_b2 |
| Maker | 2 bps |
| Taker | 10 bps |
| Slippage | 5 bps |
| Entry | limit |
| Valid candles | 2 |
| Buffer | 2 bps |
| Offset | 25 bps |

## 5. Metrics

| Symbol or pooled | Trades | Net expectancy | SE | PF | Win rate | Max DD | Max DD % | Net PnL | Signals | Filled | Exec fill rate | Capital fill rate | Skipped funds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| BTC-USDT | 93 | -0.6633 | 1.4544 | 0.90 | 36.56% | 164.8219 | 16.48% | -61.6879 | 154 | 93 | 75.61% | 60.39% | 31 |
| ETH-USDT | 95 | -1.7367 | 1.6600 | 0.78 | 33.68% | 198.7220 | 19.87% | -164.9822 | 144 | 95 | 78.51% | 65.97% | 23 |
| SOL-USDT | 112 | -3.7253 | 2.9241 | 0.76 | 38.39% | 621.2667 | 62.13% | -417.2391 | 173 | 112 | 87.50% | 64.74% | 45 |
| pooled | 300 | -2.1464 | 1.2932 | 0.80 | 36.33% | 621.2667 | 62.13% | -643.9092 | 471 | 300 | 80.65% | 63.69% | 99 |

## 6. Decision

**confirmation failed**

Pre-registered reasons:

- pooled net expectancy -2.1464 was not positive
- pooled net profit factor 0.80 was not above 1
- only 0 of 3 symbols had positive net expectancy (minimum 2 positive symbols)

| Rule | Threshold | Observed |
|---|---|---|
| Pooled closed trades | ≥ 50 | 300 |
| Pooled net expectancy | > 0 | -2.1464 |
| Pooled net profit factor | > 1 | 0.80 |
| Positive symbols | ≥ 2 | 0 of 3 |

