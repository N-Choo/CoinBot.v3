# analyzer

Rust RSI momentum analyzer. **Status: unvalidated — the previous RSI-divergence
strategy is closed and unprofitable.**

The strategy goes long when RSI crosses above `RSI_LONG_THRESHOLD` (default 60),
and goes short / exits the long when RSI crosses below `RSI_SHORT_THRESHOLD`
(default 50). The 60/50 band is hysteresis: RSI between the thresholds holds the
current position. Defaults are RSI(21) on 4hour candles. Positions have no
stop-loss or take-profit; the only exit is the opposite RSI cross (or manual
intervention).

Strict long/short alternation is enforced by `SignalGenerator` in
`strategy/generator.rs`, before a signal is emitted: a signal is accepted only
when it is the opposite of the last accepted side, so repeated same-side
crossings are suppressed in both the live and replay paths. The simulator and
engine keep a defensive same-side guard as well.

Research basis: buying strength (RSI above 60) with a slower RSI on 4hour bars was
the only robust RSI family in recent crypto tests; classic 30/70 mean reversion
underperformed. A 2017-2025 hourly crypto study found all RSI variants, including
momentum, underperformed buy-and-hold after costs, so this strategy is not a
profitability claim.

The crate has three layers:

- a pure deterministic core (`src/strategy/`) with no I/O, no clock, and no floats;
- an offline research harness (`src/harness/`, feature `harness`) for replay, sweeps, and
  cost/execution modelling;
- a live gRPC client (`src/live/`, features `live` + `grpc`) that pushes signals to
  `trade-engine`.

```bash
make analyzer-check   # fmt + clippy + tests (default, grpc, harness)
make analyzer-replay  # replay candles into a self-contained HTML report
make analyzer-sweep   # screen hyperparameter candidates and write artifacts
```

## Live gRPC client

The trade engine is the gRPC server and the analyzer is the client. The analyzer polls
`GetActiveSymbols` for activation, runs one task per active symbol, warms a rolling window,
recomputes `signals()` on every newly closed candle, and publishes each new signal once via
`Publish`. Each signal gets exactly one publish attempt; nothing is resent after a restart.

The crate-local `.env` holds both strategy and live settings. It is loaded from the current
directory first, then from `process/analyzer/.env` for `cargo run`. Start from the checked-in
example:

```bash
cp process/analyzer/.env.example process/analyzer/.env
# edit .env
cargo run -p analyzer --features grpc --bin analyzer-serve
```

Existing process environment variables take precedence over `.env` values. `LIVE_SYMBOLS` is
only a fallback: it is used when the engine returns an empty active-symbol set (or when the
engine is unreachable and no symbol tasks are running). `ANALYZER_ACTIVATION_POLL_MS` controls
how often activation is refreshed (default 30000).

`ANALYZER_ONCE=1` runs one closed-candle scan per symbol using the engine's active symbols (or
`LIVE_SYMBOLS` as the fallback), logs the signals, and exits:

```bash
ANALYZER_ONCE=1 \
GRPC_TRADE_ENGINE_ENDPOINT=http://127.0.0.1:50051 \
LIVE_SYMBOLS=BTC-USDT KUCOIN_TIMEFRAME=4hour \
cargo run -p analyzer --features grpc --bin analyzer-serve
```

## Replay report

`analyzer-replay` replays closed candles through the same signal engine and trade simulator used
by the parity tests, then writes a self-contained HTML report (no external CSS, JS, or network
references). Fees and slippage are excluded: the output is a signal/execution replay, not a
profitability claim.

By default it fetches live candles from KuCoin. `--data SYMBOL=path` is the
deterministic/offline override: for any symbol listed in `--data`, the local CSV is used instead
of calling KuCoin, while missing symbols are still fetched live.

```bash
cargo run -p analyzer --features harness --bin analyzer-replay -- \
  --symbol BTC-USDT --timeframe 4hour --limit 500 \
  --out process/analyzer/out/replay.html

# Offline / reproducible: fixed fixtures, cutoff, and report timestamp.
cargo run -p analyzer --features harness --bin analyzer-replay -- \
  --timeframe 1day --as-of 1800000000000 \
  --data BTC-USDT=tests/fixtures/btc_usdt_1day.csv \
  --out /tmp/replay-btc.html
```

Run from `process/analyzer` so relative fixture paths resolve; from the workspace root use
`process/analyzer/tests/fixtures/...` instead.

### Options

- `--symbol SYMBOL` (repeatable) selects symbols explicitly. When omitted, the `--data` symbols
  are used; if there are none, `BENCHMARK_TICKERS` is used.
- `--data SYMBOL=path` loads a CSV for that symbol instead of calling KuCoin. Entries are
  trimmed and deduplicated by symbol.
- `--timeframe` falls back to `KUCOIN_TIMEFRAME`, then `4hour`.
- `--limit` (default 500) is the fetch request size. Offline `--data` replays every closed row.
- `--init-fund` falls back to `BENCHMARK_INIT_FUND`, then `1000`; must be `> 0`.
- `--as-of` is the reference time in ms (default now). For network runs it sets the fetch cutoff;
  for all runs it filters by the closed-candle boundary. Pass a fixed value for reproducibility.
- `--generated-at` overrides the report timestamp.
- `--max-gap-candles` (default 0, must be `>= 0`) rejects datasets with larger gaps.
- `--out` defaults to `process/analyzer/out/replay.html`.

If at least one symbol succeeds the report is written and the process exits `0`; partial failures
are recorded in the report. If all symbols fail the process exits non-zero and writes nothing.

The strategy settings (`KUCOIN_TIMEFRAME`, `RSI_PERIOD`, `RSI_LONG_THRESHOLD`,
`RSI_SHORT_THRESHOLD`, `ANALYZER_WINDOW_CANDLES`) come from the same
`.env`; see `.env.example`:

```dotenv
KUCOIN_TIMEFRAME=4hour
RSI_PERIOD=21
RSI_LONG_THRESHOLD=60
RSI_SHORT_THRESHOLD=50
ANALYZER_WINDOW_CANDLES=100
```

`KUCOIN_BASE_URL` overrides the exchange endpoint and `ANALYZER_CANDLE_SETTLE_MS`
controls the closed-candle boundary (default `5000`).

## Current strategy screening (2026-10-23)

A consolidated index of every experiment, including negative results and the historical
RSI-divergence studies, is in [`experiments/README.md`](experiments/README.md).

[RSI momentum screening](experiments/rsi-momentum-4h/README.md) — 4hour, BTC/ETH/SOL,
12 months, all 24 RSI candidates. **Insufficient evidence**: no candidate met the
validation criteria; the baseline (RSI 21 / 60:50) had net expectancy −13.77 and profit
factor 0.09 over 34 validation trades. The report shows baseline holdout metrics, but no
candidate was frozen or selected, so no candidate holdout was scored.

[RSI tuning with strict alternation](experiments/rsi-tuning-1000c/README.md) —
4hour and 1day, BTC/BNB/XRP/SOL, last 1,000 closed candles, all 24 RSI candidates.
**Insufficient evidence**: no candidate met the validation criteria on either timeframe;
baseline holdout net expectancy was negative on both (−6.08 on 4hour, −3.81 on 1day).
The experiment recommends keeping the research baseline rather than promoting a tuned
configuration.

## Research summary (historical RSI-divergence strategy — closed)

The table below summarizes the closed RSI-divergence strategy. It does not
evaluate the current RSI momentum defaults.

Full artifacts from the previous strategy live under `experiments/`.

| Experiment                                             | Scope                                                                     | Result                                                                                                                                 |
| ------------------------------------------------------ | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| [Sweep](experiments/historical/rsi-divergence-sweep/report.md)                   | 648-point grid sampled to 30 candidates, 1hour + 4hour, base/stress costs | **Insufficient evidence** — no candidate cleared validation on every timeframe.                                                        |
| [Cost ladder](experiments/historical/rsi-divergence-cost-ladder/report.md)       | 4hour, 0–30 bps/side                                                      | Holdout gross edge exists but sits below realistic costs; best break-even **6.86 bps/side**. No viable path without execution changes. |
| [Execution modelling](experiments/historical/rsi-divergence-execution/report.md) | market vs limit/retest entries                                            | Nominal rescue for a retest-entry model, but the holdout edge is within noise and the holdout had been reused.                         |
| [Fresh confirmation](experiments/historical/rsi-divergence-confirm/report.md)    | Pre-registered one-shot, 4hour, 2022-11 → 2024-10                         | **Failed** — pooled net expectancy −2.15, PF 0.80, 0/3 symbols positive.                                                               |
