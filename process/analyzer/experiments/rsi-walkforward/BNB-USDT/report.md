# RSI Walk-Forward — BNB-USDT (4hour, 15 months)

Research artifact for `rsi-mom-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | BNB-USDT |
| Timeframe | 4hour |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-22T16:00:00Z |
| Candles | 2735 |
| SHA-256 | 7829044720718c6187af2a1742d2bb1d329b42e60571315b8322353743e22ddd |

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
| 1 | rsi21-long65-short50 | 11 | 7.3997 | 1.4427 | 9.9747 | 17.1224 | 5.8874 |
| 2 | rsi14-long70-short50 | 15 | 5.0350 | 1.3604 | 10.6574 | 11.6438 | 3.5274 |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short50 | 11 | 7.3997 | 1.4427 | 9.9747 | 17.1224 | 5.8874 |
| 2 | rsi14-long70-short50 | 15 | 5.0350 | 1.3604 | 10.6574 | 11.6438 | 3.5274 |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi14-long70-short40 | 11 | 17.5658 | 2.2545 | 7.9028 | 17.7053 | 16.0780 |
| 2 | rsi21-long65-short50 | 13 | 15.2463 | 1.9701 | 9.9747 | 17.7370 | 13.7540 |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long65-short45 | 10 | 22.1198 | 3.3184 | 5.0755 | 17.6395 | 20.6345 |
| 2 | rsi14-long70-short40 | 13 | 14.6831 | 2.0809 | 7.9028 | 15.1896 | 13.1984 |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress expectancy |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | rsi21-long70-short50 | 10 | 21.1384 | 2.2617 | 8.6040 | 22.9416 | 19.6448 |
| 2 | rsi21-long65-short40 | 11 | 15.3997 | 2.1706 | 7.4400 | 17.2608 | 13.9121 |

## 4. Holdout

Top two candidates per window on the following five months. Success uses base costs.

| Window | Config | Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Success |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| M1-M6 | rsi21-long65-short50 | Base | 8 | 7.6430 | 1.9355 | 4.4782 | 15.0647 | 61.1440 | 62.3631 | no |
| M1-M6 | rsi21-long65-short50 | Stress | 8 | 6.1628 | 1.6909 | 5.2675 | 15.0847 | 49.3022 | 60.9587 | — |
| M1-M6 | rsi14-long70-short50 | Base | 8 | 10.0750 | 2.0480 | 5.0554 | 16.5695 | 80.6003 | 61.9214 | no |
| M1-M6 | rsi14-long70-short50 | Stress | 8 | 8.5960 | 1.8291 | 5.6102 | 16.5926 | 68.7680 | 60.5164 | — |
| M1-M7 | rsi21-long65-short50 | Base | 10 | -5.0746 | 0.6181 | 9.8461 | 9.4167 | -50.7464 | -0.1014 | no |
| M1-M7 | rsi21-long65-short50 | Stress | 10 | -6.5700 | 0.5370 | 10.8712 | 9.4281 | -65.6997 | -1.5994 | — |
| M1-M7 | rsi14-long70-short50 | Base | 10 | -3.2400 | 0.7649 | 8.6051 | 10.1354 | -32.4003 | 6.6699 | no |
| M1-M7 | rsi14-long70-short50 | Stress | 10 | -4.7366 | 0.6775 | 9.6625 | 10.1488 | -47.3656 | 5.1821 | — |
| M1-M8 | rsi14-long70-short40 | Base | 11 | -8.0704 | 0.4487 | 9.3577 | 7.1776 | -88.7748 | 52.5136 | no |
| M1-M8 | rsi14-long70-short40 | Stress | 11 | -9.5712 | 0.3864 | 10.8677 | 7.1839 | -105.2827 | 50.9324 | — |
| M1-M8 | rsi21-long65-short50 | Base | 13 | -7.0310 | 0.4487 | 11.5345 | 6.7659 | -91.4030 | 52.5136 | no |
| M1-M8 | rsi21-long65-short50 | Stress | 13 | -8.5312 | 0.3868 | 12.5433 | 6.7746 | -110.9057 | 50.9324 | — |
| M1-M9 | rsi21-long65-short45 | Base | 13 | -5.9941 | 0.5679 | 7.9015 | 7.2418 | -77.9232 | 20.8240 | no |
| M1-M9 | rsi21-long65-short45 | Stress | 13 | -7.5056 | 0.5009 | 9.7573 | 7.2461 | -97.5729 | 19.2904 | — |
| M1-M9 | rsi14-long70-short40 | Base | 13 | -4.6115 | 0.6490 | 7.0660 | 7.0288 | -59.9491 | 20.8240 | no |
| M1-M9 | rsi14-long70-short40 | Stress | 13 | -6.1229 | 0.5683 | 8.3325 | 7.0308 | -79.5978 | 19.2904 | — |
| M1-M10 | rsi21-long70-short50 | Base | 8 | -7.6946 | 0.5884 | 9.4993 | 12.0219 | -61.5570 | 17.9367 | no |
| M1-M10 | rsi21-long70-short50 | Stress | 8 | -9.2105 | 0.5312 | 10.1448 | 12.0251 | -73.6837 | 16.4659 | — |
| M1-M10 | rsi21-long65-short40 | Base | 8 | -0.3991 | 0.9775 | 7.6056 | 16.4153 | -3.1932 | 18.4496 | no |
| M1-M10 | rsi21-long65-short40 | Stress | 8 | -1.9185 | 0.8973 | 8.3502 | 16.4024 | -15.3479 | 16.9794 | — |

The OOS interval for a window ending at month k is M(k+1)..M(k+5), using 5 months.

## 5. Robustness

**Successful-config matrix (window × tested config; base costs)**

| Window | rsi21-long65-short50 | rsi14-long70-short50 | rsi14-long70-short40 | rsi21-long65-short45 | rsi21-long70-short50 | rsi21-long65-short40 |
|---|---|---|---|---|---|---|
| M1-M6 | Fail | Fail | — | — | — | — |
| M1-M7 | Fail | Fail | — | — | — | — |
| M1-M8 | Fail | — | Fail | — | — | — |
| M1-M9 | — | — | Fail | Fail | — | — |
| M1-M10 | — | — | — | — | Fail | Fail |

No successful config was found; side, regime, and neighbor robustness are unavailable.

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- M1-M6: rsi21-long65-short50 failed OOS (8 trades, net expectancy 7.642997284317073815884791429, PF 1.9355248493008053854874119104).
- M1-M6: rsi14-long70-short50 failed OOS (8 trades, net expectancy 10.075037112732461365763255221, PF 2.0479849557923573939767951576).
- M1-M7: rsi21-long65-short50 failed OOS (10 trades, net expectancy -5.0746364372843384192233330895, PF 0.6180658405164825450204791542).
- M1-M7: rsi14-long70-short50 failed OOS (10 trades, net expectancy -3.2400319602467555540320802664, PF 0.7648606078127444260103406039).
- M1-M8: rsi14-long70-short40 failed OOS (11 trades, net expectancy -8.070433539330546665433405865, PF 0.4486582711326318383702386903).
- M1-M8: rsi21-long65-short50 failed OOS (13 trades, net expectancy -7.0310027746034503336951593377, PF 0.44868660860686521270315555).
- M1-M9: rsi21-long65-short45 failed OOS (13 trades, net expectancy -5.9940918613916864586232466637, PF 0.5678994149104412935767611572).
- M1-M9: rsi14-long70-short40 failed OOS (13 trades, net expectancy -4.6114713585795216226612371784, PF 0.648984948186566327813015768).
- M1-M10: rsi21-long70-short50 failed OOS (8 trades, net expectancy -7.6946200998300022619290675724, PF 0.5884197846449185317277896599).
- M1-M10: rsi21-long65-short40 failed OOS (8 trades, net expectancy -0.3991461920798045684350542511, PF 0.9774969978358848950671161046).

Successful configs: none.

