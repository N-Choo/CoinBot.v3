# Analyzer-RS Replay Report Polish — Design

- **Status:** Design approved in brainstorming; awaiting written-spec review
- **Date:** 2026-10-21
- **Scope:** Polish the `analyzer-replay` Terminal report: make TP/SL explicit in the ledger and
  open positions, label the chart levels, and make KuCoin fetch the default data source.
- **Related:**
  - `docs/superpowers/specs/2026-10-21-analyzer-replay-simulator-design.md`
  - `docs/superpowers/plans/2026-10-21-analyzer-replay-simulator.md`

---

## 1. Goals

- The ledger shows SL and TP (and R:R) per closed trade.
- Open positions show SL, TP, distance to each level, and unrealized PnL.
- The chart labels the TP/SL lines with their prices and distinguishes signal entry, fill, and exit.
- The summary reports TP hits, SL hits, and average R:R.
- The report fetches live KuCoin candles by default; `--data` remains an optional offline override
  for deterministic tests.
- Visual polish: sticky header/tabs, denser table, responsive layout, clearer TP/SL colors.

## 2. Non-goals

- No strategy, driver, or simulator behavior changes.
- No interactive server or symbol/timeframe selection inside the report.
- No payload schema break; the UI derives TP/SL from the existing signals and trades.
- No fees, slippage, or profitability claims.

## 3. Decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | Default data source is KuCoin fetch; `--data` is optional | The user wants live data; fixtures remain for deterministic tests. |
| 2 | Ledger adds `SL`, `TP`, and `R:R` columns | Levels must be visible per trade. |
| 3 | Open-position cards add `SL`, `TP`, distance %, and a TP/SL position bar | Show where price sits between the levels. |
| 4 | Chart TP/SL lines get right-edge price labels | Make levels readable, not just dashed lines. |
| 5 | Summary adds TP hits, SL hits, average R:R | Quick quality view alongside PF/drawdown. |
| 6 | No payload change required | The UI can locate a trade's originating signal by `entry_index == signal.candle_index + 1` and side; `risk_reward` already exists on trades. |
| 7 | Keep the Terminal tab layout | Approved direction; this is a polish pass. |

## 4. UI details

### 4.1 Chart

- Candles, SMA, window boundary, current-candle outline unchanged.
- TP/SL dashed lines from the signal candle to the exit (or current step) with right-edge labels
  `TP $…` and `SL $…`, teal for TP and red for SL.
- Signal marker (triangle), signal-entry tick, fill dot, exit dot.
- Legend: `▲ signal`, `● fill`, `× exit`, `┄ TP/SL`.

### 4.2 Ledger

Columns:

`Entry · Side · Signal Entry · Fill · SL · TP · R:R · Exit · Exit Price · PnL · Result`

- `SL`/`TP` come from the originating signal.
- `R:R` is the trade's `risk_reward` (from the fill); `—` when absent.
- Open rows show `open` in Exit/Exit Price and mark-to-market PnL.

### 4.3 Open positions

Cards with `Side · Fill · SL · TP · distance to SL/TP % · unrealized`, plus a horizontal bar
showing the current price between SL and TP.

### 4.4 Event stream

- Entry events include `SL`/`TP`.
- Signal and exit events unchanged (signal already shows SL/TP).

### 4.5 Summary

Add `TP hits`, `SL hits`, and `avg R:R` to the existing summary (equity, realized, profit factor,
max drawdown, exposure, trades, win rate).

### 4.6 Polish

- Sticky header and tab bar.
- Denser monospace table; numbers tabular-aligned.
- Responsive: table scrolls horizontally on narrow widths; cards stack.
- TP teal / SL red consistent with the chart.

## 5. Data source

- Default: `analyzer-replay` without `--data` fetches closed candles from KuCoin using the existing
  `fetch_klines` with the `--as-of` cutoff.
- `--data SYMBOL=path.csv` remains an optional offline override for deterministic runs and tests.
- README examples use the live-fetch form; the offline form is documented as the deterministic
  option.

## 6. Testing

- Template assertions: ledger header contains `SL`, `TP`, `R:R`; the chart legend contains
  `TP/SL`; the summary contains `TP hits`/`SL hits`.
- Existing payload/report tests continue to pass (no schema change).
- Headless Chrome render: no JS errors; the ledger shows SL/TP columns after stepping to a closed
  trade.
- CLI live smoke (network, optional in CI): a run without `--data` writes a report; if KuCoin is
  unreachable, the run records the failure and exits non-zero only when no symbol succeeds.

## 7. Acceptance

- `analyzer-replay --symbol BTC-USDT --timeframe 15min --limit 500 --out out/replay.html` (no
  `--data`) fetches KuCoin and writes a report.
- The report's ledger shows `SL`, `TP`, and `R:R`; open positions show both levels and distances;
  the chart labels TP/SL.
- `make analyzer-rs-check` passes and the headless render has no JS errors.
