# Analyzer Experiments — Consolidated Research Index

> **Research only. Not financial advice. No configuration in this repository has demonstrated a robust out-of-sample edge after realistic trading costs.**

This index summarizes every experiment under `process/analyzer/experiments/`. The value of this work is the negative evidence: simple RSI, ATR, Donchian, time-series momentum, and short-horizon mean-reversion rules were all tested and none survived validation.

## Common assumptions

Unless an experiment says otherwise:

- Signals are evaluated on **closed candles only**.
- Signals execute at the **next candle open**.
- One position per symbol; no pyramiding.
- **Base costs:** 10 bps fee + 5 bps slippage per side.
- **Stress costs:** 20 bps fee + 10 bps slippage per side.
- Costs are applied to traded notional / turnover.
- Chronological splits; the test or holdout period is not used for tuning.
- Results are historical simulations, not forecasts.

## Experiment summary

### Active research strategies

| # | Experiment | Strategy | Universe / timeframe | Data and split | Key result | Verdict |
|---|---|---|---|---|---|---|
| 1 | [RSI momentum](rsi-momentum-4h/README.md) | RSI 21, long >60, short/exit <50 | BTC/ETH/SOL, 4hour | 12 months; 24-point grid | Baseline validation net expectancy **−13.77**, PF **0.09**; no candidate met the criteria; no holdout scored | **Insufficient evidence** |
| 2 | [RSI per-symbol 1-year](rsi-tuning-1y/README.md) | Top-3 per symbol by PF + expectancy rank | BTC/BNB/XRP/SOL, 4hour | 1 year, M1–M4 tune, M5–M10 log | All four symbols failed; M5 negative for every top-3 config | **Fail** |
| 3 | [RSI walk-forward](rsi-walkforward/README.md) | Per-symbol top-3, M1–M6 … M1–M10, next 5 months | BTC/ETH/SOL/XRP/BNB, 4hour | 15 months, 5 windows | BTC +4.94 and XRP +2.56 OOS stress expectancy passed; ETH/SOL/BNB failed | **Fail overall (2/5 symbols)** |
| 4 | [RSI fixed 3-year](rsi-fixed-3y/README.md) | Fixed RSI 21, long 60, short 40 | BTC/BNB/XRP/SOL, 1day | 3 years full-period, no holdout | BNB **+16.63** / PF 1.48 and XRP **+60.59** / PF 2.11; BTC −1.68 and SOL −93.33 | **Mixed; not OOS-validated** |
| 5 | [RSI + ATR 4hour](rsi-atr-4h/README.md) | Fixed RSI 16 / 60:40 + ATR SL/TP variants | BTC/BNB/XRP/SOL, 4hour | 3 years full-period | ATR off best for BTC/SOL; ATR 2.0/3.0 rescued XRP; BNB negative in every variant | **Mixed; no consistent improvement** |
| 6 | [RSI + ATR 15min](rsi-atr-15m/README.md) | Same, 15min | BTC/BNB/XRP/SOL, 15min | 3 years, ~105k candles/symbol | Every variant negative after costs | **Fail** |
| 7 | [ATR grid top-3, month-12](rsi-atr-grid-4h/README.md) | RSI 16 / 60:40 + ATR grid 1.5–5.0 | BTC/BNB/XRP/SOL, 4hour | 12 months; top-3 train, month-12 test | Only BNB’s top-3 was positive on month 12; BTC/SOL/XRP still negative | **Fail overall** |
| 8 | [Donchian v1](donchian-v1-4h/README.md) | Donchian 20/55 + EMA200 + ATR trailing, long-only | BTC/ETH/SOL/BNB/XRP, 4hour and 1day | 15 months; 5 windows | 4hour SOL/XRP passed; 1day only BTC passed (1 trade) | **Fail overall** |
| 9 | [Donchian v2](donchian-v2-4h/README.md) | Same, 5-year validation, min 10 OOS trades, stability | BTC/ETH/SOL/BNB/XRP, 4hour and 1day | 5 years; 8 non-overlapping 6-month OOS windows | 4hour SOL passed one window; 1day none | **Fail overall (1/5)** |
| 10 | [TS momentum 30-day](ts-momentum-30d/report.md) | 30-day return + volatility targeting | BTC, 1day | 5 years; 60/20/20 split | Full-period B +68.10%, but test **−0.84%** base / **−4.43%** stress; benchmark test −22.59% | **Not validated** |
| 11 | [TS momentum 7-day](ts-momentum-7d/report.md) | 7-day lookback | BTC, 1day | 5 years; same split | Materially worse: full −18.47% (B), test −17.36%, more turnover | **Fail** |
| 12 | [TS walk-forward grid](ts-momentum-wf/report.md) | Lookback {30,60,90} × regime {off,on} × vol target {15,20,25} | BTC, 1day | 5 years; 8 windows | 0/8 windows passed; every window failed the 20-trade OOS gate (actual 1–10 trades) | **Fail / structurally inconclusive** |
| 13 | [Percent mean reversion](percent-momentum/README.md) | C1 = 5–7 days before C2; C1 >5% above C2 → buy; >5% below → sell | BTC/XRP/SOL, 1day | 2 years; 1-year tune, 1-year test | Training Sharpe negative for every symbol; test BTC −37.8%, XRP −12.45%, SOL −50.66%; only XRP beat B&H but still negative | **Fail** |

### Historical RSI-divergence strategy (closed)

The original strategy was RSI divergence. It is kept for the record and is not the current strategy.

| Experiment | Scope | Result |
|---|---|---|
| [Sweep](historical/rsi-divergence-sweep/report.md) | 648-point grid sampled to 30 candidates, 1hour + 4hour, base/stress costs | **Insufficient evidence** — no candidate cleared validation on every timeframe. |
| [Cost ladder](historical/rsi-divergence-cost-ladder/report.md) | 4hour, 0–30 bps/side | Holdout gross edge exists but sits below realistic costs; best break-even **6.86 bps/side**. |
| [Execution modelling](historical/rsi-divergence-execution/report.md) | market vs limit/retest entries | Nominal rescue for a retest-entry model, but the holdout edge is within noise and the holdout had been reused. |
| [Fresh confirmation](historical/rsi-divergence-confirm/report.md) | Pre-registered one-shot, 4hour, 2022-11 → 2024-10 | **Failed** — pooled net expectancy **−2.15**, PF **0.80**, 0/3 symbols positive. |

## What the evidence says

- **Classic RSI mean reversion** (buy oversold, sell overbought) did not survive costs.
- **RSI momentum** (buy strength) was better than mean reversion in some 4hour samples but failed out-of-sample.
- **ATR SL/TP** helped selectively (XRP, some BNB cases) but was not a general improvement.
- **15min** lost under every ATR variant; costs dominate.
- **Donchian trend following** showed isolated wins (SOL 4hour) but failed 5-year, multi-symbol validation.
- **Time-series momentum** protected capital versus BTC buy-and-hold in the test year but did not make money; 7-day lookbacks were worse.
- **Short-horizon mean reversion** (5–7 day, 5%) had negative training Sharpe and negative test returns.
- **Tuning on small samples** repeatedly selected overfit configurations. The walk-forward and top-3 tests exposed this.

## What to try next

1. **Do not deploy any of these configurations with real capital.**
2. If continuing research, prioritise:
   - cross-sectional momentum with a portfolio layer,
   - carry/funding/basis where the exchange supports it,
   - volatility/regime allocation rather than directional prediction,
   - execution/market-making only with the necessary infrastructure.
3. Require:
   - ≥10–20 OOS trades per validation window (or longer OOS windows),
   - non-overlapping OOS,
   - parameter-stability checks,
   - stress costs and a buy-and-hold benchmark,
   - a pre-registered test period that is not reused.
4. Accept the possibility that this stack has **no reliable edge**.

## Reproducibility

Each experiment directory contains its own `report.md`, `results.json`, and where applicable `leaderboard.csv`, `equity.csv`, or per-symbol reports. Reproduction commands are in each experiment's README/report. Data caches live outside version control under `data/`.

The current branch is `feat/analyzer-rsi`. Do not publish `.env`, API keys, wallet data, raw exchange data if restricted, or database dumps.

## Disclaimer

This repository is a research log. Nothing here is investment advice, a promise of profit, or a validated trading system. Historical simulations are sensitive to data, costs, execution assumptions, and regime. Use at your own risk.
