# Benchmark Replay Report — Design

**Date:** 2026-10-20
**Status:** Approved for planning
**Scope:** A local, self-contained HTML viewer that replays the analyzer benchmark candle by candle.

## Problem

The analyzer benchmark (`process/analyzer/benchmark/main.py`) prints a text ledger after
a backtest. It proves the outcome but hides the reasoning: which candle produced a
divergence, when the signal fired, how much later the entry occurred, and how equity
evolved while the position was open. Investigating a surprising result means reading
code or adding ad-hoc prints.

## Goal

Turn the benchmark into a single self-contained HTML report that replays the exact
causal sequence candle by candle, showing price, SMA, RSI, divergences, signals,
entries/exits, TP/SL levels, account state, and the trade ledger.

## Non-goals

- No live trading, order placement, or Redis interaction.
- No new runtime dependency and no external CDN/font/network request in the report.
- No change to the strategy, signal rules, or trade-engine math.
- Not a deployed feature; this is a local dev/inspection tool.

## Decisions

1. **Delivery:** `make benchmark-web` writes one HTML file with embedded JSON. Open it
   directly; no server.
2. **Location:** local dev tool under `process/analyzer/benchmark/`.
3. **Content:** candles + SMA, RSI subplot with divergence markers, entry/exit markers,
   TP/SL lines, account state, and ledger.
4. **Layout:** "Timeline cockpit" — full-bleed price chart, RSI below, floating HUD,
   bottom timeline with trade dots and scrub.

## Architecture

```
make benchmark-web
  └─ python -m analyzer.benchmark.main --web
       ├─ load .env / Config
       ├─ for each BENCHMARK_TICKERS symbol:
       │    ├─ fetch_klines()                     (existing)
       │    └─ run_benchmark(..., recorder=...)   (existing loop, observed)
       ├─ build_ticker_replay()                   -> JSON-safe dict
       ├─ render_report()                         -> replace template placeholder
       └─ write out/benchmark.html
```

The replay loop remains the single source of truth. The recorder observes it; it does
not re-run or re-implement signal logic. `run_benchmark` calls `recorder.begin(start, df)`
before the loop and `recorder.finish(engine)` after it, so the recorder never recomputes
the window boundary.

### Files

| File | Change |
| --- | --- |
| `process/analyzer/models/types.py` | Add optional `divergence_idx: int \| None = None` to `Signal`. Do **not** add it to `to_dict()`, so the live Redis payload is unchanged. |
| `process/analyzer/signals/generator.py` | Set `divergence_idx=best[0]` on the returned `Signal`. No behavioral change. |
| `process/analyzer/benchmark/replay.py` | `ReplayRecorder`, `build_ticker_replay()`, indicator/candle assembly, JSON sanitizing. |
| `process/analyzer/benchmark/report.py` | `render_report()`, `write_report()`, `<` escaping (`\u003c`). |
| `process/analyzer/benchmark/replay_template.html` | Self-contained viewer: CSS, SVG renderer, controls. |
| `process/analyzer/benchmark/main.py` | Optional `recorder` argument on `run_benchmark`; `--web`, `--out`; one report per run. |
| `process/analyzer/benchmark/tests/test_replay.py` | Recorder and data-contract tests. |
| `process/analyzer/benchmark/tests/test_report.py` | Rendering and escaping tests. |
| `Makefile` | `benchmark-web` target. |
| `.gitignore` | Ignore `process/analyzer/benchmark/out/`. |
| `process/analyzer/README.md` | Usage and report description. |

## Replay data contract

One report contains all successfully fetched tickers plus any failures:

```jsonc
{
  "generated_at": "2026-10-20T..",
  "tickers": [ /* one entry per successful ticker */ ],
  "failures": [ { "symbol": "SOL-USDT", "error": "KuCoinFetchError: ..." } ]
}
```

Each ticker entry:

```jsonc
{
  "ticker": "BTC-USDT",
  "config": {
    "timeframe": "1day",
    "rsi_period": 14,
    "rsi_order": 8,
    "sma_enabled": true,
    "sma_period": 200,
    "atr_period": 14,
    "sl_mult": 2.8,
    "tp_mult": 3.5,
    "init_fund": 1000.0,
    "fee_rate": 0.001,
    "benchmark_days": 360
  },
  "warmup": 200,
  "start_index": 360,
  "candles": [
    { "t": "2026-05-01", "o": 88400.0, "h": 89900.0, "l": 87900.0, "c": 89100.0 }
  ],
  "rsi": [54.2, 56.1],
  "sma": [87210.5, 87400.2],
  "divergences": [
    { "i": 398, "detected_at": 406, "kind": "regular_bull", "side": "buy" }
  ],
  "signals": [
    {
      "i": 406,
      "action": "buy",
      "confidence": 0.6,
      "reason": "regular_bullish_divergence",
      "entry": 89100.0,
      "tp": 97200.0,
      "sl": 84900.0,
      "opened": true,
      "rejected": null
    }
  ],
  "events": [
    {
      "i": 406,
      "exits": [
        { "side": "buy", "reason": "TP", "price": 96000.0, "pnl": 52.1, "pnl_pct": 10.4, "entry_i": 360, "exit_i": 406 }
      ],
      "signal": { "action": "buy", "opened": true },
      "equity": 1024.0,
      "realized": 24.0,
      "unrealized": 0.0,
      "available": 500.0,
      "open": 1
    }
  ],
  "trades": [
    { "side": "buy", "entry_i": 360, "exit_i": 406, "entry_date": "..", "exit_date": "..",
      "entry": 88000.0, "exit": 96000.0, "pnl": 52.1, "pnl_pct": 10.4, "reason": "TP" }
  ],
  "open_positions": [
    { "side": "buy", "entry_i": 406, "entry_date": "..", "entry": 89100.0,
      "tp": 97200.0, "sl": 84900.0, "E": 500.0 }
  ],
  "summary": { "trades": 2, "wins": 1, "win_rate": 50.0, "realized_pnl": 0.0,
               "unrealized_pnl": 24.0, "equity": 1024.0, "open_count": 1,
               "init_fund": 1000.0, "available_fund": 500.0 }
}
```

Rules:

- `candles`, `rsi`, `sma`, `events` are aligned by position and cover only the replay
  window (`start_index .. len(df)-1`).
- All `i` fields (`i`, `detected_at`, `entry_i`, `exit_i`) are absolute DataFrame
  positions. The viewer maps them to window-relative x coordinates with
  `i - start_index`.
- `signals[].i` and `events[].i` are the candle on which `generate_signals` was
  evaluated (the detection candle). `divergences[].i` is the pivot candle and
  `detected_at` is the candle on which that pivot became confirmed/actionable.
- Divergences whose pivot precedes `start_index` are omitted from `divergences`
  (they cannot be drawn); their signal still appears in `signals`.
- `divergences` is deduplicated by `(i, kind)` because a persistent divergence can be
  selected on several consecutive candles. `signals` is not deduplicated: it records
  what the strategy actually evaluated each candle.
- `sma` is `null` while the SMA window is not yet full; `rsi` may be `null` early.
- `divergence_idx` comes from `generate_signals`; if it is `None` (e.g. a test stub),
  no divergence marker is emitted.
- Any non-finite float becomes `null` before serialization; `json.dumps` must not emit
  `NaN`/`Infinity`.

## Causal correctness

- Signals are still produced by `generate_signals(df.iloc[: i + 1])`. The report must
  never evaluate indicators over future candles to decide a signal.
- Per candle, the recorder preserves engine order:
  1. `engine.on_candle(i, high, low)` — close any TP/SL hit.
  2. `generate_signals(...)`.
  3. `engine.on_signal(...)` — optionally open at the candle close.
- A position entered on candle `i` is not eligible to exit until candle `i + 1`
  (`SimEngine` already enforces `pos.entry_idx >= idx`).
- Full-series RSI/SMA are precomputed once only because they are causal rolling
  indicators; they are sliced, never used to alter signals.
- Divergence detection remains per-candle. The pivot index (`divergence_idx`) may be
  older than the detection candle; the viewer draws the dot at the pivot and the
  signal/entry at the detection candle.
- Chart y-axis scaling may use the whole window for stability. This is presentation
  only and does not affect signals or trades.

## Viewer specification (layout C)

**Data embedding:** the JSON lives in
`<script id="replay-data" type="application/json">…</script>`. All `<` characters are
escaped as `\u003c` (not just `</`), so no ticker or error string can close the tag.

**Frame:** dark theme matching the React app: background `#0a0c0f`, panel `#0b0e11`,
orange accent `#ff5722`, up `#1dcd9f`, down `#ff4d4d`, SMA `#f59e0b`, RSI `#a78bfa`,
muted text `#64748b` / `#94a3b8`.

**Top bar**

- Ticker tabs (only succeeds; failed tickers produce a banner row).
- Date selector label; report generated timestamp.
- If the run has failures, a dismissible warning strip listing symbol + reason.

**Price pane**

- Candlesticks for candles `0 .. step`, fixed x domain for the whole window.
- SMA line up to `step`.
- Entry marker: up triangle below the candle (buy) / above (sell), colored by side.
- Exit marker: `×` at the exit candle, colored by TP/SL outcome.
- TP/SL: dashed horizontal lines from the entry candle to the current replay candle
  while the position is open; once it closes, they end at the exit candle. They never
  extend into unrevealed future candles. Color-coded; label `TP` / `SL`.
- Crosshair + tooltip on hover/focus (date, O/H/L/C).
- Current candle outlined.

**RSI pane**

- RSI line up to `step`, shaded 30/70 bands.
- Divergence dots only when `detected_at <= step`; dot at `divergence_idx` with a
  short label (`reg bull`, `hid bear`, etc.).
- Current RSI value in the pane header.

**Floating HUD (top-left of price pane)**

- Ticker + date of current candle.
- OHLC and day change.
- Current signal: action, confidence, reason, entry/TP/SL, or `no signal`.
- Account: equity, realized, unrealized, open count, available.
- Last closed trade (if any).
- `Ledger` button opens the drawer.

**Bottom timeline**

- Transport: reset, play/pause, step back/forward, jump to end.
- Speed: 0.5x / 1x / 2x / 4x / 8x (default 1x; ~400 ms per candle at 1x).
- Progress track with click/drag scrubbing.
- Trade dots: green = entry, red = exit; click a dot to jump to that candle.
- Prev/next trade buttons for inspection.
- Counter: `candle 14 / 90`.

**Ledger drawer**

- Trades closed up to `step`, newest first, with side, entry → exit date/price,
  PnL%, PnL$, TP/SL.
- Open positions with entry, TP, SL, committed fund.
- Summary line: trades, win rate, realized, equity vs init.

**Keyboard**

- `Space` play/pause, `←`/`→` step, `Shift+←/→` prev/next trade, `Home`/`End`.

**Behavior**

- Progressive reveal: candles appear as the replay advances; nothing beyond `step` is
  drawn.
- Axes are fixed to the replay window; y domain is padded ~5%.
- State resets when switching tickers.
- The report works offline and has no build step.

## Error handling

- Fetch failure for a ticker: log, record `{symbol, error}`, continue with others.
- Invalid/non-positive TP or SL: existing skip remains; the signal is recorded with
  `opened: false` and `rejected: "invalid_tp_sl"`.
- Insufficient funds: `opened: false`; the HUD still shows the signal.
- No successful tickers: still write the HTML with a full-page empty state listing the
  failures. `main()` must not raise (preserves existing test behavior).
- Output directory is created if missing; file write errors propagate (real failure).

## Testing

- `test_replay.py`
  - exit event precedes entry event on the same candle;
  - a position never exits on its entry candle;
  - `events` length equals replay-window length;
  - summary equity equals init + realized + unrealized at the final step;
  - every opened signal corresponds to a trade or an open position;
  - JSON output contains no `NaN`/`Infinity`;
  - a `Signal` without `divergence_idx` yields no divergence marker.
- `test_report.py`
  - rendered output starts with `<!DOCTYPE html>` and embeds parseable JSON;
  - `</script>` in data cannot break out;
  - `write_report` creates parent directories and writes the file.
- Existing `benchmark/tests` and full `make analyzer-test` must pass.
- Manual smoke: build a report from a synthetic DataFrame (no network) and open it.

## Usage

```bash
# text ledger (unchanged)
make benchmark

# self-contained HTML replay
make benchmark-web
# prints the absolute path, e.g.
#   report written to process/analyzer/benchmark/out/benchmark.html

# custom output
cd process && python3 -m analyzer.benchmark.main --web --out /tmp/replay.html
```

## Future ideas (out of scope)

- Include trend-filtered signals with the reason they were dropped.
- Compare multiple tickers on one timeline.
- Export a compact JSON for embedding elsewhere.
- Optionally auto-open the report with `--open`.
