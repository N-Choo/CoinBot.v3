# Analyzer Execution Modelling — Design (Experiment 4)

- **Status:** Approved to run (Experiment 4 of the profitability research program)
- **Date:** 2026-10-22
- **Scope:** Test whether limit/maker execution can turn the thin 4hour holdout gross edge into a positive net edge under realistic fill rules.
- **Related:**
  - `docs/superpowers/specs/2026-10-22-analyzer-profitability-research-design.md`
  - `process/analyzer-rs/experiments/cost-ladder/report.md`

## 1. Question

The cost ladder showed the 4hour strategy is gross-positive in-sample but has only a thin out-of-sample gross edge: baseline holdout +0.46/trade (break-even 4.60 bps/side) and `max_age=4` +0.69/trade (break-even 6.86 bps/side). Taker costs (15 bps/side) erase it. Can limit/maker execution at ≤5–7 bps/side preserve the edge?

## 2. Models (pre-registered)

| Model | Entry | Validity | Fill buffer | Entry cost | TP cost | SL/gap/EOD cost |
|---|---|---|---:|---|---|---|
| `market` | next open | 1 candle | — | taker 15 bps | taker 15 bps | taker 15 bps |
| `limit_1c_b2` | limit at signal entry | 1 candle | 2 bps | maker 2 bps | maker 2 bps | taker 15 bps |
| `limit_2c_b2` | limit at signal entry | 2 candles | 2 bps | maker 2 bps | maker 2 bps | taker 15 bps |
| `retest_2c_b2` | limit 25 bps better than signal entry | 2 candles | 2 bps | maker 2 bps | maker 2 bps | taker 15 bps |

Focused run: these four models only.

Taker = 10 bps fee + 5 bps slippage. Maker = 2 bps fee, no slippage. All costs apply once per side on the actual notional.

## 3. Fill rules

Signal at candle `i`; limit `L`:
- Long: `L = entry * (1 - offset_bps/10_000)`, threshold `L * (1 - buffer_bps/10_000)`.
  - If `open <= threshold` → fill at `open`.
  - Else if `low <= threshold` → fill at `L`.
  - Else no fill.
- Short: `L = entry * (1 + offset_bps/10_000)`, threshold `L * (1 + buffer_bps/10_000)`.
  - If `open >= threshold` → fill at `open`.
  - Else if `high >= threshold` → fill at `L`.
  - Else no fill.
- Valid candles: `i+1 ..= i+valid_candles`; otherwise expire.
- Market: fill at `i+1` open if it exists; otherwise expire.

## 4. Exits

- Market entries may exit on the entry candle (same as the existing simulator).
- Limit entries cannot exit on the entry candle (avoids intrabar lookahead); exits start at the next candle.
- Exit checks: gap SL → gap TP → SL → TP (SL-first).
- TP is a maker exit; SL, gap, and end-of-partition are taker exits.
- At partition end, close all open positions at the last close (`eod`, taker); no open positions carry across partitions.

## 5. Accounting

- One account per symbol; `E = init_fund / 2`; available funds checked at fill; multiple signals compete in signal order.
- Gross PnL for sizing; costs deducted from PnL (net may slightly exceed the gross loss cap).
- Per-trade net = gross − entry cost − exit cost.
- Metrics reuse `evaluate::metrics_from_outcome` with a zero-cost scenario after converting to a `SimOutcome` whose `pnl` is already net (so no double charging). The report labels these as net.

## 6. Data and candidates

- Existing 24-month 4hour cache (`data/sweep`, as-of `1792661878840`).
- Same 60/20/20 splits; **validation and holdout only** (folds omitted in the focused run).
- Focused set: baseline, `joint-13`, `max_age=4` (baseline and the two configs with any holdout gross edge; `joint-13` is the SOL standout).
- Validation is the model-selection partition. Holdout is a cautionary check; it has already been scored for these candidates, so its evidential value is reduced.

## 7. Decision rule

- Compute validation and holdout for every config × model, pooled and per symbol.
- A limit model is a candidate only if:
  1. pooled validation net expectancy > 0,
  2. fill rate ≥ 50%,
  3. it beats `market` on validation net expectancy.
- It “rescues” execution only if it also has pooled holdout net expectancy > 0 and PF > 1, and is positive on ≥ 2 of 3 symbols.
- Otherwise report `execution does not rescue` and retain the baseline.
- Always report fill rates and expired orders; a high expectancy on a low fill rate is selection bias.

## 8. Deliverables

- `process/analyzer-rs/experiments/execution/execution.csv`
- `process/analyzer-rs/experiments/execution/results.json`
- `process/analyzer-rs/experiments/execution/report.md` with model definitions, validation/holdout tables, per-symbol tables, fill rates, and the decision.

## 9. Limitations

- Candle-level fill model; no order-book/queue simulation.
- Maker fills assume the limit is resting and filled when touched; adverse selection is approximated by the buffer.
- Multiple uses of the same holdout across experiments; treat holdout results as cautionary.
- Costs are provisional, not verified KuCoin fees.
