# RSI Walk-Forward — ETH-USDT (4hour, 15 months)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | ETH-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 1357ea92f4d97764eb54cac30a1bce587e60400c3848332650ddce336ecbb0ed |

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
| 1 | rsi14-long65-short50 | 23 | 4.5356 | 1.5560 | 8.8975 | 5.4383 | 3.0436 |
| 2 | rsi14-long60-short50 | 35 | 2.8755 | 1.3592 | 8.3258 | 4.2173 | 1.3802 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long60-short50 | 39 | 6.2973 | 1.7765 | 8.3258 | 5.3573 | 4.8099 |
| 2 | rsi14-long65-short50 | 23 | 4.5356 | 1.5560 | 8.8975 | 5.4383 | 3.0436 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long70-short50 | 11 | 16.7332 | 2.4055 | 6.6325 | 20.3001 | 15.2702 |
| 2 | rsi21-long70-short45 | 11 | 9.9494 | 1.5830 | 9.1783 | 20.7002 | 8.4888 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long70-short50 | 11 | 16.7332 | 2.4055 | 6.6325 | 20.3001 | 15.2702 |
| 2 | rsi21-long70-short45 | 11 | 9.9494 | 1.5830 | 9.1783 | 20.7002 | 8.4888 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long70-short50 | 11 | 16.7332 | 2.4055 | 6.6325 | 20.3001 | 15.2702 |
| 2 | rsi21-long70-short45 | 11 | 9.9494 | 1.5830 | 9.1783 | 20.7002 | 8.4888 |

## 4. Holdout

Top two candidates per window on the following five months. Success uses base costs.

| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| M1-M6 | rsi14-long65-short50 | Base | 22 | -0.8132 | 0.9460 | 26.4346 | 10.4328 | -17.8897 | 0.0094 | no |
| M1-M6 | rsi14-long65-short50 | Stress | 22 | -2.2994 | 0.8584 | 28.9449 | 10.4471 | -50.5877 | -1.4883 | — |
| M1-M6 | rsi14-long60-short50 | Base | 38 | -3.0995 | 0.7321 | 29.8468 | 5.6385 | -117.7798 | -2.0796 | no |
| M1-M6 | rsi14-long60-short50 | Stress | 38 | -4.5911 | 0.6414 | 34.1697 | 5.6459 | -174.4636 | -3.5805 | — |
| M1-M7 | rsi14-long60-short50 | Base | 41 | -5.6685 | 0.5030 | 32.4013 | 3.9329 | -232.4105 | 2.6782 | no |
| M1-M7 | rsi14-long60-short50 | Stress | 41 | -7.1689 | 0.4322 | 36.9149 | 3.9372 | -293.9252 | 1.1719 | — |
| M1-M7 | rsi14-long65-short50 | Base | 29 | -8.7390 | 0.4187 | 32.2078 | 5.7083 | -253.4324 | -9.6415 | no |
| M1-M7 | rsi14-long65-short50 | Stress | 29 | -10.2403 | 0.3695 | 35.2229 | 5.7158 | -296.9691 | -11.1293 | — |
| M1-M8 | rsi21-long70-short50 | Base | 7 | -28.0322 | 0.2530 | 24.6306 | 17.2057 | -196.2253 | 79.1157 | no |
| M1-M8 | rsi21-long70-short50 | Stress | 7 | -29.5334 | 0.2393 | 25.5184 | 17.2289 | -206.7337 | 77.4946 | — |
| M1-M8 | rsi21-long70-short45 | Base | 5 | -14.0742 | 0.5017 | 11.6871 | 21.3903 | -70.3710 | 79.1157 | no |
| M1-M8 | rsi21-long70-short45 | Stress | 5 | -15.5784 | 0.4658 | 12.2833 | 21.4082 | -77.8920 | 77.4946 | — |
| M1-M9 | rsi21-long70-short50 | Base | 9 | -16.6558 | 0.5431 | 23.8718 | 20.8895 | -149.9022 | -1.3333 | no |
| M1-M9 | rsi21-long70-short50 | Stress | 9 | -18.1750 | 0.5171 | 24.7300 | 20.9026 | -163.5746 | -2.8336 | — |
| M1-M9 | rsi21-long70-short45 | Base | 7 | -2.6822 | 0.9094 | 11.2545 | 25.0230 | -18.7753 | -1.3333 | no |
| M1-M9 | rsi21-long70-short45 | Stress | 7 | -4.2072 | 0.8620 | 11.8274 | 25.0360 | -29.4505 | -2.8336 | — |
| M1-M10 | rsi21-long70-short50 | Base | 10 | -20.4017 | 0.4066 | 25.0471 | 16.2524 | -204.0166 | 47.8382 | no |
| M1-M10 | rsi21-long70-short50 | Stress | 10 | -21.9225 | 0.3842 | 26.3818 | 16.2582 | -219.2251 | 46.4120 | — |
| M1-M10 | rsi21-long70-short45 | Base | 8 | -8.7423 | 0.6873 | 12.1234 | 19.2466 | -69.9384 | 47.2064 | no |
| M1-M10 | rsi21-long70-short45 | Stress | 8 | -10.2679 | 0.6449 | 13.1609 | 19.2503 | -82.1432 | 45.7794 | — |

The OOS interval for a window ending at month k is M(k+1)..M(k+5), using 5 months.

## 5. Robustness

**Successful-config matrix (window × tested config; base costs)**

| Window | rsi14-long65-short50 | rsi14-long60-short50 | rsi21-long70-short50 | rsi21-long70-short45 |
|---|---|---|---|---|
| M1-M6 | Fail | Fail | — | — |
| M1-M7 | Fail | Fail | — | — |
| M1-M8 | — | — | Fail | Fail |
| M1-M9 | — | — | Fail | Fail |
| M1-M10 | — | — | Fail | Fail |

No successful config was found; side, regime, and neighbor robustness are unavailable.

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M6: rsi14-long65-short50 failed OOS (22 trades, net expectancy -0.813167204778727442253061982, PF 0.9460185572166726075378403907).
- M1-M6: rsi14-long60-short50 failed OOS (38 trades, net expectancy -3.0994682471487414596266398668, PF 0.7320683616868058514014162206).
- M1-M7: rsi14-long60-short50 failed OOS (41 trades, net expectancy -5.6685489006296503108649928605, PF 0.5030098083889584517438436464).
- M1-M7: rsi14-long65-short50 failed OOS (29 trades, net expectancy -8.739049479670827153287127499, PF 0.4186638909017481056939608102).
- M1-M8: rsi21-long70-short50 failed OOS (7 trades, net expectancy -28.032187566364859573944984591, PF 0.2529670882895447292232597702).
- M1-M8: rsi21-long70-short45 failed OOS (5 trades, net expectancy -14.074198048543570287366619583, PF 0.5017272558676146124134234167).
- M1-M9: rsi21-long70-short50 failed OOS (9 trades, net expectancy -16.655803611177364972267658193, PF 0.54305041654925703691952299).
- M1-M9: rsi21-long70-short45 failed OOS (7 trades, net expectancy -2.6821908532280769419782546306, PF 0.909404190956093826921940305).
- M1-M10: rsi21-long70-short50 failed OOS (10 trades, net expectancy -20.401660943393127643006425532, PF 0.40657330396954778616569618).
- M1-M10: rsi21-long70-short45 failed OOS (8 trades, net expectancy -8.742296407945460533077775332, PF 0.687307407693072474552808785).

Successful configs: none.

