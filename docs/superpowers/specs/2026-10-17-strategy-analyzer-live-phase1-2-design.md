# Strategy Analyzer — Live Integration (Phase 1 Shadow / Phase 2 Publish) — DRAFT

- **Status:** DRAFT — **gated on the Phase 0 backtest verdict.** Do not start implementation until the Phase 0 out-of-sample bar passes (spec §10.4). If Phase 0 fails, this is not built.
- **Date:** 2026-10-17
- **Depends on:** `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md` (Phase 0), `docs/superpowers/plans/2026-10-17-strategy-analyzer-phase0.md`.
- **Supersedes:** the transport deferral in Phase 0 §12 for the items this document promotes.

---

## 1. Objective

Run the already-built, deterministic strategy core **live**, per ticker, and (Phase 2) publish signals to the existing trade engine using the **existing contract** (`TradeSignal` on Redis `signals:result`).

Integration shape is the **repo's existing engine-triggered request/response** (not the v2 "the engine must not request" rule):

```
trade-engine --PUB tickers:analyze--> strategy-analyzer         (activate/refresh symbols)
strategy-analyzer --(Phase 2) PUB signals:result--> trade-engine (Vec<TradeSignal>)
```

The analyzer **owns a continuous per-ticker loop** and **persists per-ticker state**. The engine message only registers/refreshes *which* symbols to run; it never resets state.

**Decision (adopted):** the Rust analyzer **replaces** the Python `process/analyzer/` on this path (same channels), because Option A targets the *same* `TradeSignal` contract on `signals:result`; coexistence there would double-publish.

---

## 2. Why a continuous loop (not recompute-per-trigger)

The core is a **state machine** per symbol (`StrategyState`: rolling window, running ATR, trend, pivots, zones with lifecycle, `zones_awaiting_first_tick`, `last_price`). Its guarantees — **first-touch-only**, **one attempt per zone**, **missed-touch**, **no duplicate signals across restarts** — depend on carrying state forward. Recomputing from a fixed window on each engine trigger would re-fire already-consumed zones and violate "first touch only." The engine's ~24h trigger is also far coarser than the 4H boundaries/tick touches the strategy needs, so the analyzer must run its own loop; the trigger is just activation.

---

## 3. Components

- **Per-ticker loop (task):** one independent task per symbol (matches the repo's `tokio::spawn` style), each owning its own `StrategyState`. Advances state on each new closed 4H candle (`Input::ClosedCandle`) and each price poll (`Input::PriceTick`); emits only the core's `EntrySignal`/`ZoneEvent`/`StateChange` outputs.
- **Shared price poller (one):** polls `GET /api/v1/market/allTickers` (one call, all symbols, ~2s snapshot) and **fans out** each symbol's `last` (or last/bid/ask) to its loop over a channel. (Verified: the endpoint exists and returns `last`, `buy`/`sell` + sizes; its `high`/`low` are 24h, **not** per-poll wicks.)
- **Per-symbol candle poller:** fetches recent 4H candles (REST, the `fetch-data` path) to (re)warm and to detect new closed 4H candles; shares the global rate limiter.
- **State store:** file-per-symbol (Phase 1); see §4.
- **Supervisor:** consumes `tickers:analyze` and spawns/refreshes per-symbol tasks; owns activation set and pruning.
- **Publisher (Phase 2 only):** maps `Signal → TradeSignal` and PUBs to `signals:result`, behind a single cutover flag. **Phase 1 publishes nothing** to `signals:result` (log only, or a separate shadow channel).
- **Recorded input stream (Phase 1):** every symbol's `(ClosedCandle | PriceTick)` inputs are appended to a per-symbol log so replay-equivalence can be checked live-vs-replay (the core's stated property).

---

## 4. File persistence (Phase 1) — required care

State directory must be a **mounted Docker volume** (compose), or every container restart wipes state and silently re-warms each symbol.

- **Atomic writes:** write to a temp file → `fsync(file)` → `rename` over the target → `fsync(dir)`.
- **Persist triggers:** after **every candle close**, and after **every state change that matters** (a zone consumed/terminal). Not on a timer.
- **Single writer:** a **lock file per instance**; two analyzers must not write the same state.
- **Validation / re-warm:** a state file that is missing, **corrupt**, **schema-mismatched** (`STATE_SCHEMA_VERSION`), or **older than a threshold** (more than a few candles behind) is **discarded and re-warmed from history**. Never trust an unvalidated state file. **After any re-warm, run the missed-touch rule on every currently `Active` zone**, so a restart cannot emit a historical touch.
- **Pruning:** delete state for symbols inactive past their TTL.

---

## 5. Design points (recorded)

- **Shared price poller + jitter:** avoid harvesting one call per symbol per poll; use the single bulk poller. Add **per-symbol jitter** in the shared limiter so the 4H candle-boundary burst (all symbols at once) doesn't trip rate limits.
- **Per-task failure isolation:** a symbol whose candles keep erroring (e.g. `MalformedCandle`) is **quarantined with an alert**; a panicking task must **not** crash-loop by restarting and replaying the same bad data.
- **Phase 1 channel:** shadow loops **log only** (or a separate shadow channel) — **never** `signals:result`. The `signals:result` publisher is added in Phase 2 behind **one cutover flag**.
- **First-tick:** already handled in the core (`observe_first_tick` resolves **all** awaiting zones on the first observed price, `signals.rs:316-342`) — a required precondition for live warm-up of many symbols, verified present.
- **Observations to record:** per symbol, log the **polled price** alongside the **latest 1m candle's high/low**, so Phase 1 can quantify how many wicks the polling misses.
- **Latency/staleness:** no websockets exist; price freshness is bounded by the poll interval. Staleness handling (drop/last-resort) is a Phase-1 runtime concern; the engine's `level1` polling is ~12s today.

---

## 6. Sequence and gating

1. **Phase 0 verdict passes** (§10.4). Hard gate — nothing here is built otherwise.
2. Phase 1 — **shadow**: per-ticker loops + shared poller + file state + recorded input stream; **log only**, no publish, no engine. Exit: live-vs-replay equivalence on the recorded stream; data-gap/restart behaviour verified (re-warm + missed-touch on active zones); wick-miss quantification recorded.
3. Phase 2 — **publish** (see outline). Exit: engine paper-mode integration; dedup/idempotency verified; replacement of the Python path behind the cutover flag.

---

## 7. Phase 2 outline (not committed until Phase 1 passes)

- Publish `Signal → TradeSignal` to `signals:result`; **D-1 engine-side dedup** on a stable `signal_id` (the engine currently has **none**).
- State store moves to **Postgres or Redis**, owned by a defined migrator (the trade-engine doesn't run migrations today).
- Engine consumer changes: parse the payload, dedup, apply risk. Note the engine currently enforces only a fund gate + `cap_loss`; daily-DD/open-risk/correlation/net-R:R **do not exist** and are prerequisite work.
- Cutover: feature flag switching the Python analyzer off and the Rust analyzer on for `signals:result`.
- `Clock`/`EngineAck` core inputs, re-arm policy, signal TTL.

---

## 8. Open items

- §6.3 valid-high/low human verification (video 00:29–04:30) — still open from Phase 0.
- Phase 0 OOS bar outcome — gates this entire document.
- Data store choice for Phase 2 (Postgres vs Redis) and its migration owner.
- Whether `OneMinuteExpansion` chunking (currently by count, `tick_source.rs`) should be time-bucketed before any 1m headline use (Phase 0 flagged).
