# RSI Walk-Forward — BTC-USDT (4hour, 15 months)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BTC-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 1ee09051cb265eafd9e32a673fbb283dc0c345e4bec0130ad508df29c2c6ecfb |

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
| 1 | rsi14-long70-short50 | 11 | 12.0045 | 4.1676 | 1.5693 | 9.5098 | 10.5199 |
| 2 | rsi21-long65-short50 | 11 | 9.7241 | 3.0143 | 2.0060 | 9.2345 | 8.2403 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 11 | 12.0045 | 4.1676 | 1.5693 | 9.5098 | 10.5199 |
| 2 | rsi21-long65-short50 | 11 | 9.7241 | 3.0143 | 2.0060 | 9.2345 | 8.2403 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 15 | 10.9248 | 2.3589 | 6.3488 | 10.8502 | 9.4474 |
| 2 | rsi21-long65-short50 | 15 | 8.3247 | 1.8785 | 7.3345 | 10.6358 | 6.8485 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short40 | 17 | 5.9815 | 1.5714 | 8.7032 | 9.6155 | 4.5010 |
| 2 | rsi14-long70-short50 | 21 | 5.7948 | 1.6493 | 11.1742 | 8.1314 | 4.3097 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short50 | 23 | 3.5863 | 1.3640 | 12.8976 | 7.6267 | 2.0978 |
| 2 | rsi14-long70-short45 | 21 | 2.6503 | 1.2393 | 12.8419 | 8.1645 | 1.1632 |

## 4. Holdout

Top two candidates per window on the following five months. Success uses base costs.

| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| M1-M6 | rsi14-long70-short50 | Base | 14 | -1.4848 | 0.8985 | 14.8154 | 10.2215 | -20.7866 | 4.2843 | no |
| M1-M6 | rsi14-long70-short50 | Stress | 14 | -2.9705 | 0.8109 | 16.3679 | 10.2343 | -41.5876 | 2.7929 | — |
| M1-M6 | rsi21-long65-short50 | Base | 14 | -3.5990 | 0.7782 | 17.7725 | 10.4048 | -50.3857 | 4.2843 | no |
| M1-M6 | rsi21-long65-short50 | Stress | 14 | -5.0842 | 0.7061 | 19.3212 | 10.4175 | -71.1789 | 2.7929 | — |
| M1-M7 | rsi14-long70-short50 | Base | 18 | -10.2895 | 0.3777 | 20.3988 | 6.6822 | -185.2116 | 2.2609 | no |
| M1-M7 | rsi14-long70-short50 | Stress | 18 | -11.7894 | 0.3343 | 22.2123 | 6.6908 | -212.2097 | 0.7666 | — |
| M1-M7 | rsi21-long65-short50 | Base | 15 | -13.1277 | 0.3133 | 23.6876 | 7.5290 | -196.9151 | -14.4494 | no |
| M1-M7 | rsi21-long65-short50 | Stress | 15 | -14.6284 | 0.2803 | 25.5008 | 7.5384 | -219.4255 | -15.9300 | — |
| M1-M8 | rsi14-long70-short50 | Base | 15 | -5.1096 | 0.5621 | 10.2814 | 6.9480 | -76.6436 | 97.4716 | no |
| M1-M8 | rsi14-long70-short50 | Stress | 15 | -6.6065 | 0.4819 | 11.4621 | 6.9575 | -99.0979 | 95.8229 | — |
| M1-M8 | rsi21-long65-short50 | Base | 13 | -6.9640 | 0.4717 | 11.7879 | 7.6558 | -90.5314 | 92.5466 | no |
| M1-M8 | rsi21-long65-short50 | Stress | 13 | -8.4608 | 0.4147 | 12.7444 | 7.6659 | -109.9907 | 90.9054 | — |
| M1-M9 | rsi14-long70-short40 | Base | 13 | -4.4247 | 0.7484 | 12.2932 | 11.5962 | -57.5214 | 27.0639 | no |
| M1-M9 | rsi14-long70-short40 | Stress | 13 | -5.9304 | 0.6855 | 13.2184 | 11.5930 | -77.0950 | 25.5210 | — |
| M1-M9 | rsi14-long70-short50 | Base | 13 | 4.9364 | 1.4652 | 5.5784 | 11.3572 | 64.1735 | 27.0639 | yes |
| M1-M9 | rsi14-long70-short50 | Stress | 13 | 3.4317 | 1.2945 | 6.3155 | 11.3539 | 44.6115 | 25.5210 | — |
| M1-M10 | rsi14-long70-short50 | Base | 14 | 3.7510 | 1.3600 | 5.4941 | 9.9325 | 52.5135 | 15.2787 | yes |
| M1-M10 | rsi14-long70-short50 | Stress | 14 | 2.2438 | 1.1971 | 6.3763 | 9.9266 | 31.4136 | 13.8038 | — |
| M1-M10 | rsi14-long70-short45 | Base | 14 | 0.4748 | 1.0379 | 7.3699 | 9.8486 | 6.6470 | 14.2537 | yes |
| M1-M10 | rsi14-long70-short45 | Stress | 14 | -1.0324 | 0.9235 | 7.9791 | 9.8435 | -14.4539 | 12.7773 | — |

The OOS interval for a window ending at month k is M(k+1)..M(k+5), using 5 months.

## 5. Robustness

**Successful-config matrix (window × tested config; base costs)**

| Window | rsi14-long70-short50 | rsi21-long65-short50 | rsi14-long70-short40 | rsi14-long70-short45 |
|---|---|---|---|---|
| M1-M6 | Fail | Fail | — | — |
| M1-M7 | Fail | Fail | — | — |
| M1-M8 | Fail | Fail | — | — |
| M1-M9 | Pass | — | Fail | — |
| M1-M10 | Pass | — | — | Pass |

**Long vs short (rsi14-long70-short50, OOS M10-M14, base costs)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Long | 6 | 53.4433 | 27.0639 | 8.9072 | 1.9930 | 16.6667 |
| Short | 7 | 10.7302 | 0 | 1.5329 | 1.1275 | 42.8571 |

**BTC regime (winner OOS, base costs; bull = BTC close > 200-period SMA at entry)**

| Side | Trades | Net PnL | Open mark | Net expectancy | PF | Win rate % |
|---|---:|---:|---:|---:|---:|---:|
| Bull | 11 | 73.8077 | 27.0639 | 6.7098 | 1.6252 | 27.2727 |
| Bear | 2 | -9.6342 | 0 | -4.8171 | 0.5159 | 50.00 |
| Unknown | 0 | 0 | 0 | 0 | — | 0 |

**Neighboring configs for the winner (training window, base costs)**

Same RSI period and long/short thresholds within ±5 points, including the winner. Training window M1-M9.

| Min net expectancy | Median | Max | Count |
|---:|---:|---:|---:|
| -4.2997 | 2.2894 | 5.7948 | 4 |

## 6. Decision

**Per-symbol verdict: Pass**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M9: rsi14-long70-short50 passed OOS with 13 trades, net expectancy 4.9364207180565492243303490038, profit factor 1.465187321101133154371296793.
- M1-M10: rsi14-long70-short50 passed OOS with 14 trades, net expectancy 3.7509650019768152006653004699, profit factor 1.3600214024388584718937210433.
- M1-M10: rsi14-long70-short45 passed OOS with 14 trades, net expectancy 0.4747863768926996393150233734, profit factor 1.0379161705462027652631380959.

Successful configs:

| Window | Config | OOS trades | OOS net expectancy | OOS PF |
|---|---|---:|---:|---:|
| M1-M9 | rsi14-long70-short50 | 13 | 4.9364 | 1.4652 |
| M1-M10 | rsi14-long70-short50 | 14 | 3.7510 | 1.3600 |
| M1-M10 | rsi14-long70-short45 | 14 | 0.4748 | 1.0379 |

