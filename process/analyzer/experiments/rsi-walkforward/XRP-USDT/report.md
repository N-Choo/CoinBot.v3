# RSI Walk-Forward — XRP-USDT (4hour, 15 months)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | XRP-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 759c89f2eb5d0e60e4e9d20f48ed5cf5a1f82e3230d9046582cee830444df10c |

| Train window | Train candles | OOS window | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---:|---|---:|---|---|---|---|
| M1-M6 | 0..1094 | M7-M11 | 1094..2005 | 2025-07-24T00:00:00Z | 2026-01-22T04:00:00Z | 2026-01-22T08:00:00Z | 2026-06-23T00:00:00Z |
| M1-M7 | 0..1276 | M8-M12 | 1276..2188 | 2025-07-24T00:00:00Z | 2026-02-21T12:00:00Z | 2026-02-21T16:00:00Z | 2026-07-23T12:00:00Z |
| M1-M8 | 0..1458 | M9-M13 | 1458..2370 | 2025-07-24T00:00:00Z | 2026-03-23T20:00:00Z | 2026-03-24T00:00:00Z | 2026-08-22T20:00:00Z |
| M1-M9 | 0..1641 | M10-M14 | 1641..2552 | 2025-07-24T00:00:00Z | 2026-04-23T08:00:00Z | 2026-04-23T12:00:00Z | 2026-09-22T04:00:00Z |
| M1-M10 | 0..1823 | M11-M15 | 1823..2735 | 2025-07-24T00:00:00Z | 2026-05-23T16:00:00Z | 2026-05-23T20:00:00Z | 2026-10-22T16:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy version | rsi-mom-v1 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT, XRP-USDT, BNB-USDT (5) |
| RSI grid | period 14, 21 × long 55, 60, 65, 70 × short 40, 45, 50 (24 candidates) |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Initial fund | 1000 |
| Simulation window | 100 warm-up candles |

Execution assumptions:

- A signal produced at a candle close fills at the next candle open.
- One position per symbol, reversal-only exits.
- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks.
- Train selection and OOS success use base costs; stress costs are reported for context.

Training gates:

- At least 10 trades, net expectancy > 0, profit factor > 1, max drawdown <= 20%.
- Eligible candidates are ranked by train net expectancy; the best two are tested OOS.

OOS success gates:

- At least 10 trades, net expectancy > 0, profit factor > 1 (max drawdown is reported, not gated).

Decision policy:

- A symbol passes if at least one tested config in any window is successful; the config does not need to be the same across symbols.
- The overall experiment passes only if all 5 symbols pass.

## 3. Validation

Top two eligible candidates per expanding training window, base costs.

### M1-M6 training (M7-M11)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 12 | 12.4461 | 2.9044 | 3.4324 | 8.5675 | 10.9687 |
| 2 | baseline | 16 | 9.2691 | 2.2117 | 3.9887 | 7.5093 | 7.7869 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short50 | 10 | 22.0236 | 2.9632 | 4.8308 | 17.7993 | 20.5757 |
| 2 | rsi14-long70-short50 | 14 | 20.0101 | 3.8562 | 3.4324 | 12.6149 | 18.5477 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short50 | 12 | 16.8789 | 2.5596 | 6.1568 | 15.1032 | 15.4229 |
| 2 | rsi14-long70-short50 | 16 | 16.4318 | 3.2799 | 2.8382 | 11.2559 | 14.9651 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 18 | 14.4727 | 3.1700 | 3.0402 | 10.0627 | 13.0029 |
| 2 | rsi21-long65-short50 | 14 | 13.6492 | 2.3301 | 7.7937 | 13.0755 | 12.1886 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 22 | 8.0897 | 1.8785 | 9.3727 | 8.7285 | 6.6141 |
| 2 | rsi21-long65-short50 | 18 | 5.5020 | 1.4201 | 14.2345 | 10.7542 | 4.0316 |

## 4. Holdout

Top two candidates per window on the following five months. Success uses base costs.

| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| M1-M6 | rsi14-long70-short50 | Base | 12 | 2.4585 | 1.2087 | 11.0501 | 11.5280 | 29.5024 | 26.2558 | yes |
| M1-M6 | rsi14-long70-short50 | Stress | 12 | 0.9852 | 1.0764 | 12.2860 | 11.5419 | 11.8228 | 24.7974 | — |
| M1-M6 | baseline | Base | 22 | -5.8142 | 0.6104 | 25.3958 | 7.0430 | -127.9123 | 21.0962 | no |
| M1-M6 | baseline | Stress | 22 | -7.2996 | 0.5480 | 28.0045 | 7.0512 | -160.5913 | 19.6300 | — |
| M1-M7 | rsi21-long65-short50 | Base | 13 | -7.4593 | 0.4750 | 12.9069 | 7.0649 | -96.9710 | -19.5613 | no |
| M1-M7 | rsi21-long65-short50 | Stress | 13 | -8.9480 | 0.4175 | 14.1069 | 7.0730 | -116.3236 | -21.0342 | — |
| M1-M7 | rsi14-long70-short50 | Base | 13 | -3.5607 | 0.6675 | 11.0493 | 6.8787 | -46.2889 | -11.6717 | no |
| M1-M7 | rsi14-long70-short50 | Stress | 13 | -5.0483 | 0.5743 | 12.2493 | 6.8865 | -65.6282 | -13.1565 | — |
| M1-M8 | rsi21-long65-short50 | Base | 13 | -6.2641 | 0.5612 | 11.2611 | 7.3575 | -81.4337 | 183.0053 | no |
| M1-M8 | rsi21-long65-short50 | Stress | 13 | -7.7490 | 0.4977 | 12.1619 | 7.3654 | -100.7368 | 181.2282 | — |
| M1-M8 | rsi14-long70-short50 | Base | 13 | -1.7588 | 0.8270 | 9.4035 | 7.0269 | -22.8638 | 183.0053 | no |
| M1-M8 | rsi14-long70-short50 | Stress | 13 | -3.2435 | 0.7135 | 10.3043 | 7.0346 | -42.1653 | 181.2282 | — |
| M1-M9 | rsi14-long70-short50 | Base | 17 | -1.9707 | 0.8926 | 16.8009 | 13.0774 | -33.5021 | 9.5724 | no |
| M1-M9 | rsi14-long70-short50 | Stress | 17 | -3.4760 | 0.8217 | 17.7255 | 13.0684 | -59.0927 | 8.0558 | — |
| M1-M9 | rsi21-long65-short50 | Base | 17 | -7.4029 | 0.6608 | 19.9217 | 11.9391 | -125.8495 | 9.5724 | no |
| M1-M9 | rsi21-long65-short50 | Stress | 17 | -8.9080 | 0.6123 | 20.9615 | 11.9334 | -151.4353 | 8.0558 | — |
| M1-M10 | rsi14-long70-short50 | Base | 16 | 2.5595 | 1.1609 | 16.0018 | 13.5638 | 40.9523 | 6.3527 | yes |
| M1-M10 | rsi14-long70-short50 | Stress | 16 | 1.0570 | 1.0627 | 16.8037 | 13.5533 | 16.9120 | 4.8645 | — |
| M1-M10 | rsi21-long65-short50 | Base | 14 | -5.7937 | 0.7273 | 19.6330 | 13.7607 | -81.1113 | 54.3036 | no |
| M1-M10 | rsi21-long65-short50 | Stress | 14 | -7.3022 | 0.6728 | 20.6436 | 13.7513 | -102.2302 | 52.8872 | — |

The OOS interval for a window ending at month k is M(k+1)..M(k+5), using 5 months.

## 5. Robustness

**Successful-config matrix (window × tested config; base costs)**

| Window | rsi14-long70-short50 | baseline | rsi21-long65-short50 |
|---|---|---|---|
| M1-M6 | Pass | Fail | — |
| M1-M7 | Fail | — | Fail |
| M1-M8 | Fail | — | Fail |
| M1-M9 | Fail | — | Fail |
| M1-M10 | Pass | — | Fail |

**Long vs short (rsi14-long70-short50, OOS M11-M15, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 8 | 33.8633 | 0 | 4.2329 | 1.2480 | 25.00 |
| Short | 8 | 7.0890 | 6.3527 | 0.8861 | 1.0601 | 50.00 |

**BTC regime (winner OOS, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 12 | -8.2675 | 0 | -0.6890 | 0.9646 | 33.3333 |
| Bear | 4 | 49.2198 | 6.3527 | 12.3050 | 3.3759 | 50.00 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs for the winner (training window, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the winner. Training window M1-M10.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| -5.6323 | 0.2609 | 8.0897 | 4 |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M6: rsi14-long70-short50 passed OOS with 12 trades, net expectancy 2.4585302902164517584554459208, profit factor 1.2087280478725078266076334676.
- M1-M10: rsi14-long70-short50 passed OOS with 16 trades, net expectancy 2.5595209804067993243619133878, profit factor 1.1608762480612349529648200755.

Successful configs:

| Window | Config | OOS trades | OOS net expectancy | OOS PF |
|---|---|---:|---:|---:|
| M1-M6 | rsi14-long70-short50 | 12 | 2.4585 | 1.2087 |
| M1-M10 | rsi14-long70-short50 | 16 | 2.5595 | 1.1609 |

