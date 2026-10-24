# RSI Walk-Forward — SOL-USDT (4hour, 15 months)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 63fbbe83a517aa3429ffaac428c32b4a981954e05359494fdffcb8b4f15841ee |

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
| 1 | rsi21-long55-short40 | 13 | 2.1909 | 1.1235 | 11.9164 | 13.0098 | 0.7055 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long55-short40 | 14 | 14.3822 | 1.8727 | 11.9164 | 17.1378 | 12.9144 |
| 2 | rsi21-long60-short40 | 12 | 12.5820 | 1.5728 | 17.2700 | 21.5170 | 11.1175 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short40 | 11 | 8.6410 | 1.3240 | 17.5034 | 24.9694 | 7.1863 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short40 | 12 | 5.9083 | 1.2233 | 18.8816 | 22.9570 | 4.4471 |

### M1-M10 training (M11-M15)

No candidate met the training gates.

## 4. Holdout

Top two candidates per window on the following five months. Success uses base costs.

| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| M1-M6 | rsi21-long55-short40 | Base | 13 | -3.7893 | 0.8471 | 27.8635 | 17.9238 | -49.2604 | 23.0950 | no |
| M1-M6 | rsi21-long55-short40 | Stress | 13 | -5.2603 | 0.7972 | 29.1931 | 17.9443 | -68.3843 | 21.5581 | — |
| M1-M7 | rsi21-long55-short40 | Base | 15 | -11.3265 | 0.4650 | 26.5585 | 10.5217 | -169.8975 | -6.4834 | no |
| M1-M7 | rsi21-long55-short40 | Stress | 15 | -12.8288 | 0.4268 | 27.9172 | 10.5324 | -192.4317 | -7.9760 | — |
| M1-M7 | rsi21-long60-short40 | Base | 13 | -20.6272 | 0.2729 | 28.1275 | 12.2936 | -268.1541 | -13.1209 | no |
| M1-M7 | rsi21-long60-short40 | Stress | 13 | -22.1309 | 0.2477 | 30.2305 | 12.3089 | -287.7016 | -14.6034 | — |
| M1-M8 | rsi21-long65-short40 | Base | 9 | -16.7783 | 0.3584 | 15.1919 | 15.1710 | -151.0049 | 112.4145 | no |
| M1-M8 | rsi21-long65-short40 | Stress | 9 | -18.2721 | 0.3329 | 16.4449 | 15.1896 | -164.4489 | 110.7434 | — |
| M1-M9 | rsi21-long65-short40 | Base | 9 | 0.8787 | 1.0348 | 13.1382 | 24.4710 | 7.9080 | 21.3054 | no |
| M1-M9 | rsi21-long65-short40 | Stress | 9 | -0.6518 | 0.9753 | 13.6830 | 24.4659 | -5.8663 | 19.7712 | — |

The OOS interval for a window ending at month k is M(k+1)..M(k+5), using 5 months.

## 5. Robustness

**Successful-config matrix (window × tested config; base costs)**

| Window | rsi21-long55-short40 | rsi21-long60-short40 | rsi21-long65-short40 |
|---|---|---|---|
| M1-M6 | Fail | — | — |
| M1-M7 | Fail | Fail | — |
| M1-M8 | — | — | Fail |
| M1-M9 | — | — | Fail |
| M1-M10 | — | — | — |

No successful config was found; side, regime, and neighbor robustness are unavailable.

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M6: rsi21-long55-short40 failed OOS (13 trades, net expectancy -3.7892629587731998632171728969, PF 0.8470918775556517070639144488).
- M1-M7: rsi21-long55-short40 failed OOS (15 trades, net expectancy -11.326502669332494177094871112, PF 0.465037770126918104461669448).
- M1-M7: rsi21-long60-short40 failed OOS (13 trades, net expectancy -20.627238596459528022920132277, PF 0.2728558643347222579078262558).
- M1-M8: rsi21-long65-short40 failed OOS (9 trades, net expectancy -16.778322513077156562991118022, PF 0.3583729199686449560677986595).
- M1-M9: rsi21-long65-short40 failed OOS (9 trades, net expectancy 0.8786682519151842648376172673, PF 1.0347645590739998607775964278).
- M1-M10: no candidate met the training gates.

Successful configs: none.

