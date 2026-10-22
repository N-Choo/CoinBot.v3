# Analyzer Profitability Research — Design

- **Status:** Approved direction; Experiment 1 approved to run
- **Date:** 2026-10-22
- **Scope:** Determine whether the RSI-divergence strategy has a gross edge worth pursuing, and which cost, execution, exit, or regime changes could make it net profitable. This is a research program, not a promise of profitability.
- **Related:**
  - `docs/superpowers/specs/2026-10-22-analyzer-hyperparameter-screening-design.md`
  - `docs/superpowers/plans/2026-10-22-analyzer-hyperparameter-screening.md`
  - `out/sweep/report.md`

## 1. What the screening established

- 1hour: all 30 candidates negative after costs; the gross edge is also negative.
- 4hour: the baseline is roughly gross-positive but net-negative. With ~1.5 round-trip cost per 500-unit position, net −0.33/trade implies a gross edge of roughly +1.2/trade (~0.24%). That is below the 30 bps round-trip assumption.
- The four 4hour validation winners looked strong in-sample but all lost on the holdout (−0.8 to −5.1/trade).
- 24 months, 3 majors, one chronological split, 30 of 648 grid points.

**Conclusion:** more grid search on the same window is not the next step. The next questions are whether a real gross edge exists, and whether execution/exit/regime changes can preserve it after costs.

## 2. Research sequence

### Experiment 1 — Cost ladder (run first, decisive)

**Question:** at what all-in per-side cost does the 4hour strategy break even, and is that cost realistically achievable?

**Design:**
- Configs: baseline and the four frozen 4hour candidates (`joint-13`, `joint-24`, `max_age=4`, `atr_tp=4.0`).
- Symbols: BTC-USDT, ETH-USDT, SOL-USDT.
- Data: the existing 24-month 4hour cache (`data/sweep`, as-of `1792661878840`).
- Splits: the same 60/20/20 chronological splits and 3 expanding folds.
- Cost levels, per side: 0, 2.5, 5, 7.5, 10, 15, 30 bps. (15 = base; 30 = stress.)
- Partitions: validation and holdout for the frozen set; validation folds for selection context.
- Metrics per config × cost × partition × symbol and pooled: trades, gross/net expectancy, net PnL, PF, win rate, max DD %, stderr.
- Break-even per-side cost: linear interpolation of pooled net expectancy between adjacent levels; report `0 (no gross edge)` if 0 bps is negative and `> 30` if still positive at 30.
- Also report the fraction of symbols and folds positive at 0, 5, 10, 15, and 30 bps.

**Decision rule:**
- If 0 bps is negative for the baseline or winners → no gross edge; stop tuning this signal and move to signal/regime redesign.
- If break-even is below realistic execution costs (e.g., < ~10 bps/side after maker/limit modelling) → no viable path without execution changes.
- If there is a positive gross edge with break-even comfortably above realistic costs → proceed to Experiment 2 and execution modelling.

**Deliverables:** `out/cost-ladder/report.md`, `cost_ladder.csv`, `results.json`.

### Experiment 2 — Excursion study and exit redesign

Measure MFE/MAE and time-to-level for every signal; test trailing stops, breakeven moves, partial TP, time stops, and opposite-divergence exits. This replaces more TP/SL grid search.

### Experiment 3 — Regime and signal attribution

Split signals by higher-timeframe trend, volatility regime, session, divergence age/kind, RSI level, distance from SMA, volume, and candle structure. Identify which features separate winners from losers; pre-register a small filter set.

### Experiment 4 — Execution modelling

Compare market-at-next-open (current) with limit-at-signal-entry/retest, maker fees, and realistic fill probabilities. Include unfilled signals as no-trade.

### Experiment 5 — Portfolio and risk sizing

Volatility-targeted sizing, fixed fractional risk based on SL distance, correlation caps across BTC/ETH/SOL, and exposure limits. This improves risk-adjusted survival, not raw expectancy.

### Experiment 6 — Robustness

Longer history across regimes, more symbols (including alts and delisted), purged/embargoed walk-forward, deflated Sharpe / multiple-testing correction, second exchange, higher slippage for alts, and pooled 1day with 3–5 years.

## 3. Guardrails

- Pre-register each experiment's variants and decision rule before running.
- Keep a locked holdout per experiment; do not reuse the screening holdout for selection.
- Report uncertainty (standard error, bootstrap CI with a fixed seed).
- No production-default changes and no live switch from research results.
- If an experiment lacks sufficient trades, report `insufficient evidence` rather than the least-bad result.

## 4. Deliverables and estimates

| Experiment | Artifacts | Estimated agent time |
|---|---|---|
| 1. Cost ladder | CSV/JSON/Markdown | 2–4 h total (run 5–15 min on cache) |
| 2. Excursion | CSV/JSON/Markdown | 1–2 days |
| 3. Regime/signal attribution | CSV/JSON/Markdown | 1–2 days |
| 4. Execution model | CSV/JSON/Markdown | 1 day |
| 5. Portfolio/risk | CSV/JSON/Markdown | 1 day |
| 6. Robustness | report | 2–5 days depending on data |

Estimates are agent wall-clock including implementation, tests, review, and reporting; they exclude human review time.
