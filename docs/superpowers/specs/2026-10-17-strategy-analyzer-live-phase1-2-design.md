# Strategy Analyzer — Live Integration (Phase 1 Shadow / Phase 2 Publish) — DRAFT v3

- **Status:** DRAFT — **gated on the Phase 0 backtest verdict.** Nothing here is built until the Phase 0 out-of-sample bar passes (Phase 0 spec §10.4).
- **Date:** 2026-10-17
- **Depends on:** `2026-10-17-rust-strategy-analyzer-design.md`, `2026-10-17-strategy-analyzer-phase0.md`.

**Verified references (with sources):**
- Bulk tickers: `GET /api/v1/market/allTickers` — snapshot every ~2s; per-symbol `last`, `buy`/`sell`, sizes; `high`/`low` are **24h**, not per-poll wicks. Source: KuCoin docs, "Get All Tickers" (`https://www.kucoin.com/docs-new/rest/spot-trading/market-data/get-all-tickers`).
- Rate limits: public endpoints share the **Public pool**, IP-based; **VIP0 = 2000 requests / 30s** (~66.7 req/s). Source: KuCoin docs, "Rate Limit Rule (Classic)" (`https://www.kucoin.com/docs-new/rate-limit-rule-classic`).
- Symbol metadata (price increment): `GET /api/v2/symbols` → `priceIncrement`. Server time: `GET /api/v1/timestamp`.
- Engine consumes `TradeSignal` (`lib/share/src/models/signals.rs`) and **honors SL/TP** (`process/trade-engine/src/trader/mod.rs:69-79` validates them; `position_exit/mod.rs:88` uses `entry.tp_price`/`entry.sl_price`).
- First-tick correctness is covered by **named regression tests** (`strategy/signals.rs`): `first_tick_non_nearest_zone_touch_is_not_missed` (warm several zones; a **non-nearest** zone's first tick arrives from the approach side → expects a normal touch, **not** `missed_touch`), `first_tick_clears_awaiting_for_all_created_zones`, `multi_zone_first_touch_takes_the_nearest_to_prev`, `veto_on_the_nearest_zone_continues_to_the_next_crossed_zone`, `missed_touch_on_first_observation`.

---

## 1. Objective

Run the deterministic core live, per ticker, and serve signals to the trade engine over **gRPC** via the **existing** `AnalyzerService.Analyze` (`process/proto/analyzer.proto`, already compiled by `share/build.rs` and re-exported as `share::AnalyzerService` / `pub use analyzer::*`). The Rust analyzer **replaces** the Python `process/analyzer/`.

```
trade-engine --gRPC Analyze(tickers)--> strategy-analyzer     (unary request/response)
strategy-analyzer --AnalyzeResponse{signals}--> trade-engine
```

Integration is **request/response**: the engine calls `Analyze(tickers)` and the analyzer returns the signals it produced since the previous call per ticker. Per-ticker loops run continuously and own per-ticker state; the RPC never resets it. The analyzer implements the tonic **server** (pattern: `process/deposit-worker/src/grpc_handler.rs`); the engine gains a tonic **client** (it has no `tonic` dependency today; `share` already exposes the generated types).

## 2. Why a continuous loop

The core is a per-symbol **state machine**; its guarantees (first-touch-only, one attempt per zone, missed-touch, no duplicate signals across restarts) require carrying state forward. The engine's ~24h trigger is coarser than 4H boundaries/tick touches, so the analyzer runs its own loop; the trigger only activates.

## 3. Components

- **Per-ticker loop (task):** one independent task per symbol (repo `tokio::spawn` style), each owning its `StrategyState`; advances on each closed 4H candle (`ClosedCandle`) and each price poll (`PriceTick`).
- **Shared price poller (one):** `GET /api/v1/market/allTickers`, fan out per-symbol `last` over a channel.
- **Per-symbol candle poller:** REST 4H candles (the `fetch-data` path); shares the global limiter.
- **Symbol metadata / clock:** `priceIncrement` from `/api/v2/symbols`; time from `/api/v1/timestamp`.
- **State store:** file-per-symbol (Phase 1); §4.
- **Supervisor:** consumes `tickers:analyze`; spawns/refreshes tasks; owns the activation set, TTL, pruning.
- **Publisher (Phase 2 only):** maps `Signal → TradeSignal` (§7) behind one cutover flag. **Phase 1 logs only** (or a shadow channel) — never `signals:result`.
- **Recorded input stream (Phase 1):** append each symbol's `(ClosedCandle | PriceTick)` inputs for live-vs-replay equivalence.

### 3a. Candle-close rule (adapter-owned, since the core has no clock)

A 4H candle is closed only when `open_time + 4h + settle <= exchange_now`, with `settle = 5s` (proposed) to absorb late revisions. `exchange_now = local monotonic clock plus an offset measured against KuCoin's time endpoint and refreshed periodically`. Candle fetches drop any row that is not yet closed. A hole in the returned series triggers an **in-order backfill** before any candle is fed to the core. If the exchange revises an already-processed candle, **alert and re-warm**, because the core cannot amend history. **Alert when a closed candle is more than 5 minutes late.**

### 3b. Config scope

`config_version` hashes **strategy parameters only**. Per-symbol `price_scale` (from the symbol's price increment at activation) is **instance data** and is excluded from the hash. Otherwise adding a ticker changes the version on every signal and invalidates the freeze lock. This also requires a small change to the Phase 0 plan (Task 4), where `Config` currently holds the `price_scale` map and `default_for` hardcodes 4 dp.

### 3c. Activation message + TTL

`tickers:analyze` payload (JSON):
```json
{ "schema_version": 1, "symbols": ["XRP-USDT", "BTC-USDT"], "sent_at_ms": 1767225600000 }
```
- **Activation TTL** well above the 24h refresh (proposed `activation_ttl = 72h`): on expiry the symbol is **deactivated** (loop stops; state retained until pruning). One missed message must not drop a symbol and its zone state.
- **Removal:** a symbol absent from the active set is deactivated (state retained, pruned per TTL).
- **Re-add after a long gap:** treated as a **cold start** — re-warm from history and apply §4 (gap rule), so no fabricated first touch.

## 4. Persistence and restart/gap safety (Phase 1)

State directory is a **mounted Docker volume** (compose), or every restart silently re-warms each symbol.

- **Atomic writes:** temp file → `fsync(file)` → `rename` → `fsync(dir)`.
- **Persist triggers:** after **every candle close**, and after every state change that matters (a zone consumed/terminal). Not on a timer.
- **Single writer:** a lock file per instance.
- **Validate-or-re-warm:** a state file that is missing, corrupt, schema-mismatched (`STATE_SCHEMA_VERSION`), or older than a threshold (more than a few candles behind) is discarded and re-warmed from history.
- **Pruning:** delete state for symbols inactive past their TTL.

**Restart / gap rule.** On every state load (not only a re-warm), and after any price-feed gap longer than the staleness limit (§5), set `last_price = None` and flag every `Active` zone as awaiting its first tick. The first observed price then resolves them through the missed-touch rule, so a cross that happened while the analyzer was down can never read as a first touch. The state file's `last_price` is never trusted across a restart.

**Resume is a pure core input.** The adapter does not poke state itself: it injects `Input::Resume`, and the **core** clears `last_price` and flags the Active zones. This keeps restart/gap logic inside the deterministic core where the harness can inject restarts and gaps (snapshot-resume tests, including "price moves through a zone during downtime" → `missed_touch`, not a signal). Phase 1's "≥3 restarts" exit criterion is tested through this path.

### 4a. Candle-close reconciliation (wick-miss)

Clearing `last_price` catches a cross that is *still inside* the zone after downtime; it does **not** catch a wick that touched a zone and returned above it between two polls. The core only invalidates on closes, so nothing else would catch it. Therefore, at each **closed candle**, an `Active` zone whose **candle range overlapped it** (demand: `candle.low <= zone.high`; supply: `candle.high >= zone.low`) but for which the core **observed no tick touch** is consumed as `missed_touch`. It is keyed strictly to **observed** touches (a zone whose touch was observed is already consumed by signal/veto, so it is not `Active`). Synthetic backtest paths sample each candle's low and high, so replay results are unchanged; the wick-miss rate then shows up **in state**, not only in a log.

## 5. Staleness and poll cadence

Poll `allTickers` every **5s** (proposed). A price older than **30s** (proposed) is not fed as a tick, and recovery from that gap follows the §4 rule. The staleness limit and poll interval are **config**, recorded in the shadow report.

## 6. Delivery semantics (gRPC `Analyze`)

`Analyze` is **unary request/response** (`analyzer.proto`). The engine calls `Analyze(tickers)` and the analyzer returns the signals produced **since the last successful call** for those tickers. Consequences:
- A **lost or retried** call must not create or drop a trade → stable `signal_id` + **engine-side dedup** (idempotency). The engine has **no** dedup today.
- **`analyzer.proto` must be extended before use:** its `Signal` is `{ticker, action, confidence, entry_price}` — it lacks `sl_price`, `tp_price`, `zone_id`, and `signal_id`. The engine **honors SL/TP**, so these fields (or an equivalent) must be added, alongside a `schema_version`.
- **Direction decision (confirm):** the engine drives the call cadence (pull). If signals must be pushed the instant a touch occurs, a server-streaming RPC is needed instead of unary `Analyze` (a proto change).

## 7. `Signal → gRPC Signal` mapping

The proto `Signal` today: `{ ticker, action, confidence, entry_price }`. **Extend it** (proto change) to `{ schema_version, signal_id, ticker, action, confidence, entry_price, sl_price, tp_price, zone_id, reason }` so the engine can honor SL/TP and dedup:

| proto `Signal` field | type | from Phase-0 `Signal` |
| --- | --- | --- |
| `signal_id` (new) | string | `signal.signal_id` (stable; the engine's dedup key) |
| `ticker` | string | `signal.symbol` (KuCoin form) |
| `action` | string `"buy"`/`"sell"` | `Long → "buy"`, `Short → "sell"` |
| `confidence` | double | fixed `1.0` (engine does not gate on it — logging only) |
| `entry_price` | double | `reference_entry_price` (Decimal→f64 at the boundary; round-trip pinned) |
| `sl_price` (new) | double | `stop_loss` |
| `tp_price` (new) | double | `take_profit` |
| `zone_id` (new) | string | `signal.zone_id` |
| `reason` (new) | string | `supply_demand_4h rr=<gross_rr> mode=<target_mode>` |

The engine **honors `sl_price`/`tp_price`** (validated in `trader/mod.rs`; used by `position_exit`). `Decimal`→`f64` conversion happens **only** at the RPC boundary. **The engine must also quantize SL/TP to the symbol's `priceIncrement`** (it currently rounds only quantity) — the analyzer now snaps levels to `price_increment` in the core, and the engine re-validates.

## 8. Rate limiting, isolation, quarantine

- **Shared limiter** for all loops; **per-symbol jitter** so the 4H boundary burst doesn't trip limits.
- **Request budget (computed):** the Public pool is **2000 req / 30s (~66.7 req/s)** per IP at VIP0. Usage ≈ `0.2` (one shared allTickers poll / 5s) `+ N/60` (4H candle poll per symbol / minute) `+ N/300` (1m wick-comparison fetch per symbol / 5 min) req/s. For **N = 50** ≈ **1.2 req/s** (~2% of quota); **N = 200** ≈ **4.2 req/s** (~6%). Comfortably within quota at request *count*; per-endpoint **weights** must be confirmed from each endpoint's docs before relying on any headroom (assumption: small). If weights are heavier, reduce 1m fetch frequency first.
- **Per-task failure:** a symbol whose candles keep erroring (e.g. `MalformedCandle`) is **quarantined with an alert**; a panicking task must not crash-loop replaying bad data.
- **Quarantine exit:** a quarantined symbol leaves quarantine by **manual operator action**, or **automatically after a timed cooldown followed by a clean re-warm** (from history, applying §4). **Alerts go to** the structured log, and to an operator alerting hook if one is wired (none currently).
- **Observations to record:** per symbol, log the polled price next to the latest 1m candle's high/low, to quantify missed wicks.

## 9. Shadow outcome tracking (Phase 1)

Run each shadow signal through the **Phase 0 sim** (`harness::sim`) against the recorded prices, producing an early, non-authoritative outcome distribution. Diagnostic, not a result.

## 10. Phase 1 exit criteria (numeric)

- Duration **≥ 2 weeks** on the live symbol set.
- **≥ 3 forced restarts** and **≥ 3 forced network drops**, each verified to resume correctly (state load → §4 → no fabricated first touch; no duplicate signal).
- **Live-vs-replay equivalence** on the recorded input stream (byte-identical outputs).
- **Wick-miss rate** measured; if it exceeds a threshold (**proposed: >5% of zone touches**), switch touch detection to **1m-derived synthetic ticks** (`OneMinuteExpansion`) and re-run.
- Zero `signals:result` publications in Phase 1.

## 11. Phase 2 outline (not committed until Phase 1 passes)

- Serve gRPC `Analyze` (§6/§7) behind **one cutover flag**; **Python kept deployable for rollback**; the two analyzers **mutually exclusive** under that flag.
- **Delivery:** idempotent unary `Analyze` + **engine-side dedup on `signal_id`** (§6); extend `analyzer.proto` with `sl_price`/`tp_price`/`zone_id`/`signal_id`/`schema_version`.
- **State store** moves to Postgres/Redis with a defined migration owner.
- Engine consumer + risk: the engine today has only a fund gate + `cap_loss`; daily-DD/open-risk/correlation/net-R:R **do not exist**.
- **Paper-trading bar before any live cutover:** ≥ **4 weeks** and ≥ **30 signals**, with every difference between paper outcomes and the backtest explained.
- `Clock`/`EngineAck` inputs, re-arm policy, signal TTL.

## 12. Sequence and gating

1. **Phase 0 verdict passes** (§10.4) — hard gate.
2. Phase 1 — shadow (loops + shared poller + file state + recorded stream; log only). Exit: §10.
3. Phase 2 — publish. Exit: §11.

## 13. Open items

- §6.3 valid-high/low human verification (video 00:29–04:30) — still open from Phase 0.
- **Phase 0 OOS bar outcome — gates this entire document.**
- Per-endpoint **weights** to finalize the §8 request budget.
- The **missed-touch rule** is *approach-from-outside* (Phase 0 §7.2, updated): a zone is `missed_touch` unless the first observation is an approach from outside it (i.e. `prev` is `None` or already inside/beyond ⇒ missed). Covered by `activated_while_inside_is_missed`, `first_observation_approach_from_outside_is_not_missed`, `missed_touch_on_first_observation`. Restart/gap handling is now the pure `Input::Resume` input (testable via replay; §4).
- Phase 2 state store (Postgres vs Redis) + migration owner; extend `analyzer.proto`; add a `tonic` client to `trade-engine`.
- `OneMinuteExpansion` chunking by count → time-bucketed before any 1m headline use (also relevant to the §10 wick-miss fallback).
- `signal_id` field in `TradeSignal` (or `reason`-embedded) until the engine schema changes.
- Phase 0 plan Task 4 change: move `price_scale` out of hashed `Config` (§3b).
