# Analyzer-RS Replay Report Polish — Design

- **Status:** Design revised per review; awaiting written-spec review
- **Date:** 2026-10-21
- **Scope:** Polish the `analyzer-replay` Terminal report: make TP/SL explicit in the ledger and open
  positions, label the chart levels, precompute per-step account state, and make KuCoin fetch the
  default data source.
- **Related:**
  - `docs/superpowers/specs/2026-10-21-analyzer-replay-simulator-design.md`
  - `docs/superpowers/plans/2026-10-21-analyzer-replay-simulator.md`

---

## 1. Goals

- The ledger shows SL, TP, and planned R:R per closed trade, taken from the **originating signal**
  identified by a stable `signal_id`.
- Open positions show SL, TP, absolute percentage distance to each level, direction-aware labels,
  and the precomputed unrealized PnL.
- The chart labels TP/SL lines with prices; lines start at the **fill** candle and end at the exit
  or the current step.
- The summary reports TP hits, SL hits, other exits, and average planned R:R, all from the
  simulator's exit classification.
- Per-step account state (equity, available, reserved, unrealized, open-position unrealized) is
  **precomputed in Rust** and embedded, so the UI is a read-only view of replay output.
- KuCoin fetch is the default data source; `--data` remains an optional offline override.
- Visual polish: sticky header/tabs, denser table, responsive layout, consistent TP/SL colors.

## 2. Non-goals

- No strategy, driver, or signal-generation changes.
- No interactive server or in-report symbol/timeframe selection.
- No fees, slippage, or profitability claims.
- No UI inference of signals or exit events from candle prices.

## 3. Decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | Default data source is KuCoin fetch; `--data` bypasses the network for that symbol | Live data by default, deterministic offline tests. |
| 2 | Trades and open positions carry a stable `signal_id` | Unambiguous signal attribution; no index/side guessing. |
| 3 | TP/SL hits are counted from the simulator's exit reason | `TP`/`gap_tp` = TP hit, `SL`/`gap_sl` = SL hit; all other exits are `other`. |
| 4 | `avg planned R:R` = arithmetic mean of eligible closed trades' `risk_reward` | Explicit method; absent/non-finite values excluded. |
| 5 | Open-position distance = `abs((price - level) / price) * 100`, direction-aware label | Defined denominator and sign. |
| 6 | Chart TP/SL lines start at the fill candle, end at exit or current step | Levels are active only after the position exists. |
| 7 | Per-step account state is precomputed in Rust | The UI must not recompute strategy/account math. |
| 8 | Ledger/Result uses the simulator's exit reason | No UI-derived TP/SL guess. |
| 9 | Keep the Terminal tab layout | Approved direction; polish pass. |

## 4. Payload additions (no breaking changes)

Per symbol:

- `trades[]`: add `signal_id`; keep `reason` (`TP`/`SL`/`gap_tp`/`gap_sl`), `risk_reward`,
  `signal_entry`, `fill`, `exit`.
- `open_positions[]`: add `signal_id`, `unrealized`.
- `account[]`: one entry per replayed candle (index `i`):
  `{ i, available_fund, reserved, unrealized, equity, open: [{ signal_id, unrealized }] }`.
- `summary`: add `tp_hits`, `sl_hits`, `other_exits`, `avg_risk_reward`.
- `signals[]`: unchanged (already has `signal_id`, `signal_entry`, `sl`, `tp`).

`signal_id` is already computed by `identity::signal_id` and carried by `ReplaySignal`; the
simulator propagates it onto `Position`/`ClosedTrade`.

## 5. UI details

### 5.1 Price roles (explicit)

| Price | Meaning |
|---|---|
| Signal entry | Entry proposed by the analyzer (`signals[].signal_entry`) |
| Fill | Actual simulated entry (next candle open) |
| Exit | Actual simulated exit price |

### 5.2 Chart

- Candles, SMA, window boundary, current-candle outline unchanged.
- TP/SL dashed lines per trade/open position: start at the **fill** candle, end at the exit candle
  (or the current step while open); teal for TP, red for SL; right-edge labels `TP $…` / `SL $…`.
- Labels are clamped inside the plot and vertically staggered when they would overlap.
- If a level is outside the visible price range, the line is still drawn and the label is pinned to
  the nearest plot edge with an arrow.
- Markers: `▲/▼` signal, a small tick at signal entry, `●` fill, `×` exit.
- Legend: `▲ signal`, `● fill`, `× exit`, `┄ planned TP/SL (from the signal)`.
- Multiple overlapping trades each draw their own lines; no merging of levels.

### 5.3 Ledger

Columns: `Entry · Side · Signal Entry · Fill · SL · TP · R:R · Exit · Exit Price · PnL · Result`.

- `SL`/`TP` come from the signal whose `signal_id` matches the trade.
- `R:R` is the trade's `risk_reward`; `—` when absent.
- `Exit` is the actual exit reason for closed trades; `Open` for open rows.
- `Exit Price` is the actual exit fill; `—` for open rows.
- `PnL` is realized for closed trades and the precomputed unrealized value for open rows.
- `Result` is the simulator's reason; no UI-derived classification.

### 5.4 Open positions

Cards per open position:

- `Side · Fill · SL · TP · unrealized`
- distance to SL and TP as `abs((price - level) / price) * 100` with an `above`/`below` label
  relative to the level
- a direction-aware bar showing the current price between SL and TP; if price is beyond a level,
  the bar shows `beyond TP` / `beyond SL` instead of clamping to 100%; if a level is missing, the
  bar shows `—`.

Distance, unrealized, and the bar are three distinct values; a profitable position can still be
closer to its stop than its target.

### 5.5 Event stream

- Entry events include `SL`/`TP`.
- Signal and exit events unchanged.
- Event order follows the simulator: fill at open → exit → signal at close.

### 5.6 Summary

- Existing: equity, realized, unrealized, profit factor, max drawdown, exposure, trades, win rate.
- Added: `TP hits` (`reason` in `TP`/`gap_tp`), `SL hits` (`SL`/`gap_sl`), `other exits`
  (anything else, e.g. boundary), `avg planned R:R` (mean of eligible closed `risk_reward`).
- Win rate and profit factor definitions are unchanged.
- The banner keeps: `signals live-exact · trades simulated · fees excluded · not a profitability
  claim`.

### 5.7 Polish

- Sticky header and tab bar.
- Denser monospace table, tabular numbers.
- Responsive: ledger scrolls horizontally on narrow widths; cards stack.
- TP teal / SL red consistent across chart, ledger, and cards.

## 6. Data source behavior

- **Default:** no `--data` → fetch closed candles from KuCoin via the existing `fetch_klines`,
  anchored at `--as-of` (default now), excluding the forming candle.
- **Offline:** `--data SYMBOL=path.csv` bypasses the network for that symbol; tests never fall back
  to live data.
- **Partial failures:** successful symbols render; failures are listed as disabled tabs. Exit `0`
  when at least one symbol succeeds; non-zero when none succeed, with a clear stderr message and no
  empty report.
- **Limit:** `--limit` requests that many replay candles plus warm-up. If the exchange returns
  fewer, the report records the effective count in `provenance.effective_range.candles`; the run
  still succeeds when the `W + 2` minimum is met.
- **Reproducibility:** live fetches are not inherently reproducible (exchange data can be corrected
  and fetch times differ). Deterministic runs use `--data` with `--as-of` and `--generated-at`.
- **Read-only view:** the UI formats and aggregates precomputed replay/account data; it does not
  recompute indicators, signals, exits, or account math.

## 7. Testing

Required tests before implementation:

- **Signal attribution:** a trade/open position's `signal_id` resolves to the correct signal,
  including two signals on the same candle/side in a synthetic payload.
- **Long and short:** SL/TP, distances, labels, and bar orientation for both directions.
- **Open position:** no exit price; both levels, unrealized, and distances from `account[]`.
- **Exit classification:** `TP`/`gap_tp` counted as TP hits, `SL`/`gap_sl` as SL hits, boundary and
  other reasons as `other_exits`; open trades excluded.
- **Average R:R:** mean over eligible trades; absent/non-finite values excluded.
- **Stepping:** at each step, chart lines, ledger, open positions, and summary read the matching
  `account[i]` and produce consistent values.
- **Deterministic fixture:** same input + fixed `--as-of`/`--generated-at` → identical report.
- **Live fetch failures:** all-failed and partially successful multi-symbol runs follow the exit
  policy.
- **Browser render:** no JS errors; narrow layout, overlapping chart labels, and table scroll.
- **Regression:** existing live/grpc and signal-parity suites still pass.

## 8. Acceptance

- `analyzer-replay --symbol BTC-USDT --timeframe 15min --limit 500 --out out/replay.html` (no
  `--data`) fetches KuCoin and writes a report when at least one symbol succeeds.
- Ledger shows SL/TP from the matching `signal_id` and R:R from the trade.
- TP/SL hit counts match the simulator's exit classifications; other exits are separate.
- Open-position distances and the bar are correct for long and short, including prices outside the
  planned range.
- All-failed runs exit non-zero with a clear error and no empty report.
- `make analyzer-rs-check` passes and the headless render has no JS errors.
