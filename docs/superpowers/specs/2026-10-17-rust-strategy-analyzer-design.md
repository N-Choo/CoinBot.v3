# Rust Strategy Analyzer — Phase 0 Design (Pure Core + Backtest Harness)

- **Status:** Design — revised per review, awaiting approval
- **Implementation status (2026-10-17):** core (Tasks 1–16) and harness (Tasks 17–24) landed and
  green; Phase 0 exit criteria (§10.5) are met apart from the human §6.3 video check and the
  OOS backtest run. Crate overview: `process/strategy-analyzer/README.md`.
- **Date:** 2026-10-17
- **Scope:** Phase 0 only
- **Supersedes (for Phase 0):** the "Rust strategy analyzer for CoinBot" v2 spec
- **Related:** `docs/superpowers/specs/2026-08-02-analyzer-module-design.md`, `2026-10-17-engine-fidelity-benchmark-design.md`
- **Repo facts this doc is grounded in:** see §0.1.

---

## 0. Scope, non-goals, and corrections to v2

### 0.1 Repository reality (verified)

- Backend is a Rust Cargo workspace: `process/api_gateway` (api-gateway), `lib/share` (share), `process/deposit-worker` (deposit), `process/trade-engine` (trade-engine). `lib/kucoin` is **excluded** from the workspace; the build uses crates.io `kucoin = "0.8.1"`.
- Signal flow is **engine-triggered request/response over Redis Pub/Sub**: `trade-engine` PUBs `tickers:analyze` (on a 24h timer) to trigger the Python analyzer, which replies on `signals:result`; the engine SUBs that and deserializes `Vec<TradeSignal>` (`lib/share/src/models/signals.rs`). There is no HTTP/gRPC pull and no signals table. `process/proto/analyzer.proto` is dead scaffolding. **Note:** the v2 rule "the engine must not request signals" would require changing this existing trigger; Phase 0 has no transport, so this design does not depend on that rule either way.
- There is **no websocket feed**; Rust price access is REST polling of `level1`. Candle ingestion exists **only** in Python (`process/analyzer/fetchers/kucoin.py`).
- No market-structure / supply-demand / pivot / BOS logic exists in any language. All such logic is net-new.
- `trade-engine` does not run migrations; only `api_gateway` and `deposit-worker` do. There is no dedup/signals persistence.

### 0.2 Corrections adopted from the design review

| # | v2 issue | Resolution in this design |
|---|---|---|
| 1 | `target_r = min_rr = 2.5` gives no cost cushion; any fees/slippage/borrow push net R:R below the engine's 2.5 floor, so the engine would later veto every signal. Additionally, in `fixed_r` mode a `gross_rr >= min_gross_rr` gate is a **tautology** (TP is defined as `target_r` × risk). | Decouple target from floor. `fixed_r` uses `target_r` (3.0); its cushion comes from `target_r` exceeding the engine's net floor (2.5). `min_gross_rr` (3.0) gates **`structural` mode only**, evaluated on the rounded payload levels. The tautological fixed_r gross gate is removed. |
| 2 | "Keep the old pull path behind a flag / remove in Phase 5" is premised on a passive pull path that does not exist. | The real mechanism is **engine-triggered request/response** over Redis Pub/Sub (engine PUBs `tickers:analyze` → analyzer replies `signals:result`, §0.1); there is no "pull path" to keep or remove. Phase 0 has no transport at all; transport work is a separate phase (§12). |
| 3 | The engine was assumed to enforce daily-DD / open-risk / correlation / net-R:R. None exist. | Out of scope for Phase 0; recorded as prerequisite/owner items for later phases (§12). |
| 4 | Touch detection assumed a live feed that does not exist. | Phase 0 drives touches from a `TickSource`; live feed is a later-phase prerequisite (§12). |

### 0.3 Objective (Phase 0)

Deliver a **self-contained, deterministic Rust crate** implementing the supply-and-demand 4H strategy as a pure, I/O-free core, plus an offline backtest/replay harness. No network in the backtest, no Redis, no database, no trade-engine changes.

### 0.4 Explicit non-goals (Phase 0)

- No transport, Redis publishing, outbox, or engine consumer.
- No live price feed, no exchange order placement, no risk/account state.
- No replacement of or changes to the Python `process/analyzer/` — Phase 0 is a parallel, self-contained crate. Its eventual role (replace vs. coexist) is deferred (§12).
- `EngineAck` and `Clock` inputs are **deferred**. Consequently **re-arm and signal TTL are absent in Phase 0** (both depend on those inputs).

---

## 1. Crate layout & architecture

New workspace member: **`process/strategy-analyzer`** (package `strategy-analyzer`). The module is named **`strategy/`**, not `core/`, to avoid any ambiguity with the standard `core` crate in `use core::...` paths.

```
process/strategy-analyzer/
├── Cargo.toml                 # core deps: rust_decimal, serde, thiserror
│                              # harness deps (clap, csv, serde_json, reqwest/ureq) behind feature "harness"
├── src/
│   ├── lib.rs                 # pub mod strategy; #[cfg(feature="harness")] pub mod harness;
│   ├── strategy/              # PURE CORE — no I/O, no f64, no wall-clock, no HashMap iteration
│   │   ├── mod.rs             # #![deny(clippy::float_arithmetic)]
│   │   ├── decimal.rs         # rounding helper, scales, checked ops
│   │   ├── error.rs           # StrategyError (typed, no panics)
│   │   ├── config.rs          # Config + config_version + validation
│   │   ├── types.rs           # Input, Output, StrategyState, Zone, ZoneState, Signal, TrendState
│   │   ├── step.rs            # pub fn step(...)
│   │   ├── indicators/atr.rs  # Wilder ATR, Decimal
│   │   ├── structure.rs       # pivots, BOS, trend state machine
│   │   ├── zones.rs           # creation, lifecycle, mitigation, size limits, selection
│   │   └── signals.rs         # trigger, SL/TP, obstacle check, target modes
│   ├── harness/               # may use f64 for reporting only; NEVER feeds core
│   │   ├── mod.rs
│   │   ├── tick_source.rs     # TickSource trait + OneMinuteExpansion + Ohlcv4hSynthesis
│   │   ├── dataset.rs         # cached OHLCV loader, dataset hash
│   │   ├── replay.rs          # drives core; enforces event ordering
│   │   ├── sim.rs             # synthetic position manager (fills, exits, costs, holding time)
│   │   ├── report.rs          # ledger (Decimal), stats (f64), mode/path comparison, calibration
│   │   └── fetch.rs           # network fetch (used ONLY by the fetch-data subcommand)
│   └── bin/backtest.rs        # CLI: `fetch-data` + `backtest`; [[bin]] required-features = ["harness"]
└── tests/                      # integration + replay-equivalence fixtures
```

**Dependency direction is strictly one-way:** `harness → strategy`. The core has no knowledge of tick origin, resolution, or fill model.

**Boundary invariants (enforced):**
- `strategy/` carries `#![deny(clippy::float_arithmetic)]`.
- **Extended float ban:** a `syn` **token-level** CI test (plan Task 29/CI hardening) parses every `strategy/**/*.rs` and fails on any `f32`/`f64` **token** (clippy only bans float *arithmetic*, not types/literals/casts). Comment-stripped greps are kept only as a fast pre-filter; the token test catches a real `f64` on a line that also has a trailing comment and does not flag `f64` inside comments or string literals.
- **No `HashMap`/`HashSet` iteration in `strategy/`** — use `BTreeMap`/`Vec`. Randomized iteration order is the usual hidden nondeterminism. Enforced by CI grep/lint.
- Harness deps are feature-gated: `#[cfg(feature = "harness")] pub mod harness;`. `cargo build -p strategy-analyzer --no-default-features` builds the pure core alone, with no `clap`/`csv`/HTTP deps compiled.
- Config numbers parse as `Decimal`/strings, never through `f64`.

**Workspace wiring:** add `process/strategy-analyzer` to root `Cargo.toml` `[workspace].members`; declare `[[bin]] name = "backtest", required-features = ["harness"]` so `--no-default-features` builds succeed; add `strategy-analyzer` to `Makefile` `PACKAGES`; extend `make ci` with the code-scoped float grep, the `HashMap` grep, and a `cargo build -p strategy-analyzer --no-default-features` step.

**CI note:** the cross-platform golden-hash check (§10.2) needs a matrix with **≥ 2 architecture entries (x86_64 + arm64)**. `ci.yml` runs the Rust job on `ubuntu-latest` (x86_64) and `ubuntu-24.04-arm` (arm64), and the golden hash is compared on both legs. (`ubuntu-24.04-arm` is a GitHub-hosted arm64 runner whose availability depends on the account plan and may need enabling for forks/private repos.)

---

## 2. Numeric & determinism policy

- **Working scale:** 12 dp for all indicator math.
- **Price scale / increment:** per-symbol, from config. `price_scale` (dp) is used only when producing levels that appear in signals (zone bounds, stop, take-profit, trigger); a single global dp rule is wrong for both BTC-USDT and low-priced pairs. **Increment decision (2026-10-17):** a dp scale cannot express a step-5/step-25 grid (e.g. KuCoin `0.0005` at 4 dp), so the **core also carries the symbol's `price_increment`** — instance data alongside `price_scale` — and, when configured, **snaps** stop/take-profit/reference levels to valid multiples: stop **away from** entry, take-profit **toward** entry, reference/obstacle to the **nearest** (§7.4/§7.6). Published prices are therefore valid exchange multiples; the **engine still re-validates** them. With no increment configured for a symbol the core keeps the Phase 0 dp `round_price` behavior (no change to existing fixtures/goldens).
- **One rounding helper:** `round_price(value, price_scale)`; rounding strategy is **half-up**, requested explicitly via `round_dp_with_strategy` (rust_decimal's default is half-even). Nobody calls `.round_dp()` directly. Any change to the strategy is a one-line change in `decimal.rs`.
- **Ratios are NOT rounded before comparison.** `gross_rr`, `zone_height_atr`, and `obstacle_rr` are compared at working scale (rounding to 4 dp first could turn 2.99996 into 3.0000 and pass a 3.0 floor). They are rounded (toward zero, 4 dp) **only when serializing or reporting**.
- **Self-consistent payload:** stop, take-profit, and ratios are derived from the **already-rounded** trigger and stop, so the serialized fields reconcile exactly.
- **ATR (pinned):** Wilder smoothing; seed = simple mean of the first 14 true ranges; **the first candle's true range = high − low** (no previous close); each step rounded at working scale; warm-up = no signals until 14 completed candles exist. Documented so implementations agree; note the claim "avoid drift vs. the Python analyzer" holds **only if that analyzer has an ATR implementation** (its current signals are RSI-divergence based) — otherwise drop the cross-check claim.
- **No panics:** all `Decimal` operations use checked variants; overflow / divide-by-zero return `StrategyError`. **`ATR == 0` on a flat series is *not* an error** — it is handled in-band (§4) as a `StateChange`, so the stream does not stall.
- **Serialization:** Decimals serialize as fixed-scale **strings**, never JSON numbers. Timestamps are `i64` UTC milliseconds internally; ISO-8601 only at the serialization edge.
- **Float carve-out:** the harness's reporting statistics (expectancy, drawdown, Sharpe-style ratios, bootstrap bounds) may use `f64`. They never feed the core. The **trade ledger is `Decimal`**.
- **`config_version` = hash of the canonical strategy parameters.** The version attached to signals is derived by hashing the canonicalized strategy parameters (deterministic field order), **excluding per-symbol `price_scale` instance data** (`Config::canonical_params_json`), so it equals the freeze-lock value (§10.4) and does not move when a ticker is added or a symbol's scale is refined.
- **State scale normalization:** when serializing state, every `Decimal` field is normalized to a fixed scale so snapshot round-trips are byte-identical (§10.2).

---

## 3. Strategy configuration

All values configurable and versioned; `config_version` is attached to every signal. None hardcoded.

`price_scale` and `price_increment` are the exceptions to "everything versioned": they are **per-symbol instance data** (the symbol's price scale and its actual price step, discovered at activation), not strategy parameters. They stay on `Config` so the core can round/snap levels, but they are **excluded from `config_version`** (via `canonical_params_json`). Hashing them would change the version — and invalidate the freeze lock (§10.4) — every time a ticker is added, **before the freeze lock is written** or after.

| Key | Default | Notes |
|---|---|---|
| `timeframe` | `4hour` | |
| `atr_period` | 14 | Wilder, seed = mean of first 14 TR |
| `pivot_left` / `pivot_right` | 2 / 2 | |
| `bos_min_distance` | 0.1 × ATR | |
| `zone_search_lookback` | 10 | completed candles before breakout candle |
| `impulse_body_threshold` | 1.5 × ATR | breakout candle body |
| `zone_expiry_candles` | 30 | completed 4H candles |
| `stop_buffer` | 0.1 × ATR | |
| `min_zone_height` | 0.5 × ATR | instrumented (§7.5) |
| `max_zone_height` | 3.0 × ATR | instrumented (§7.5) |
| `mitigation_grace_candles` | 1 | |
| `target_mode` | `fixed_r` | `fixed_r` or `structural` |
| `target_r` | 3.0 (proposed) | used by `fixed_r` only |
| `min_gross_rr` | 3.0 (proposed) | gross floor for **`structural` mode only**; engine net floor stays 2.5 |
| `structural_buffer` | 0.1 × ATR | TP sits this far inside the obstacle |
| `allow_no_obstacle` | `true` | |
| `direction` | long + short | |
| `entry_trigger` | first zone touch | |
| `entry_attempts` | first touch only | |
| `re_arm_policy` | `none` | depends on `EngineAck` → not active in Phase 0 |
| `working_scale` | 12 | dp for indicator math |
| `price_scale` | per symbol | dp only; from config / tick size; **instance data — excluded from `config_version`** |
| `price_increment` | per symbol (optional) | the symbol's actual price step (e.g. `0.0005`); when set the core snaps stop/TP/reference levels to it (§7.4/§7.6); **instance data — excluded from `config_version`** |
| `ratio_serialization_scale` | 4 | dp, serialization only |
| `rounding_strategy` | half-up | explicit via helper |
| `config_version` | string (hash) | hash of the canonical **strategy parameters** (excludes per-symbol `price_scale`/`price_increment` instance data); matches the freeze lock (§10.4) |

**Validation (reject config otherwise):** all periods ≥ 1; `min_zone_height <= max_zone_height`; positive ATR multipliers; `price_scale` present for each configured symbol; and **`target_r > engine_net_floor` and `min_gross_rr > engine_net_floor`** (`engine_net_floor = 2.5`, recorded in config) so a typo cannot undercut the cushion. (The earlier `target_r >= min_gross_rr` rule is **dropped**: `min_gross_rr` applies to `structural` only, and equality in `fixed_r` produced a rounding-sensitive tautology, see §7.5.)

**Pre-registered backtest parameters** (recorded in the freeze lock, §10.4 — not strategy config): fee rate, entry slippage model, **separate worse stop-fill slippage**, borrow rate, the **entry fill rule** (the touching tick fills; **latency is a fill-model assumption, not a tick index** — a live feed would inject a time-based latency model instead), bootstrap block length / resample count / seed, the chosen `target_mode`, and the **in-sample/OOS boundary timestamp**.

**Deferred to transport phase:** live-price staleness (10s), closed-candle lateness alert (5m), signal TTL (60s), and all account-risk parameters (0.5% / 2% / 1%) — those are engine-side and out of scope.

---

## 4. Core input/output model

```rust
enum Input {
    ClosedCandle(Candle),   // structure, zones, indicators
    PriceTick(Tick),        // touch detection ONLY
    Resume,                 // restart / reconnect / data-gap (pure, no I/O)
}
// EngineAck and Clock are deferred to the transport phase (§12).

enum Output {
    ZoneEvent(ZoneEvent),   // created / invalidated / expired / consumed (with reason)
    EntrySignal(Signal),    // the only actionable output
    StateChange(StateChange),
}

// state is passed by reference; an error leaves the caller's state unchanged
// (validate-then-commit: the new state is produced only on success).
pub fn step(state: &StrategyState, input: Input, cfg: &Config)
    -> Result<(StrategyState, Vec<Output>), StrategyError>;
```

**Event-ordering contract (enforced by the caller/harness):** for each candle, all ticks belonging to that candle are delivered **first**, and the `ClosedCandle` event **last**. A zone activates only on the breakout candle's close, so ticks *inside* the breakout candle cannot touch it.

**Single touch code path:** the core detects touches **only** from `PriceTick`. Structure and zones come only from `ClosedCandle`. This is what makes the pure-core requirement pay off: backtest and live run identical logic.

**State is per symbol.** The core holds no cross-symbol logic; correlation caps and portfolio risk are engine-side (later phase). A different symbol is a different `StrategyState` instance. The per-symbol price scale is likewise **instance data** held on `Config` (for level rounding), and — unlike the strategy parameters — is excluded from `config_version` (§3).

**Typed errors (data integrity):** `StrategyError` includes `OutOfOrderCandle`, `DuplicateCandle`, `GappedCandle`, `MalformedCandle` (negative/zero, `high < low`, non-finite), **`MalformedTick`** (zero/negative price), **`OutOfOrderTick`** (timestamp earlier than the previous tick or the last closed candle), plus `DecimalOverflow`, `DivideByZero`, `ConfigInvalid`. Candle **and tick** validation lives **in the core** so backtest and live behave identically — the core catches violations of the event-ordering contract rather than trusting the caller. On any of these the state is unchanged and no output is produced.

**`ATR == 0` is in-band, not an error.** A flat warm-up series yields `ATR == 0`; it is emitted as a `StateChange` (reason `atr_zero`). It skips **only** BOS detection and zone creation — pivot confirmation and zone lifecycle still run on that candle — and the candle is **still recorded**. (Making it an `Err` would leave the candle unrecorded, so the next candle would read as `GappedCandle` and the stream would stall.)

**Gap recovery:** on `GappedCandle` the adapter **feeds the missing candles in order**, then resumes normal event delivery; the core treats the gap as an error only until the missing candles are supplied. Recovery never generates a historical entry (§7.2).

**`Resume` input (restart / reconnect / data-gap).** After any restart, reconnect, or data-gap the adapter injects `Input::Resume`, a pure core input (no wall-clock, no I/O) whose effect is: forget the last observed price (the next tick's approach side is unknown) and flag **every** `Active` zone as awaiting its first tick. A zone the price moved through during downtime therefore becomes a `missed_touch` (§7.2), never a stale entry, and the restart/gap path is testable in isolation. The harness (`run`) can inject `Resume` mid-stream, which makes the Phase 1 "≥ 3 restarts" exit criterion testable.

**State must be serializable and schema-versioned** (`STATE_SCHEMA_VERSION`) to support snapshot-resume replay equivalence (§10). Decimal scales are normalized on store so round-trips are byte-identical.

---

## 5. Market structure

- **Pivots (2/2):** swing high at `i` if `high[i] > high[i+1] && high[i] > high[i+2] && high[i] >= high[i-1] && high[i] >= high[i-2]`; swing low mirrored. With a non-strict left test and a strict right test, a plateau of equal highs (or lows) resolves to its **rightmost** member. A pivot is available only after its two right candles close.
- **BOS:** each confirmed swing level can be broken at most once. Bullish = a completed candle closes ≥ `bos_min_distance` above a BOS-eligible high; bearish mirrored. **ATR is computed from data strictly before the breakout candle closes (no look-ahead).**
- **Trend state machine:** `Unknown | Bullish | Bearish`, driven by BOS + valid high/low, not by counting pivot pairs. Any bullish BOS → `Bullish` and sets `valid_low`; any bearish BOS → `Bearish` and sets `valid_high`.
  - **valid_low (after bullish BOS):** most recent confirmed swing low formed after the broken swing high and before the breakout candle; else the most recent confirmed swing low before the broken swing high; else state stays `Unknown`. valid_high mirrored.
  - **Explicit same-candle branch:** if pivot confirmation and the BOS occur on the same candle (pivot available exactly on the breakout candle), the valid high/low is taken from candles strictly before the breakout candle; if none, fall back as above.
  - > **TO-CONFIRM (human gate):** §6.3 valid high/low is the spec's working definition. A human must verify it against the source video at **00:29–04:30** (or a transcript) **before** the OOS run; if it differs, capture the difference as a **config option** and document the deviation. Phase 0 does not perform this check.
- **Consequences:** only `Bullish` produces longs, only `Bearish` shorts, `Unknown` none. On trend flip, every `Active` zone in the now-wrong direction → `Invalidated` (reason `trend_flip`).
- **Restart / resume:** a `Resume` input (§4) clears the last observed price and re-arms first-tick observation for every `Active` zone, and nothing else. It never reconstructs structure, synthesizes pivots/BOS, or generates a historical touch; the resumed stream continues from the same structure and zone state.

---

## 6. Supply and demand zones

- **Demand zone (after bullish BOS):** search back up to `zone_search_lookback` completed candles before the breakout candle; select the most recent bearish candle (`close < open`); require breakout candle body ≥ `impulse_body_threshold`; zone = that candle's full range `[low, high]`; apply size limits and mitigation; activate when the breakout candle closes; record pre-breakout ATR as the zone's `A`. Supply mirrored after bearish BOS. No qualifying candle / any check fails → no zone, log reason.
- **Lifecycle states:** `Active | Consumed | Invalidated | Expired`. **In Phase 0 every non-`Active` state is terminal — `Invalidated | Expired | Consumed` never return to `Active`.** (The re-armable distinction from v2 depends on `EngineAck`/`Clock` and is inactive in Phase 0, per §0.4.) This removes the double-order ambiguity. Terminal transitions without a signal record a separate **`TerminalReason`** (`trend_flip | close_beyond_edge | expired`); **`ConsumeReason`** is reserved for signal/consumption cases (`signal_published | missed_touch | engine_* | expired_undelivered`).
- **Rules:** expire an untouched zone after `zone_expiry_candles`; invalidate a demand zone on a completed close below `zone_low` (supply mirrored above `zone_high`); invalidate opposite zones on trend flip; **candle-close wick reconciliation:** a still-`Active` zone whose candle range was reached (demand `candle.low <= zone.high`; supply `candle.high >= zone.low`) but for which no tick touch was observed that candle is consumed as `missed_touch` (§7.2), because polling can skip a wick; first touch only; **mark `Consumed` + persist the pending signal atomically before publishing** (persistence is Phase ≥ 2; Phase 0 only marks state in-memory and emits the signal). Never reactivate a terminal zone, including after restart.
- **Consumption reasons:** `signal_published`, `missed_touch`, `vetoed_obstacle_too_close`, `vetoed_no_obstacle`, `vetoed_invalid_levels`, `engine_accepted`, `engine_rejected_strategy`, `engine_rejected_transient`, `expired_undelivered`. Every terminal transition records a reason code. The `vetoed_*` reasons are the first-touch entry vetoes (§7.5): the veto itself consumes the zone, honoring "first touch only". (In Phase 0 all of these are terminal; the reason codes are carried for forward compatibility, not to enable re-arm.)
- **Mitigation (revisit before activation):** examine completed candles **strictly after** the zone candle and **strictly before** the breakout candle; skip the first `mitigation_grace_candles`; demand mitigated if `low <= zone_high` (supply if `high >= zone_low`); the zone candle and breakout candle are never counted.
- **Zone size limits (§7.5):** reject when `(zone_high − zone_low) / A` is outside `[min_zone_height, max_zone_height]`. **Both bounds are instrumented** (count how often each binds) — the 0.5 A minimum is expected to bind more often than the 3.0 A cap for single-candle zones. The 3.0 A cap is retained as spec'd pending backtest evidence.
- **Multiple zones:** select the one nearest to current price in the trend direction.

---

## 7. Entry signals, stop/target, obstacle check

### 7.1 Trigger
Long eligible when: trend `Bullish`; an `Active`, unconsumed demand zone exists; previous observed price was above `zone_high`; current price touches/crosses `zone_high` (first touch); no prior attempt; obstacle check passes. Short mirrored. Entry is a **touch trigger from `PriceTick`**, never a 4H close.

### 7.2 Missed touches
A zone is a `missed_touch` **unless the first observation is an approach from outside the zone**. Per direction, the first observation is missed iff the price is inside/beyond the zone (demand `price <= zone_high`; supply `price >= zone_low`) **and** the approach did not come from outside this tick — i.e. there is no trustworthy previous price (cold start / restart / reconnect / data-gap, §4) or the previous price was already inside/beyond (demand `prev <= zone_high`; supply `prev >= zone_low`). Equivalently: `missed = inside_or_beyond(price) && !(prev is Some(approach_from_outside))`, where the demand approach side is above `zone_high` and the supply approach side is below `zone_low`. A genuine approach from outside is left `Active` for the normal first-touch path (§7.1). When missed, do not signal; mark `Consumed` with reason `missed_touch`. Never emit historical touches.

**Candle-close reconciliation (a wick-miss that polling can miss).** Touch tracking is keyed strictly to **observed** ticks: any tick inside/beyond a zone during a candle records that zone for the candle. At each `ClosedCandle`, every still-`Active` zone whose range the candle reached but which had **no observed tick touch** that candle is consumed as `missed_touch` and emits the corresponding `ZoneEvent`. A zone whose touch *was* observed is already consumed by its signal or veto, so reconciliation never fires for it; the observed-touch ledger is reset for a newly created zone. Synthetic backtest paths include each candle's low and high, so on them the reconciliation is a no-op (the four fixtures are unchanged).

### 7.3 Signal price
The signal carries the observed trigger price + timestamp. The reference entry price is a reference, not a guaranteed fill; the engine validates the executable price.

### 7.4 Stop / target
Long: `stop_loss = L − stop_buffer·A`; `risk_t = T − stop_loss`; take-profit depends on mode.
Short: `stop_loss = H + stop_buffer·A`; `risk_t = stop_loss − T`.

**Increment snapping (when the symbol's `price_increment` is configured).** Each computed level is snapped to a valid multiple of the symbol's actual price step (§2, §3): the **stop away from** entry, the **take-profit toward** entry, and reference/obstacle levels to the **nearest**; `risk_t` is then recomputed from the snapped stop. This makes the published prices valid exchange multiples that the engine can re-validate. With no increment configured for the symbol, levels use dp `round_price(price_scale)` (Phase 0 behavior).

### 7.5 Obstacle check (the R:R filter)
For a long, obstacle = the **nearest** of: the nearest unbroken confirmed swing high above `T`; the **highest high over the window from the breakout candle through the latest completed candle** (this **includes the breakout candle itself**, whose extreme is often the nearest obstacle, and catches unconfirmed impulse extremes from fast pullbacks); the lower boundary of the nearest `Active` supply zone above `T`. Short mirrored (nearest swing low / the **lowest low over the same window including the breakout candle** / upper boundary of the nearest `Active` demand zone below `T`). `obstacle_rr = |obstacle − T| / risk_t` (correct sign per direction; a sign error would reject every short — covered by tests).

**Target modes:**
- `fixed_r`: `take_profit = T + target_r·risk_t`; **the only gate is the obstacle veto — veto if `obstacle_rr < target_r`** (target never placed past an obstacle). There is deliberately **no `min_gross_rr` comparison here**: gross R:R ≡ `target_r` by construction, so that gate would be a tautology (and rounding-sensitive at equality). The cushion in this mode is the config choice `target_r` > engine net floor.
- `structural`: `take_profit = obstacle − structural_buffer` (long); **veto if the resulting gross R:R < `min_gross_rr`**; if no obstacle exists and `allow_no_obstacle`, fall back to `fixed_r`.
- If no obstacle exists and `allow_no_obstacle` is false → veto.

**The gate evaluates the rounded levels that go into the payload** (`trigger_price`, `stop_loss`, `take_profit`, obstacle — each rounded to the symbol's price scale, or snapped to its `price_increment` when configured), not raw working-scale values. A coarse-price-scale test (§10.1) guards this.

A **first-touch veto consumes the zone** and records the attempt, with a persisted reason: `vetoed_obstacle_too_close` / `vetoed_no_obstacle`, or `vetoed_invalid_levels` when the risk is non-positive (or the stop rounds onto the wrong side of the trigger). It is **not** left `Active`. Because the veto is the zone's first touch (§6 "first touch only"), the §7.1 first-touch rule holds and a later/deeper tick cannot re-trigger it. (The alternative — leave the zone `Active` so a later or deeper touch re-arms the veto — was **rejected**: it would grant a hidden second chance and re-emit on every re-cross.)

> **Cost cushion:** the analyzer's chosen floor (`target_r` in `fixed_r`, `min_gross_rr` in `structural`) sits above the engine's 2.5 net floor; the difference absorbs fees + slippage + borrow. The harness computes the realized cushion and applies the 1.5× cost gate (§10.4).

### 7.6 Self-consistent levels
`stop_loss` and `take_profit` are computed from the **rounded/snapped** `trigger_price` and `stop_loss`; ratios are derived from those **payload levels** (so the gate and the serialized fields use the same numbers), then serialized at `ratio_serialization_scale`. Fields reconcile exactly. When a symbol's `price_increment` is configured the trigger, stop, take-profit, and obstacle are each snapped to valid multiples first (§7.4), so every published price is a valid exchange multiple.

---

## 8. Harness

### 8.1 TickSource
Trait producing an ordered `(PriceTick, ClosedCandle)` event stream with the §4 ordering. Implementations:
- **`OneMinuteExpansion`** — fetch/cached 1m candles; `ClosedCandle` events are **4H aggregates built from the 1m series**, aligned to the 4H grid (boundaries at 4H multiples of the epoch, i.e. 00:00/04:00/… UTC). Before trusting a 1m run, **validate the 1m-derived 4H series against the independently fetched 4H series**.
- **`Ohlcv4hSynthesis`** — expand each cached 4H candle into an ordered path; works with any OHLCV file.

The core never knows which source is used. Resolution is stamped on every report; cross-resolution comparison is refused unless explicitly forced.

> **CLI resolution (Phase 0).** The `OneMinuteExpansion` source and 1m data are implemented and used by `calibrate` (1m→4H aggregate validation, bucketed by the 4H grid cell), but the headline `backtest`/`count-signals` CLI is **4H-synthesis-only**: there is deliberately **no `--resolution` flag**, so cross-resolution comparison is not exposed. Wiring 1m into the headline run is deferred — it first requires the perturbation diagnostics to fall back to 4H synthesis to stay within budget (§10.4).

### 8.2 Data acquisition
- `fetch-data` is a **separate subcommand** that pages KuCoin `GET /api/v1/market/candles` by time (`startAt`/`endAt`, seconds; max 1500 records/request; spot has **no historical range limit**; **throttled to respect rate limits**), writes a cached file, and prints its **dataset hash**. Backtest never touches the network.
- Deep 1m history **is obtainable by pagination** (≈ 700 throttled requests/pair for 2 years), so 1m-grade data can be a **Phase 0** deliverable (`fetch-data --interval 1min`) rather than deferred. If 1m is not fetched, headline results come from 4H synthesis and 1m is used only for calibration (§8.5).
- Every report records the **dataset hash** and the resolution.

### 8.3 Intrabar path policy
Two **global** policies — the harness does not need to know trade direction:
- `low_first`: O→L→H→C
- `high_first`: O→H→L→C

**Both are run on every backtest.** Structure and zones depend only on closed candles, so the signal set is nearly identical across policies; the difference is touch/exit ordering. The **adversarial composite** is assembled in reporting (longs from `low_first`, shorts from `high_first`). Determinism: no RNG; replay must be byte-identical. The chosen policy is recorded per run.

### 8.4 Position simulation (harness-only, engine-standing-in)
- **Entry:** the fill happens **at the touching tick's price** (the first `PriceTick` that crosses the zone boundary, i.e. `ticks[0]` of the signal's tick stream), not at the boundary price, plus a configurable **entry slippage** (`entry_slip_bps`). **Latency is a fill-model assumption, not a tick index:** Phase 0 does not model a tick-count delay. **4H synthesis expands each candle into 4 synthetic ticks** (`O→L/H→H/L→C`); that grid is a coarse, fictitious intrabar sequence, not the true 1m ordering, so 4H-synthesis results are labeled **latency-insensitive** (real wall-time latency is not recoverable from it). A live feed would inject a **time-based** latency model here (e.g. the poll interval from §0.1) instead; true sub-candle latency needs the 1m source (§8.1).
- **Unfillable entries:** if the entry price is already at/beyond the stop or the target, the signal is **not filled** (`Unfillable`) — otherwise a long that gapped past its target would report a "take profit" at a loss (and one that opened through its stop a "stop" at a gain).
- **Exit:** stop or take-profit resolved tick-by-tick along the same path, with costs applied. **Stop fills use a separate, worse slippage** than target fills (stops are hit in fast moves). A stop's cost **worsens** the loss: long `R = (fill − entry − cost)/risk`, short `R = (entry − fill − cost)/risk`.
- **Pre-registered sim parameters:** entry slippage, stop slippage, and fee/borrow rates are all fixed in the freeze lock (§10.4), not chosen after seeing results.
- **Open-position policy:** if a signal fires while a position is already open on that symbol, **skip it and count `skipped_open_position`** (otherwise the sim would take overlapping trades the engine's caps would block). These skips count against the trade bar.
- **Unresolved positions:** a signal whose position neither stops nor takes profit before the data ends is **excluded from the trade ledger and the verdict trade set**, reported separately (`RunResult.unresolved`), and still occupies the one-position slot to the end of data.
- **Units:** report primarily in **R-multiples**. The sim tracks **holding time** so borrow interest can be applied.
- Results are labeled an **approximation of engine net-R:R / fill logic**.

### 8.5 Reporting
- Trade ledger in `Decimal`; statistics in `f64`.
- **Both target modes on identical data, side by side** (evidence-based mode choice).
- **Both path policies**, with the adversarial composite.
- **Cushion calibration:** realized average cost per trade in R (fees + slippage + interest). Flag if > 0.5 R → the **analyzer's floor (`target_r` in `fixed_r`, `min_gross_rr` in `structural`) is too low**.
- **1m calibration (when 1m data exists):** report how often the assumed intrabar order was wrong on the overlapping window. Stated in the report header.
- **Header** encodes: symbol(s), resolution, dataset hash, `config_version`, path policy, target mode, and the resolution caveat.

---

## 9. Signal payload (Phase 0)

Versioned, serializable; decimals as fixed-scale strings; timestamps as `i64` UTC ms internally (ISO-8601 at the edge).

```
schema_version, event_type = "strategy.entry_signal",
signal_id, strategy = "supply_demand_4h", strategy_version, config_version,
symbol, market = "margin", direction, timeframe, trend_state,
zone_id, zone_low, zone_high, zone_height_atr, atr,
trigger_price, reference_entry_price, stop_loss, take_profit,
target_mode, target_r, min_gross_rr, gross_rr,
structure_obstacle_price, obstacle_rr,
triggered_at, candle_timestamp
```

- `zone_id` is deterministic: `"{symbol}:{breakout_ts}:{zone_candle_ts}:{direction}"`. `signal_id` = `zone_id` + attempt number (stable across retries).
- `expires_at` is **omitted in Phase 0** (TTL depends on `Clock`).
- Never include balances or credentials. The reference entry is not an execution instruction.
- The payload carries both `target_mode` and `min_gross_rr` (per the decoupling).

---

## 10. Testing & Phase 0 exit criteria

### 10.1 Unit tests (§15.1 coverage)
Pivots incl. tie rule and confirmation delay; BOS thresholds + no-lookahead ATR; ATR seed and first-candle TR; trend machine incl. valid high/low + reversal + same-candle branch; zone invalidation on flip; zone selection/size/mitigation incl. grace; lifecycle incl. terminal-state rules; first-touch/approach/missed — including a zone activated while the price is already inside it (→ `missed_touch`, not a signal) and the candle-close wick reconciliation (an overlapping candle with no observed tick touch consumes the zone as `missed_touch`); the `Resume` input (clears the last price, re-arms first-tick observation); SL/TP/obstacle incl. no-obstacle and short-sign; serialization + stable IDs; **typed data-integrity errors** (out-of-order / duplicate / gapped / malformed candles) leave state unchanged; **config validation** (periods, height bounds, ATR multipliers, `price_scale`); and a **coarse-price-scale test** (a large-tick symbol) confirming the obstacle/R:R gate and the serialized levels agree after rounding.

### 10.2 Replay equivalence (strengthened)
A pure function replayed on one machine usually matches itself, so add:
1. **Determinism:** same input stream twice → byte-identical outputs.
2. **Snapshot resume:** serialize `StrategyState` mid-stream (schema-versioned), deserialize, inject `Input::Resume`, and compare against an uninterrupted run at candle boundaries — this is the §13 restart property. A dedicated downtime case moves the price through a zone between snapshot and resume: the first tick back is a `missed_touch`, never a signal.
3. **Cross-platform golden hash:** the committed golden file `tests/golden/outs.hash` is compared on the two-leg **architecture** matrix in `ci.yml` (x86_64 `ubuntu-latest` + arm64 `ubuntu-24.04-arm`). The test never writes the golden file: a missing file fails the leg (no write-if-empty fallback).
4. **No `HashMap`/`HashSet` iteration in `strategy/`** (CI-enforced; use `BTreeMap`/`Vec`).

### 10.3 Harness tests
Event ordering (ticks before `ClosedCandle`); `low_first` vs `high_first`; both modes; `skipped_open_position` counting; cushion-calibration output; cross-resolution guard; fetch-data paging/rate-limit behavior (mocked).

### 10.4 Backtest bar (pre-registered — fixed before the OOS run)
Exploration is free **in-sample** (counts only, no PnL, to size pairs/history). The bar is frozen **before the OOS segment is first run**, and the harness enforces it:

**In-sample/OOS boundary (pre-registered):** the split is a fixed UTC timestamp recorded in the lock **before any exploration**: data before the boundary is in-sample; data at/after it is OOS (the most recent **≥ 30%** of history). Because the boundary is a lock input, it cannot be moved after seeing OOS results.

**Step 0 (before any PnL):** count **in-sample signals only** (counts, no PnL) to size pairs and history against the **≥ 200 executed-trade** bar below; do not lower the bar afterwards.

- **Parameter-freeze lock file:** `lock = hash(config_version + backtest_params)` — a single hash over the canonical config **plus** the pre-registered backtest parameters (not "the config_version" alone). The OOS run **refuses to start** without a matching lock and **logs how many variants were tried**.
- **Lock contents (pre-registered):** the **in-sample/OOS boundary timestamp**, chosen `target_mode` (mode comparison is **in-sample only** — selecting on OOS is a hidden degree of freedom), fee rate, entry slippage model, **separate worse stop-fill slippage**, borrow rate, the **entry fill rule** (the touching tick fills; latency is a fill-model assumption, not a tick index), and the bootstrap settings below.
- **OOS is one-shot:** every OOS invocation is logged; a second run on the same data is refused without an explicit `--oos-consumed` override, and an overridden run is **stamped non-confirmatory** in its report and **records the earlier run's hash**. After a failure, further tuning requires **fresh data**.

Proposed bar (thresholds are proposals; **"no edge" is a valid, documented Phase 0 outcome**):

- **≥ 200 OOS executed trades**, where "executed" **excludes `skipped_open_position`** and **excludes `Unresolved`** positions (both reported separately, never in the trade set). If the in-sample **signal count** (counts only) shows this is unreachable with 3 pairs / 2 years, **size pairs and history up before the OOS run** — do not lower the bar afterwards.
- **OOS mean net R > 0**, with the **95% lower bound > 0** under a **time-blocked bootstrap** computed on the **adversarial composite** (longs from `low_first`, shorts from `high_first`) — not on a single policy.
- **Bootstrap settings fixed:** calendar-based block length (proposed default **2 weeks**), fixed resample count (proposed 10,000), fixed seed.
- **Positive under both path policies** (a separate gate from the composite bound).
- **Cost stress:** the composite mean is still positive at **1.5× costs**.
- **Perturbation gates are diagnostics, not selectors:**
  - one parameter at a time, ±20%, with **≥ 70% of cells positive**;
  - **seeded joint perturbation**, 200 draws, each parameter uniform in ±20%, with **≥ 60% positive**.
- **Diagnostics performance budget:** the perturbation diagnostics run on **4H synthesis only** (not tick granularity) and must complete within a pre-registered wall-clock budget (proposed: the full OOS + diagnostics run under a few hours per pair). If 1m data is present, diagnostics still use 4H synthesis to stay within budget.
- **Sizing guidance from the bar itself:** with +3R / −1R, per-trade σ ≈ 1.8R. At n = 200 a plain 95% lower bound above zero needs a mean ≈ **+0.25R** (≈ 31% win vs. 25% break-even); a **block** bootstrap needs more. At n = 100 it is ≈ **+0.36R**. Size the sample accordingly.
- **Regime & concentration:** report **long/short** and **per-pair** splits. Define regime **objectively** (proposed: close vs. its 200-day average). Flag an OOS segment containing **only one regime** as **inconclusive**. **No single pair may supply more than 50% of the total R.**

Data/cost requirements: ≥ 2 years across ≥ 3 liquid pairs spanning up + down regimes (subject to the in-sample count above); OOS ≥ 30% (most recent); costs = fees + slippage + borrow interest; intrabar ambiguity = stop-first; paper-trading (≥ 4 weeks, ≥ 30 signals) is a **later phase**, not Phase 0.

### 10.5 Phase 0 exit criteria
- The crate builds with `--no-default-features` (core only) and with the `harness` feature; `cargo clippy` passes with the float deny; the `syn` token-level purity test and the comment-stripped greps pass; `cargo test` green; all wired into `make ci` (`strategy-analyzer` added to `PACKAGES`, plus a `--no-default-features` core build step).
- Replay equivalence (all four checks) passes. Check 3 (cross-platform golden hash) runs on the ≥ 2-entry **architecture** matrix (x86_64 `ubuntu-latest` + arm64 `ubuntu-24.04-arm`) in `ci.yml`; the committed golden file is never written by the test (a missing file fails the leg).
- A backtest report is produced and documented per §10.4, with its resolution + dataset-hash caveats stated.

---

## 11. Observability (core-adjacent)
Structured logs (harness side; the core returns typed `Output`/`StrategyError` rather than logging): candle gaps/late/stale, pivot confirmations, BOS, trend transitions, zone create/reject/invalidate/expire/consume with reasons, obstacle vetoes, entry triggers — each with symbol, timeframe, zone ID, signal ID, `strategy_version`, `config_version`. Never log secrets. (There is no verified metrics system in the repo; revisit before Phase ≥ 2.)

---

## 12. Deferred items (later phases — recorded here for traceability)

1. **Transport:** deliver signals to the engine. The existing mechanism is **engine-triggered request/response** over Redis Pub/Sub (engine PUBs `tickers:analyze` → analyzer replies `signals:result`), not a passive push with no trigger. Contract migration path from `Vec<TradeSignal>` to the new schema, and a compatibility window.
2. **Persistence + outbox** and its storage owner; a migration runner for `trade-engine`; a processed-signal-id dedup table.
3. **Live price feed** for tick-based touches (no websockets exist today) — or a documented v1 fallback to candle-close entries.
4. **Engine-side risk** the v2 doc assumed but which does not exist: daily-DD, total open-risk, correlation caps, net-R:R with fees/slippage/borrow, protective-order placement + fill capture.
5. **`EngineAck` / `Clock`** inputs, re-arm policy, and signal TTL.
6. **Analyzer topology:** whether this crate replaces or coexists with the Python `process/analyzer/`, and which backtest harness is authoritative.
7. **Deep 1m acquisition** is a data-fetch cost, not a blocker. **Decision:** Phase 0 *does* include the `OneMinuteExpansion` `TickSource` and `fetch-data --interval 1min` (paged by time; ~700 throttled requests/pair for 2 years). The only thing deferred is running that acquisition over full multi-year history — which is a runtime cost, not a code deliverable. When 1m data is present, Task 31's intrabar calibration runs against it; otherwise headline results come from 4H synthesis (with the caveat stamped).

---

## 13. Open items to confirm (all currently agreed)
1. Zone max height stays 3.0 A, configurable and instrumented (both bounds).
2. §6.3 valid high/low implemented as spec'd; verification against the source video (**00:29–04:30**, or a transcript) is a **human gate** before OOS, and any deviation is captured as a **config option**.
3. `EngineAck`/`Clock` deferred; re-arm and TTL absent in Phase 0.
4. Backtest bar pre-registered with a config-hash freeze lock (one-shot OOS).
5. Path policy = two global policies run both, composite in reporting.
6. Open-position policy = skip + `skipped_open_position`.
7. `step` takes `&StrategyState`, validate-then-commit; typed data-integrity errors (§4).
8. `min_gross_rr` gates `structural` only; `fixed_r` uses the obstacle veto alone; gates evaluate rounded payload levels (§3, §7).
9. Obstacle set includes the window from the breakout candle through the latest completed candle (highest high / lowest low, **including the breakout candle**) (§7.5).
10. Per-symbol state; no cross-symbol logic in the core (§4).
11. `bin/backtest.rs` requires the `harness` feature; the cross-platform golden hash runs on the two-leg architecture matrix (x86_64 + arm64) in `ci.yml`.
12. `config_version` = hash of canonical config; Decimal scale normalized on state store.
13. `ATR == 0` handled in-band as a `StateChange` (skips **only** BOS + zone creation; pivots/lifecycle still run; candle still recorded); `GappedCandle` recovery = adapter feeds the missing candles in order, then resumes (§4).
14. Tick validation lives in the core: `MalformedTick`, `OutOfOrderTick` (§4).
15. Entry fills at the touching tick (`ticks[0]`); **latency is a fill-model assumption, not a tick index** (a live feed would inject a time-based model); 4H-synthesis results labeled latency-insensitive (§8.4).
16. 1m mode builds 4H `ClosedCandle`s from the 1m series, aligned to the 4H grid, validated against the independently fetched 4H series (§8.1).
17. `lock = hash(config_version + backtest_params)`; `--oos-consumed` overrides are stamped non-confirmatory and record the earlier run. Bootstrap block = 2 weeks; "most of the R" = a single pair > 50%. Perturbation diagnostics run on 4H synthesis only, within a wall-clock budget (§10.4).
