# Donchian Trend-Following — SOL-USDT (1day, 15 months)

Research artifact for `donchian-v1`. This is not a profitability claim.

## 1. Dataset

| Field | Value |
|---|---|
| Symbol | SOL-USDT |
| Timeframe | 1day |
| Date range | 2025-07-24T00:00:00Z .. 2026-10-21T00:00:00Z |
| Candles | 455 |
| SHA-256 | 3fd29ebffd6d5143dc1c2f48826f28435d28f7cf2196ff21498056cf2d5b6cfc |

| Train window | Train range | Train candles | OOS window | OOS range | OOS candles | Train first | Train last | OOS first | OOS last |
|---|---|---:|---|---|---:|---|---|---|---|
| M1-M6 | 0..182 | 182 | M7-M11 | 182..333 | 151 | 2025-07-24T00:00:00Z | 2026-01-21T00:00:00Z | 2026-01-22T00:00:00Z | 2026-06-21T00:00:00Z |
| M1-M7 | 0..212 | 212 | M8-M12 | 212..364 | 152 | 2025-07-24T00:00:00Z | 2026-02-20T00:00:00Z | 2026-02-21T00:00:00Z | 2026-07-22T00:00:00Z |
| M1-M8 | 0..242 | 242 | M9-M13 | 242..394 | 152 | 2025-07-24T00:00:00Z | 2026-03-22T00:00:00Z | 2026-03-23T00:00:00Z | 2026-08-21T00:00:00Z |
| M1-M9 | 0..273 | 273 | M10-M14 | 273..424 | 151 | 2025-07-24T00:00:00Z | 2026-04-22T00:00:00Z | 2026-04-23T00:00:00Z | 2026-09-20T00:00:00Z |
| M1-M10 | 0..303 | 303 | M11-M15 | 303..455 | 152 | 2025-07-24T00:00:00Z | 2026-05-22T00:00:00Z | 2026-05-23T00:00:00Z | 2026-10-21T00:00:00Z |

## 2. Experiment

| Field | Value |
|---|---|
| Strategy | donchian-v1 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT, BNB-USDT, XRP-USDT (5) |
| Entry | Close > prior Donchian high (N ∈ [20, 55]) and close > EMA(200) |
| Exit | ATR(14) trailing stop (highest high since entry − multiplier × ATR) or close < EMA(200) at the next open |
| Grid | entry N [20, 55] × Donchian exit N 55 × ATR multiplier [2, 3, 4] × EMA 200 × ATR 14 = 6 configs (Donchian exit N is fixed metadata and is not an active exit in this variant) |
| Walk-forward | Expanding training windows M1-M6 through M1-M10; each tested on the next 5 months |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Initial fund | 1000 |
| Budget per position | init_fund / 2 |

Execution assumptions:

- Long-only; one position per symbol; no shorts and no pyramiding.
- A signal produced at a candle close fills at the next candle open.
- The trailing stop is checked intrabar using the highest high and ATR known at the prior close. A gap below the stop exits at the open; otherwise the exit price is the stop level.
- An EMA cross below is acted on at the next open; the stop takes precedence when both trigger in the same bar.
- Costs are charged on the actual entry and exit notional, not on the initial budget.

Selection and pass policy:

- Each window ranks all 6 configs on base-cost training metrics by the sum of the profit-factor rank (∞ first) and net-expectancy rank. The top 3 advance to the holdout.
- There is no minimum-trade gate.
- A run passes only when stress OOS net expectancy > 0, stress OOS PF > 1, and stress OOS net PnL > the stress buy-and-hold benchmark.
- A symbol passes when any top-3 run in any window passes. The experiment passes only when all 5 symbols pass.
- The benchmark buys at the first OOS open and sells at the last OOS close with the same half-budget and cost scenario.

## 3. Validation

Top 3 configs per expanding training window, selected on base-cost training metrics.

### M1-M6 training (M7-M11)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M7 training (M8-M12)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M8 training (M9-M13)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M9 training (M10-M14)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

### M1-M10 training (M11-M15)

| Rank | Config | Trades | Net expectancy | PF | Max DD % | Stderr | Stress net expectancy | Stress PF |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | entry20-exit55-ema200-atr14x2 | 0 | 0 | — | 0 | 0 | 0 | — |
| 2 | entry20-exit55-ema200-atr14x3 | 0 | 0 | — | 0 | 0 | 0 | — |
| 3 | entry20-exit55-ema200-atr14x4 | 0 | 0 | — | 0 | 0 | 0 | — |

## 4. Holdout

Top 3 configs per window on the following 5 months, with buy-and-hold comparison.

### M7-M11 holdout (M1-M6)

Buy & hold benchmark (half budget): base net PnL -221.4660 (-44.2932%), stress net PnL -222.6356 (-44.5271%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -221.4660 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -222.6356 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -221.4660 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -222.6356 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -221.4660 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -222.6356 | yes | Fail |

### M8-M12 holdout (M1-M7)

Buy & hold benchmark (half budget): base net PnL -41.0603 (-8.2121%), stress net PnL -42.5009 (-8.5002%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | -41.0603 | yes | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | -42.5009 | yes | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | -41.0603 | yes | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | -42.5009 | yes | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | -41.0603 | yes | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | -42.5009 | yes | Fail |

### M9-M13 holdout (M1-M8)

Buy & hold benchmark (half budget): base net PnL 41.8702 (8.3740%), stress net PnL 40.3051 (8.0610%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 0 | 0 | — | 0 | 0 | 41.8702 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 0 | 0 | — | 0 | 0 | 40.3051 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 0 | 0 | — | 0 | 0 | 41.8702 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 0 | 0 | — | 0 | 0 | 40.3051 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 0 | 41.8702 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 0 | 40.3051 | no | Fail |

### M10-M14 holdout (M1-M9)

Buy & hold benchmark (half budget): base net PnL 138.0808 (27.6162%), stress net PnL 136.3711 (27.2742%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 2 | -22.5653 | 0 | 5.3408 | -53.4084 | 138.0808 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 2 | -24.0337 | 0 | 5.7835 | -57.8349 | 136.3711 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 1 | 14.5459 | ∞ | 0.8159 | 6.2681 | 138.0808 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 1 | 13.0218 | ∞ | 0.9642 | 3.2542 | 136.3711 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 0 | 0 | — | 0 | 91.3305 | 138.0808 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 0 | 0 | — | 0 | 89.6910 | 136.3711 | no | Fail |

### M11-M15 holdout (M1-M10)

Buy & hold benchmark (half budget): base net PnL 131.8147 (26.3629%), stress net PnL 130.1144 (26.0229%).

| Config | Costs | Trades | Net expectancy | PF | Max DD % | Net PnL | Benchmark net PnL | Beat benchmark | Success |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| entry20-exit55-ema200-atr14x2 | Base | 3 | -10.9703 | 0.2708 | 4.5131 | -32.9110 | 131.8147 | no | — |
| entry20-exit55-ema200-atr14x2 | Stress | 3 | -12.4561 | 0.2226 | 4.8067 | -37.3683 | 130.1144 | no | Fail |
| entry20-exit55-ema200-atr14x3 | Base | 2 | 2.8449 | 1.6425 | 0.8729 | 5.6899 | 131.8147 | no | — |
| entry20-exit55-ema200-atr14x3 | Stress | 2 | 1.3384 | 1.2588 | 1.0212 | 2.6768 | 130.1144 | no | Fail |
| entry20-exit55-ema200-atr14x4 | Base | 1 | 65.9576 | ∞ | 0 | 65.9576 | 131.8147 | no | — |
| entry20-exit55-ema200-atr14x4 | Stress | 1 | 64.3562 | ∞ | 0 | 64.3562 | 130.1144 | no | Fail |

## 5. Robustness

**Success matrix (window × tested config; stress OOS gates)**

| Window | entry20-exit55-ema200-atr14x2 | entry20-exit55-ema200-atr14x3 | entry20-exit55-ema200-atr14x4 |
|---|---|---|---|
| M1-M6 | Fail | Fail | Fail |
| M1-M7 | Fail | Fail | Fail |
| M1-M8 | Fail | Fail | Fail |
| M1-M9 | Fail | Fail | Fail |
| M1-M10 | Fail | Fail | Fail |

**Config consistency (across the five expanding windows)**

| Config | Windows selected | Windows passed | Best OOS stress expectancy |
|---|---:|---:|---:|
| entry20-exit55-ema200-atr14x2 | 5 | 0 | 0 |
| entry20-exit55-ema200-atr14x3 | 5 | 0 | 13.0218 |
| entry20-exit55-ema200-atr14x4 | 5 | 0 | 64.3562 |

**Benchmark comparison by window (stress costs)**

| Window | Best top-3 config | Best stress net PnL | Benchmark stress net PnL | Difference | Result |
|---|---|---:|---:|---:|---|
| M1-M6 | entry20-exit55-ema200-atr14x4 | 0 | -222.6356 | 222.6356 | Beat |
| M1-M7 | entry20-exit55-ema200-atr14x4 | 0 | -42.5009 | 42.5009 | Beat |
| M1-M8 | entry20-exit55-ema200-atr14x4 | 0 | 40.3051 | -40.3051 | Underperform |
| M1-M9 | entry20-exit55-ema200-atr14x4 | 89.6910 | 136.3711 | -46.6801 | Underperform |
| M1-M10 | entry20-exit55-ema200-atr14x4 | 64.3562 | 130.1144 | -65.7582 | Underperform |

## 6. Decision

**Per-symbol verdict: Fail**

**Overall experiment verdict: Fail** (all 5 symbols must pass)

Reasons:

- No top-3 config met the OOS stress gates (net expectancy > 0, PF > 1, net PnL above the buy-and-hold benchmark).
- M1-M6: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -222.63556705010422295993206207.
- M1-M7: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark -42.500885687293339631554085970.
- M1-M8: best top-3 stress run entry20-exit55-ema200-atr14x4 had 0 OOS trades, net expectancy 0, PF infinite, net PnL 0 vs benchmark 40.305091626072836928786824399.
- M1-M9: best top-3 stress run entry20-exit55-ema200-atr14x3 had 1 OOS trades, net expectancy 13.021784973159173567400990156, PF infinite, net PnL 3.254216206522083948944912062 vs benchmark 136.37114410681399631675874770.
- M1-M10: best top-3 stress run entry20-exit55-ema200-atr14x4 had 1 OOS trades, net expectancy 64.356242884353355479554155635, PF infinite, net PnL 64.356242884353355479554155635 vs benchmark 130.11444141689373297002724795.

Successful runs: none.

