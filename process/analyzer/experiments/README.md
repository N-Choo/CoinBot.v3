# Analyzer Experiments — Research Index

> **Research only. Not financial advice. No strategy in this repository has demonstrated a robust out-of-sample edge after realistic trading costs.**

This directory documents experiments under `process/analyzer/experiments/`.

The primary finding is negative: **none of the tested strategy families has demonstrated a reliable, cost-adjusted out-of-sample edge.** RSI, ATR, Donchian, time-series momentum, and short-horizon mean-reversion strategies all failed to establish consistent performance across validation periods, symbols, and execution assumptions.

## Table of Contents

- [Common Assumptions](#common-assumptions)
- [Active Research Strategies](#active-research-strategies)
- [Historical RSI Divergence](#historical-rsi-divergence)
- [Key Findings](#key-findings)
- [Reproducibility](#reproducibility)
- [Disclaimer](#disclaimer)

## Common Assumptions

Unless an experiment specifies otherwise:

- **Signal timing:** Closed candles only.
- **Execution:** Next candle open.
- **Position sizing:** One position per symbol; no pyramiding.
- **Base costs:** 10 bps fee + 5 bps slippage per side.
- **Stress costs:** 20 bps fee + 10 bps slippage per side.
- **Cost basis:** Traded notional / turnover.
- **Validation:** Chronological splits; test and holdout periods are excluded from tuning.
- **Interpretation:** Historical simulations, not forecasts.

## Active Research Strategies

### RSI Strategies

| Experiment | Strategy | Scope | Key Result | Verdict |
|---|---|---|---|---|
| [RSI Momentum — 4h](rsi-momentum-4h/README.md) | RSI 21; long above 60, short/exit below 50 | BTC, ETH, SOL · 4h · 12 months | Baseline validation net expectancy: **−13.77**; PF: **0.09**. No candidate passed; no holdout scored. | Insufficient evidence |
| [RSI Per-Symbol — 1 Year](rsi-tuning-1y/README.md) | Top 3 configurations per symbol, ranked by PF and expectancy | BTC, BNB, XRP, SOL · 4h · 1 year | All four symbols failed. Every top-3 configuration had negative M5 results. | Fail |
| [RSI Walk-Forward](rsi-walkforward/README.md) | Per-symbol top 3 configurations across expanding training windows | BTC, ETH, SOL, XRP, BNB · 4h · 15 months | BTC (+4.94) and XRP (+2.56) passed OOS stress expectancy checks. ETH, SOL, and BNB failed. | Fail overall — 2/5 symbols |
| [RSI Fixed — 3 Years](rsi-fixed-3y/README.md) | Fixed RSI 21; long at 60, short at 40 | BTC, BNB, XRP, SOL · 1d · 3 years | BNB: **+16.63 / PF 1.48**. XRP: **+60.59 / PF 2.11**. BTC and SOL were negative. No holdout. | Mixed; not OOS-validated |
| [RSI + ATR — 4h](rsi-atr-4h/README.md) | RSI 16, 60:40 thresholds, ATR stop-loss/take-profit variants | BTC, BNB, XRP, SOL · 4h · 3 years | ATR-off performed best for BTC and SOL. ATR 2.0/3.0 improved XRP. BNB was negative across all variants. | Mixed; no consistent improvement |
| [RSI + ATR — 15m](rsi-atr-15m/README.md) | RSI 16, 60:40 thresholds, ATR stop-loss/take-profit variants | BTC, BNB, XRP, SOL · 15m · 3 years | Every variant lost money after costs. | Fail |
| [ATR Grid — Month 12](rsi-atr-grid-4h/README.md) | RSI 16, 60:40 thresholds, ATR grid from 1.5 to 5.0 | BTC, BNB, XRP, SOL · 4h · 12 months | Only BNB's top-3 configurations were profitable in month 12. BTC, SOL, and XRP remained negative. | Fail overall |

### Trend-Following Strategies

| Experiment | Strategy | Scope | Key Result | Verdict |
|---|---|---|---|---|
| [Donchian v1](donchian-v1-4h/README.md) | Donchian 20/55, EMA 200, ATR trailing stop; long-only | BTC, ETH, SOL, BNB, XRP · 4h and 1d · 15 months | SOL and XRP passed on 4h. On 1d, only BTC passed, with one trade. | Fail overall |
| [Donchian v2](donchian-v2-4h/README.md) | Same strategy with stricter validation and stability checks | BTC, ETH, SOL, BNB, XRP · 4h and 1d · 5 years | Across eight non-overlapping six-month OOS windows, only SOL on 4h passed one window. No 1d configuration passed. | Fail overall — 1/5 symbols |
| [TS Momentum — 30 Days](ts-momentum-30d/report.md) | 30-day return with volatility targeting | BTC · 1d · 5 years | Full-period baseline return: **+68.10%**. Test return: **−0.84%** at base costs and **−4.43%** under stress. BTC buy-and-hold returned −22.59% during the test. | Not validated |
| [TS Momentum — 7 Days](ts-momentum-7d/report.md) | 7-day momentum lookback | BTC · 1d · 5 years | Baseline full-period return: **−18.47%**; test return: **−17.36%**. Higher turnover and worse performance than the 30-day variant. | Fail |
| [TS Momentum Walk-Forward](ts-momentum-wf/report.md) | Lookback 30/60/90 days, optional regime filter, 15/20/25% volatility targets | BTC · 1d · 5 years | **0/8 windows passed.** Each window failed the 20-trade OOS minimum, producing only 1–10 trades. | Fail; inconclusive samples |

### Mean-Reversion Strategies

| Experiment | Strategy | Scope | Key Result | Verdict |
|---|---|---|---|---|
| [Percent Mean Reversion](percent-momentum/README.md) | Compare prices 5–7 days apart; trade when the difference exceeds ±5% | BTC, XRP, SOL · 1d · 2 years | Training Sharpe was negative for every symbol. Test returns: BTC **−37.8%**, XRP **−12.45%**, SOL **−50.66%**. Only XRP beat buy-and-hold, while still losing money. | Fail |

## Historical RSI Divergence

The original strategy was based on RSI divergence. This research is **closed** and retained for historical reference; it is not the current strategy.

| Experiment | Scope | Key Result |
|---|---|---|
| [Parameter Sweep](historical/rsi-divergence-sweep/report.md) | 648-point grid sampled to 30 candidates; 1h and 4h; base and stress costs | No candidate passed validation across every timeframe. |
| [Cost Ladder](historical/rsi-divergence-cost-ladder/report.md) | 4h; costs from 0–30 bps per side | The holdout showed a gross edge, but it was below realistic costs. Best break-even cost: **6.86 bps per side**. |
| [Execution Modelling](historical/rsi-divergence-execution/report.md) | Market, limit, and retest entries | A retest-entry variant showed a nominal improvement, but the holdout edge was within noise and the holdout had been reused. |
| [Fresh Confirmation](historical/rsi-divergence-confirm/report.md) | Preregistered, one-shot 4h test; November 2022–October 2024 | **Failed:** pooled net expectancy **−2.15**, PF **0.80**, and 0/3 symbols profitable. |

## Key Findings

1. **RSI mean reversion did not survive trading costs.** Buying oversold conditions and selling overbought conditions failed to produce robust results.
2. **RSI momentum showed isolated improvements but failed to generalize.** Some 4h configurations performed better than mean-reversion variants, but out-of-sample results were inconsistent.
3. **ATR stops and targets were not a universal improvement.** They helped selected XRP and BNB configurations but did not consistently improve results across symbols.
4. **Short timeframes were especially sensitive to costs.** Every tested 15m RSI + ATR variant lost money after costs.
5. **Donchian trend following lacked cross-market stability.** Isolated wins did not persist across the stricter five-year validation.
6. **Time-series momentum reduced losses relative to buy-and-hold in one test period but did not make money.** The shorter 7-day lookback performed worse.
7. **Short-horizon mean reversion failed.** All symbols had negative training Sharpe and negative test returns.
8. **Small-sample tuning repeatedly selected overfit configurations.** Walk-forward testing and top-3 configuration checks exposed this weakness.

**Research conclusion:** Positive historical results on individual symbols or periods are not sufficient evidence of a tradable edge. Robustness across unseen data, realistic costs, adequate trade counts, and market regimes remains unproven.


## Disclaimer

This repository is a research log, not investment advice or a validated trading system. No result guarantees future performance. Historical simulations depend on data quality, fees, slippage, execution assumptions, and market regimes.

Use at your own risk.
