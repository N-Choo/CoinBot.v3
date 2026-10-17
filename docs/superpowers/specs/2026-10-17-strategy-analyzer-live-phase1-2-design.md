# Strategy Analyzer — Live Integration (Phase 1 Shadow / Phase 2 Publish) — DRAFT v2

- **Status:** DRAFT — **gated on the Phase 0 backtest verdict.** Do not start implementation until the Phase 0 out-of-sample bar passes (Phase 0 spec §10.4). If Phase 0 fails, this is not built.
- **Date:** 2026-10-17
- **Depends on:** `2026-10-17-rust-strategy-analyzer-design.md` (Phase 0), `2026-10-17-strategy-analyzer-phase0.md`.
- **References (verified):**
  - KuCoin bulk tickers: `GET /api/v1/market/allTickers` — snapshot every ~2s, returns per-symbol `last`, `buy`/`sell` (best bid/ask) + sizes; its `high`/`low` are **24h** extremes, not per-poll wicks. (KuCoin docs: "Get All Tickers", spot market data.)
  - KuCoin symbol metadata (price increment): `GET /api/v2/symbols` → `priceIncrement`.
  - KuCoin server time: `GET /api/v1/timestamp`.
  - Engine consumes `TradeSignal` on Redis `signals:result` (`lib/share/src/models/signals.rs`); engine **honors SL/TP** (`process/trade-engine/src/trader/mod.rs:69-79` validates them; `position_exit/mod.rs:88` uses `entry.tp_price`/`entry.sl_price`).
  - First-tick fix regression tests (core): `first_tick_clears_awaiting_for_all_created_zones`, `multi_zone_first_touch_takes_the_nearest_to_prev`, `veto_on_the_nearest_zone_continues_to_the_next_crossed_zone`, `missed_touch_on_first_observation` (`strategy/signals.rs`).

---

## 1. Objective

Run the deterministic core **live, per ticker**, and (Phase 2) publish to the trade engine on the **existing contract** (`TradeSignal` on Redis `signals:result`). Integration is the repo's **engine-triggered request/response**:

```
trade-engine --PUB tickers:analyze--> strategy-analyzer         (activate/refresh symbols)
strategy-analyzer --(Phase 2) PUB signals:result--> trade-engine (Vec<TradeSignal>)
```

The analyzer **owns continuous per-ticker loops** and **persists per-ticker state**. The engine message only registers/refreshes *which* symbols run; it never resets state. **Decision:** the Rust analyzer **replaces** the Python `process/analyzer/` on this path (same channels; Option A targets the same `TradeSignal` contract, so coexistence would double-publish).

## 2. Why a continuous loop (not recompute-per-trigger)

The core is a **state machine** per symbol; its guarantees (first-touch-only, one attempt per zone, missed-touch, no duplicate signals across restarts) depend on carrying state forward. Recomputing from a fixed window per trigger would re-fire consumed zones. The engine's ~24h trigger is coarser than 4H boundaries/tick touches, so the analyzer runs its own loop; the trigger only activates.

## 3. Components

- **Per-ticker loop (task):** one independent task per symbol (repo `tokio::spawn` style), each owning its `StrategyState`; advances on each **closed** 4H candle (`ClosedCandle`) and each price poll (`PriceTick`); emits only core outputs.
- **Shared price poller (one):** `GET /api/v1/market/allTickers`, fan out per-symbol `last` over a channel to each loop.
- **Per-symbol candle poller:** REST 4H candles (the `fetch-data` path) for (re)warm and new-closed-candle detection; shares the global rate limiter.
- **Symbol metadata / clock:** fetch `priceIncrement` per symbol from `/api/v2/symbols`; fetch server time from `/api/v1/timestamp`.
- **State store:** file-per-symbol (Phase 1); §5.
- **Supervisor:** consumes `tickers:analyze`; spawns/refreshes tasks; owns the activation set, TTL, and pruning.
- **Publisher (Phase 2 only):** maps `Signal → TradeSignal` (§7) behind a single cutover flag. **Phase 1 logs only** (or a separate shadow channel) — never `signals:result`.
- **Recorded input stream (Phase 1):** append every symbol's `(ClosedCandle | PriceTick)` inputs to a per-symbol log for live-vs-replay equivalence.

### 3.1 Activation message + TTL

`tickers:analyze` payload (JSON):
```json
{ "schema_version": 1, "symbols": ["XRP-USDT", "BTC-USDT"], "sent_at_ms": 1767225600000 }
```
- **Activation TTL** well above the 24h refresh (proposed `activation_ttl = 72h`): on expiry the symbol is **deactivated** (its loop stops; state is retained until pruning). One missed message must not drop a symbol and its zone state.
- **Removal:** a symbol absent from the active set is deactivated (state retained, pruned per TTL).
- **Re-add after a long gap:** treated as a **cold start** — re-warm from history and apply the restart/gap rule (§4.1).

## 4. Persistence and restart/gap safety (Phase 1)

State directory is a **mounted Docker volume** (compose) or every restart silently re-warms each symbol.

- **Atomic writes:** temp file → `fsync(file)` → `rename` → `fsync(dir)`.
- **Persist triggers:** after **every candle close**, and after every state change that matters (a zone consumed/terminal). Not on a timer.
- **Single writer:** a lock file per instance.
- **Validate-or-re-warm:** a state file that is missing, **corrupt**, **schema-mismatched** (`STATE_SCHEMA_VERSION`), or **older than a threshold** (more than a few candles behind) is discarded and re-warmed from history.
- **Pruning:** delete state for symbols inactive past their TTL.

### 4.1 Restart / feed-gap rule (applies on EVERY state load and after ANY gap)

**A stale `last_price` can fabricate a first touch.** A price that crossed a zone while the analyzer was down would otherwise look like a legitimate first touch on the next live tick (previous price above the zone, current inside). Therefore, on **every state load** and after **any price-feed gap**:
1. **Clear `last_price`** (no "previous price" is trusted across a gap).
2. **Flag every `Active` zone** as awaiting its first tick (`zones_awaiting_first_tick`), so the next observed price is evaluated by the **missed-touch rule** (§7.2 of Phase 0), not as a first touch.
3. Resume only once data is fresh (per §4.2).

This is the same rule as post-re-warm, applied at *every* load and gap — not only after re-warm.

### 4.2 Staleness (with defaults)

- A **price older than `price_staleness_ms` (default 10 000 ms)** is **not fed** as a tick; the gap triggers §4.1.
- **Candle-lateness alert:** if the expected closed 4H candle has not arrived within **5 minutes** of its boundary, log an alert (Phase 0's proposed default).
- **Clock source:** exchange time from `/api/v1/timestamp`; never local NTP. Candle-close and freshness are computed against exchange time.

## 5. Candle-close and clock rule

A 4H candle at `open_time` counts as **closed only when `open_time + 4h <= exchange_time`**. The **forming candle is excluded** from every fetch. Apply a short **settle delay** (proposed 2s) after the boundary before accepting the candle, in case the exchange revises the last candle. A **hole** in the series triggers an **in-order backfill** before resuming — the recovery path the core's `GappedCandle` error expects.

## 6. config_version must not depend on the symbol set

Phase 0's `Config` includes the per-symbol `price_scale` map and `config_version` hashes the whole config; adding a ticker would change the version on every signal and invalidate the freeze lock. Fix:
- **Move per-symbol `price_scale` out of the hashed strategy config** into per-symbol **instance data** (a registry keyed by symbol), fetched as `priceIncrement` from `/api/v2/symbols` at activation.
- **`config_version` hashes only the strategy parameters** (not the symbol set), so it is stable across ticker additions.
- The **freeze lock** records the strategy `config_version` + the fixed constants; the symbol registry is separate.

## 7. Delivery semantics — Pub/Sub is at-most-once

Phase 2 publishing via Redis **Pub/Sub** is **at-most-once**: a signal is lost if the engine is down (there is no persistence/ack). The consume-then-publish ordering in Phase 0 does not change this. Options (decide before Phase 2 exit):
- **Redis Streams** with a consumer group + `XACK` (at-least-once; engine dedups on `signal_id`), **or**
- keep Pub/Sub with a **durable outbox** and engine-side idempotency (the engine currently has **no** dedup table).

This is recorded in §10 and must be resolved before the cutover.

## 8. `Signal → TradeSignal` mapping (Option A)

| `TradeSignal` (engine) | type | from Phase-0 `Signal` |
| --- | --- | --- |
| `ticker` | String | `signal.symbol` (KuCoin form, e.g. `XRP-USDT`; must match the engine's contract ticker) |
| `action` | String `"BUY"`/`"SELL"` | `Long → "BUY"`, `Short → "SELL"` |
| `confidence` | `f64` | fixed `1.0` (or derived; documented) |
| `entry_price` | `f64` | `reference_entry_price` (Decimal string → `f64` **at the publish edge only**) |
| `sl_price` | `f64` | `stop_loss` |
| `tp_price` | `f64` | `take_profit` |
| `reason` | String | compact: `supply_demand_4h zone=<id> rr=<gross_rr> mode=<target_mode>` |

Notes: the engine **honors `sl_price`/`tp_price`** (validated in `trader/mod.rs:69-79`, used by `position_exit`). Decimal→`f64` conversion happens **only** at the publisher boundary; the core and state stay `Decimal`. The stable `signal_id` (Phase 0) is carried in `reason` until the engine schema supports a dedicated id field (Phase 2 dedup key).

## 9. Rate limiting, isolation, quarantine

- **Shared limiter:** one global KuCoin rate limiter for all loops; add **per-symbol jitter** so the 4H candle-boundary burst doesn't trip limits.
- **Request budget:** size against KuCoin's documented per-endpoint limits for the **real symbol count**, including the 1m candle fetches used for wick comparison. (A concrete table is required at implementation time; not yet computed.)
- **Per-task failure:** a symbol whose candles keep erroring (e.g. `MalformedCandle`) is **quarantined with an alert**. A panicking task must **not** crash-loop replaying the same bad data.
- **Quarantine exit:** a quarantined symbol leaves quarantine by **manual operator action**, or **automatically after a timed cooldown followed by a clean re-warm** (from history, with §4.1). **Alerts go to** the project's structured log (and, if a channel exists, an operator alerting hook — none is currently wired).
- **Observations to record (Phase 1):** per symbol, log the **polled price** next to the **latest 1m candle's high/low**, to quantify wicks the polling misses.

## 10. Shadow outcome tracking (Phase 1 — early live evidence)

Beyond equivalence, Phase 1 also **simulates each shadow signal against the recorded prices using the Phase 0 sim** (`harness::sim`), producing an early, non-authoritative outcome distribution. This is diagnostic, not a result.

## 11. Phase 1 exit criteria (numeric)

- Duration **≥ 2 weeks** on the configured live symbol set.
- **≥ 3 forced restarts** and **≥ 3 forced network drops**, each verified to resume correctly (state load → §4.1 → no fabricated first touch; no duplicate signal).
- **Live-vs-replay equivalence** holds on the recorded input stream (byte-identical outputs).
- **Wick-miss rate** measured (polled price vs 1m high/low). If it exceeds a **threshold (proposed: wick-miss on >5% of zone touches)**, switch touch detection to **1m-derived synthetic ticks** (the existing `OneMinuteExpansion` path) and re-run.
- Zero `signals:result` publications in Phase 1 (log/ shadow channel only).

## 12. Phase 2 outline (not committed until Phase 1 passes)

- Publish (§8 mapping) to `signals:result` behind **one cutover flag**; **Python kept deployable for rollback**; the two publishers are **mutually exclusive** under that single flag.
- **Delivery:** Streams+`XACK` or outbox (§7); engine-side **dedup on `signal_id`** (must be built — none exists today).
- **State store** moves to Postgres/Redis with a defined migration owner (the trade-engine doesn't run migrations today).
- Engine consumer changes + risk: the engine today has only a fund gate + `cap_loss`; daily-DD/open-risk/correlation/net-R:R **do not exist**.
- **Paper-trading bar before any live cutover:** ≥ **4 weeks** and ≥ **30 signals**, with every difference between paper outcomes and the backtest on the same period explained.
- `Clock`/`EngineAck` inputs, re-arm policy, signal TTL.

## 13. Sequence and gating

1. **Phase 0 verdict passes** (§10.4) — hard gate.
2. Phase 1 — shadow (loops + shared poller + file state + recorded stream; log only). Exit: §11.
3. Phase 2 — publish (outline above). Exit: §12.

## 14. Open items

- §6.3 valid-high/low human verification (video 00:29–04:30) — still open from Phase 0.
- **Phase 0 OOS bar outcome — gates this entire document.**
- Phase 2 delivery choice (Streams vs outbox) and state store (Postgres vs Redis) + migration owner.
- Concrete request-budget table (§9) against KuCoin limits at the real symbol count.
- Whether `OneMinuteExpansion` chunking (currently by count, `tick_source.rs`) should be time-bucketed before any 1m headline use (Phase 0 flagged) — also relevant to the §11 wick-miss fallback.
- `signal_id` field in `TradeSignal` (or `reason`-embedded) until the engine schema changes.
