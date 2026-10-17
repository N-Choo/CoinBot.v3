# Strategy Analyzer — Live Integration (Phase 1 Shadow / Phase 2 Publish) — DRAFT v4

- **Status:** DRAFT — **gated on the Phase 0 backtest verdict.** Nothing here is built until the Phase 0 out-of-sample bar passes (Phase 0 spec §10.4).
- **Date:** 2026-10-17
- **Depends on:** `2026-10-17-rust-strategy-analyzer-design.md`, `2026-10-17-strategy-analyzer-phase0.md`.
- **Numbers:** anything marked *(proposed)* is a starting value, not a repo-fixed constant.

**Verified this session (read from the repo):**
- `process/proto/analyzer.proto` exists; service `AnalyzerService` with unary `rpc Analyze(AnalyzeRequest) returns (AnalyzeResponse)`; `AnalyzeRequest{ repeated string tickers }`; `Signal{ string ticker; string action; double confidence; double entry_price }`; `AnalyzeResponse{ repeated Signal signals }`. Compiled by `share/build.rs` and re-exported (`pub use analyzer::*`).
- `trade-engine` has **no `tonic` dependency** in its `Cargo.toml` (only `share` does). The tonic server pattern to follow is `process/deposit-worker/src/grpc_handler.rs`.
- The engine consumes `TradeSignal` (`lib/share/src/models/signals.rs`: `{ticker, action, confidence: f64, entry_price: f64, sl_price: f64, tp_price: f64, reason}`) and **honors SL/TP** (`trader/mod.rs:69-79` validates them; `position_exit/mod.rs:88` uses `entry.tp_price`/`entry.sl_price`). It rounds **quantity** to `base_increment` (`sizing.rs`) but does **not** quantize `entry_price`/`sl_price`/`tp_price` (no `priceIncrement` fetch).
- `confidence` is **not gated** by the engine (used only in `TradeSignal::log`).
- No generated `analyzer::Signal` struct literals exist in the workspace today (grep clean).

**External docs (cited):** KuCoin "Get All Tickers" (`GET /api/v1/market/allTickers`, snapshot ~2s; `last`/`buy`/`sell`; `high`/`low` are 24h) — `https://www.kucoin.com/docs-new/rest/spot-trading/market-data/get-all-tickers`; "Rate Limit Rule (Classic)" (Public pool, VIP0 = 2000/30s) — `https://www.kucoin.com/docs-new/rest/rate-limit-rule-classic`; `/api/v2/symbols` (`priceIncrement`); `/api/v1/timestamp`.

**Unverified (not read this session — `unverified`):** the engine's health/alerting wiring (whether any alert hook exists), the engine's exact contract-signature check flow, and any existing signal dedup. Treat engine-side items in §8 as requirements to confirm against the engine code.

---

## 1. Objective / integration shape

Run the deterministic core live, per ticker, and serve signals to the trade engine over **gRPC** via the **existing** `AnalyzerService` (`process/proto/analyzer.proto`). Keep the RPC **unary**. The Rust analyzer **replaces** the Python `process/analyzer/`.

```
trade-engine --gRPC Analyze(tickers, after_seq)--> strategy-analyzer   (unary)
strategy-analyzer --AnalyzeResponse{signals: seq > after_seq}--> trade-engine
```

Integration is a **sequence-acknowledged unary pull** (§6): the engine calls `Analyze(tickers, after_seq)` every **5–10 s (proposed)** with its full ticker list; the analyzer returns every signal with `seq > after_seq`, ascending; advancing `after_seq` acknowledges them. The call is also the activation (§3c). Per-ticker loops run continuously and own per-ticker state; the RPC never resets it. The analyzer implements the tonic **server**; the engine gains a tonic **client** (it has none today).

## 2. Why a continuous loop

The core is a per-symbol **state machine**; its guarantees (first-touch-only, missed-touch, no duplicate signals across restarts) require carrying state forward. The engine's call cadence (5–10 s) does not define the strategy's clock — 4H boundaries and tick touches do — so the analyzer runs its own loop; the call only activates.

## 3. Components

- **Per-ticker loop (task):** one independent task per symbol; owns its `StrategyState`; advances on each closed 4H candle and each price poll. Lifetime: §5a.
- **Shared price poller (one):** `allTickers`, fan out per-symbol `last`.
- **Per-symbol candle poller:** REST 4H candles; shares the global limiter.
- **Symbol metadata / clock:** `priceIncrement` from `/api/v2/symbols`; time from `/api/v1/timestamp`.
- **State store:** file-per-symbol, plus the **global `seq`** (§6); §4.
- **Supervisor:** serves gRPC `Analyze`; spawns/refreshes tasks from the requested ticker set; owns activation, TTL, pruning.
- **gRPC service (Phase 2 only):** tonic server implementing `AnalyzerService`; maps `Signal → proto Signal` (§7) behind one cutover flag. **Phase 1 has no gRPC server and publishes nothing** — it logs only.
- **Recorded input stream (Phase 1):** append each symbol's `(ClosedCandle | PriceTick)` inputs for live-vs-replay equivalence.

### 3a. Candle-close rule (adapter-owned; the core has no clock)

A 4H candle is closed only when `open_time + 4h + settle <= exchange_now`, `settle = 5 s (proposed)`. `exchange_now = local monotonic clock + an offset measured against KuCoin's time endpoint and refreshed periodically`. Fetches drop any row not yet closed. A hole in the series triggers an **in-order backfill** before any candle is fed. If the exchange revises an already-processed candle, **alert and re-warm** (the core cannot amend history). **Alert when a closed candle is > 5 min late (proposed).**

### 3b. Config scope

`config_version` hashes **strategy parameters only**. Per-symbol `price_scale` and `price_increment` are **instance data**, excluded from the hash (`Config::canonical_params_json()` omits them), so adding a ticker does not change the version or invalidate the freeze lock.

A dp `price_scale` alone cannot express a step-5/step-25 grid (`priceIncrement` can be `0.0005` at 4 dp), so the **core carries each symbol's `price_increment` and snaps** `stop_loss`/`take_profit`/reference levels to valid multiples — stop **away from** entry, take-profit **toward** entry — before publishing. Fallback: dp `round_price` when no increment is configured.

### 3c. Activation

There is no `tickers:analyze` message; the **`Analyze(tickers, after_seq)` call is the activation** — each call refreshes the active ticker set. One call carries all symbols; one reply can contain signals for several.
- **`activation_ttl = 5 min (proposed)`** (replaces the earlier 72 h, which was sized for the coarse 24 h Redis trigger). A symbol **not mentioned for `activation_ttl` is deactivated** (loop stops; state retained until pruning).
- **Re-add after a long gap is a cold start:** re-warm from candle history, then apply the restart/gap rule (§4).

## 4. Persistence and restart/gap safety (Phase 1)

State directory is a **mounted Docker volume** (compose).
- **Atomic writes:** temp → `fsync(file)` → `rename` → `fsync(dir)`.
- **Persist triggers:** after **every candle close** and every state change that matters (zone consumed/terminal). Not a timer.
- **Single writer:** a lock file per instance.
- **Validate-or-re-warm:** missing / corrupt / schema-mismatched (`STATE_SCHEMA_VERSION`) / older than a threshold (more than a few candles behind) → discard and re-warm.
- **Pruning:** delete state for symbols inactive past their TTL.

**Restart / gap rule.** On every state load (not only a re-warm) and after any price-feed gap longer than the staleness limit (§5), set `last_price = None` and flag every `Active` zone as awaiting its first tick. The first observed price then resolves them through the missed-touch rule, so a cross that happened while down can never read as a first touch. `last_price` is never trusted across a restart.

**Resume is a pure core input.** The adapter injects `Input::Resume`; the **core** clears `last_price` and flags Active zones — so restarts/gaps are testable via replay (snapshot-resume incl. "price moves through a zone during downtime" → `missed_touch`).

### 4a. Candle-close reconciliation (wick-miss)

Clearing `last_price` catches a cross still inside the zone after downtime; it does **not** catch a wick that touched and returned between polls. So at each **closed candle**, an `Active` zone whose candle range overlapped it (demand: `candle.low <= zone.high`; supply: `candle.high >= zone.low`) with **no observed tick touch** is consumed as `missed_touch`. Keyed strictly to **observed** touches (a first touch either signals or vetoes-consume; a zone is never left `Active` after an observed touch). Synthetic paths sample each candle's extremes, so replay is unchanged; the wick-miss rate shows up **in state**, not only a log.

## 5. Staleness and poll cadence

Poll `allTickers` every **5 s (proposed)**. A price older than **30 s (proposed)** is not fed; recovery follows §4. Both are config, recorded in the shadow report.

### 5a. Background loop lifecycle (per symbol)

Runs continuously while the symbol is active. **Stops** when: activation expires (§3c); the engine drops the ticker; the symbol is **quarantined** (repeated candle errors; §8); or the analyzer shuts down (resumes from saved state on restart).
**Does not stop** on: a signal being sent; an open position (**the analyzer never manages positions**); or a short engine outage.
**State file** is kept after the loop stops until inactive-symbol pruning.
**Re-add after a long gap** = cold start (§3c/§4).
**Touches that occur while the loop is stopped are not signaled** (and are not retroactively emitted).

## 6. RPC shape and delivery (gRPC `Analyze`)

`Analyze(tickers, after_seq)` returns the signals with **`seq > after_seq`**, in **ascending `seq`** order. Semantics:
- **`seq` is ONE global monotonic counter across all symbols** (assigned at emission), **not per symbol**. It **must be persisted** so a restart does not reset it.
- The engine **acknowledges by sending a higher `after_seq`** on its next call. There is **no separate ack RPC**.
- The analyzer **keeps a signal until acknowledged or expired**. Repeating the same `after_seq` **replays** the same pending signals, so a lost response loses nothing.
- The engine **must dedup on `signal_id`** regardless (replay and retries).
- **`after_seq` ahead of the analyzer's counter** (engine state lost, or the analyzer reset): *proposed* — the analyzer returns an empty page plus its current `latest_seq`, and the engine **resets `after_seq = latest_seq`**. Whether to add a response field for this (`latest_seq` and/or an explicit `reset` flag) is an **open question** (§13); today the proto has no such field.
- **Streaming** can be added later over the **same `Signal` message**; it needs reconnect logic in an engine with no tonic dependency today, so **unary ships first**.

## 7. `Signal → proto Signal` mapping

**Extend `analyzer.proto` additively** — never renumber or reuse fields 1–4. Add: `schema_version`, `signal_id`, `seq`, `sl_price`, `tp_price`, `zone_id`, `reason`, `expires_at_ms`, `trigger_time_ms`; add `after_seq` to `AnalyzeRequest`. Keep prices as `double` (to match the engine's `TradeSignal`). `snap to priceIncrement` stays an **engine-side** job. Adding fields breaks any generated-`Signal` struct literal without `..Default::default()` → **require a workspace-wide grep before the proto change merges** (currently clean).

| proto `Signal` field | type | from Phase-0 `Signal` |
| --- | --- | --- |
| `schema_version` (new) | uint32 | payload schema |
| `signal_id` (new) | string | `signal.signal_id` (stable; dedup key) |
| `seq` (new) | uint64 | global monotonic, assigned at emission |
| `expires_at_ms` (new) | int64 | emission time + expiry (§ below) |
| `trigger_time_ms` (new) | int64 | `signal.triggered_at_ms` |
| `ticker` | string | `signal.symbol` (KuCoin form) |
| `action` | string `"buy"`/`"sell"` | `Long → "buy"`, `Short → "sell"` |
| `confidence` | double | fixed `1.0` (engine does NOT gate on it) |
| `entry_price` | double | `reference_entry_price` (Decimal→f64 at the boundary; round-trip pinned) |
| `sl_price` (new) | double | `stop_loss` |
| `tp_price` (new) | double | `take_profit` |
| `zone_id` (new) | string | `signal.zone_id` |
| `reason` (new) | string | `supply_demand_4h rr=<gross_rr> mode=<target_mode>` |

**Expiry.** Signal expiry = **60 s (proposed)**. An unacknowledged signal that expires is **dropped**, and its zone is recorded as **`expired_undelivered`** (the zone was already consumed at emission, so it never re-fires). This requires the core's **`EngineAck`/`Clock`** inputs, which are **Phase 2 work** (Phase 0 defers them).

`Decimal`→`f64` conversion happens **only** at the RPC boundary (`harness::publish::price_f64`), pinned by a round-trip test across price scales.

## 8. Rate limiting, quarantine, and engine-side rules

Analyzer side:
- **Shared limiter** + **per-symbol jitter** (4H boundary burst).
- **Request budget:** Public pool **2000/30s (~66.7 req/s)** VIP0; usage ≈ `0.2 + N/60 + N/300` (allTickers / 4H candles / 1m wick fetches) → N=50 ≈ 1.2 req/s (~2%), N=200 ≈ 4.2 req/s (~6%). Per-endpoint **weights** *(unverified)* must be confirmed.
- **Failure:** a symbol whose candles keep erroring is **quarantined with an alert**; a panicking task must not crash-loop replaying bad data.
- **Quarantine exit:** manual operator action, or a timed cooldown + clean re-warm. **`quarantine_cooldown = 24 h (proposed)`.** Alerts → structured log (and an operator hook if one is wired; **none is currently** — *unverified*).
- **Observations:** per symbol, log the polled price next to the latest 1m high/low.

**Engine-side rules (Phase 2 prerequisites — spec only here):**
- **Health:** mark the analyzer **unhealthy after 3 consecutive failed calls (proposed)**; alert, and **open no new positions from it** until healthy again.
- **Open positions** keep being managed by the engine's own SL/TP logic (not the analyzer).
- **Reject expired signals.**
- **Skip a not-yet-expired signal** if price has moved beyond a tolerance from `trigger_price` **or** is already past the stop. **Tolerance = TBD** (not invented).
- **Dedup on `signal_id`** — needs a persisted table and a **migration owner** (the trade-engine doesn't run migrations today).

## 9. Shadow outcome tracking (Phase 1)

Run each shadow signal through the **Phase 0 sim** (`harness::sim`) over the recorded prices — early, non-authoritative evidence. Diagnostic, not a result.

## 10. Phase 1 exit criteria (numeric)

- Duration **≥ 2 weeks** on the live symbol set.
- **≥ 3 forced restarts** and **≥ 3 forced network drops**, each resuming correctly (state load → §4 → no fabricated first touch; no duplicate signal).
- **Live-vs-replay equivalence** on the recorded input stream (byte-identical outputs).
- **Wick-miss rate** measured; if it exceeds a threshold (**>5% of zone touches, proposed**), switch touch detection to **1m-derived synthetic ticks** (`OneMinuteExpansion`) and re-run.
- **Phase 1 has no gRPC server** and publishes nothing (logs only).

## 11. Phase 2 outline (not committed until Phase 1 passes)

- **Server + client + proto + flag** are all Phase 2 (Phase 1 has none).
- Serve gRPC `Analyze` (§6/§7) behind **one cutover flag**; the engine reads from **exactly one** source — python/Redis **or** rust/gRPC — never both.
- **Python stays deployable for rollback** until the paper bar passes; then retire the Redis `signals:result` path and `tickers:analyze`. Because the Python analyzer is RSI-divergence, this is a **strategy swap as well as a transport swap**, gated on the Phase 0 verdict.
- **Delivery/idempotency:** unary `Analyze` + engine-side dedup on `signal_id` (§6/§8); extend `analyzer.proto` (§7).
- **State store** moves to Postgres/Redis with a defined migration owner; the **global `seq`** persists there.
- **Engine risk** still absent: only a fund gate + `cap_loss`; daily-DD/open-risk/correlation/net-R:R do not exist.
- **Paper bar before live cutover:** ≥ **4 weeks** and ≥ **30 signals**, every backtest-vs-paper difference explained.
- Core `Clock`/`EngineAck` inputs, re-arm policy, signal TTL (expiry, §7).

## 12. Sequence and gating

1. **Phase 0 verdict passes** (§10.4) — hard gate.
2. Phase 1 — shadow (loops + shared poller + file state + recorded stream; **log only, no gRPC**). Exit: §10.
3. Phase 2 — gRPC server/client/proto/flag. Exit: §11.

## 13. Open items and conflicts

**Undecided (do not resolve silently):**
1. **`latest_seq` / reset indication:** if `after_seq` is ahead of the analyzer's counter, how does the engine learn the analyzer's current `latest_seq`? Proposed: add a response field; **not decided**. (§6)
2. **Global-`seq` persistence:** where the single monotonic counter lives across restart (state file vs a small DB row) is unspecified. (§6)
3. **Engine skip tolerance** for a moved price / past-stop signal — **TBD**, not invented. (§8)
4. **Engine health threshold** (3 failed calls) and **quarantine cooldown** (24 h) and **activation TTL** (5 min) and **expiry** (60 s) and **poll cadence** (5–10 s) are **proposed**. (§3c/§5a/§6/§7/§8)
5. **Full-ticker-list omission semantics:** if a caller omits an active ticker from `tickers`, does that deactivate it immediately or only after `activation_ttl`? Proposed: only after TTL. (§3c)
6. **Per-endpoint rate-limit weights** — unverified; needed for the §8 budget. (§8)

**Conflicts with existing text:**
- Phase 0 §12 lists `EngineAck`/`Clock` as deferred, but §7 (expiry → `expired_undelivered`) **requires** them; this doc assigns that work to **Phase 2**. (Not a contradiction if Phase 2 owns it; flagging.)
- Phase 0's `Signal` has no `seq`; the global counter and its persistence are net-new (Phase 2), and `seq` is not something the pure core currently emits. (Phase 0 plan unchanged.)
- The earlier Redis-based §6 (at-most-once / Streams-vs-outbox) is **superseded** by this gRPC design.

**Carried over from prior edits (unchanged):** restart/gap rule (§4), `Input::Resume`, candle-close reconciliation (§4a), candle-close rule (§3a), config scope + `price_increment` snapping (§3b), "first touch only" veto wording (§4a and Phase 0 §6/§7), shadow outcome tracking (§9).
