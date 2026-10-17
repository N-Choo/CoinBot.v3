# strategy-analyzer

Phase 0 of the supply-and-demand 4H strategy, as a **pure deterministic core** plus an
**offline backtest/replay harness**. This crate is deliberately not wired to the trade engine
yet; it exists to freeze the strategy semantics, prove replay determinism, and produce a
pre-registered backtest report.

Spec: `docs/superpowers/specs/2026-10-17-rust-strategy-analyzer-design.md`.
Plan: `docs/superpowers/plans/2026-10-17-strategy-analyzer-phase0.md`.

## Architecture: a one-way boundary

```
strategy/   (pure core)   ── no I/O, no f64, no wall-clock, no HashMap iteration
    ▲
    │  harness → strategy  (one-way dependency)
    │
harness/    (feature "harness")  ── data loading, tick synthesis, position sim, reporting
```

- `src/strategy/` is the core: `Decimal`-only math, typed `StrategyError`, no panics, and an
  `#![deny(clippy::float_arithmetic)]` at `strategy/mod.rs`. It knows nothing about where ticks
  come from, the resolution, or the fill model.
- `src/harness/` may use `f64` for reporting statistics, but those values never feed back into
  the core. The trade ledger stays in `Decimal`.
- `src/bin/backtest.rs` requires the `harness` feature and is the only place that touches the
  network (via the `fetch-data` subcommand). `backtest` never fetches.

The core exposes:

```rust
step(&StrategyState, Input, &Config) -> Result<(StrategyState, Vec<Output>), StrategyError>
```

with **validate-then-commit** semantics: on any error the caller's state is untouched (a new
state is produced only on success).

## Building: core-only vs. harness

```bash
# Pure core only — no clap/ureq/csv compiled, `backtest` bin skipped.
cargo build -p strategy-analyzer --no-default-features

# Core + harness + the `backtest` bin.
cargo build -p strategy-analyzer --all-features

# The purity gate (float/hash-map greps + both builds). Run from the repo root.
make strategy-check
```

## Running the backtest CLI — run it from the crate directory

**Fixture paths are resolved relative to the current working directory**, so the CLI must be run
from the crate directory:

```bash
cd process/strategy-analyzer
cargo run -p strategy-analyzer --features harness --bin backtest -- \
    backtest --data tests/fixtures/signal_4h.csv
```

Running it from the repo root instead makes `tests/fixtures/...` resolve against the root and
fail. (Tests get this for free: Cargo runs integration tests with the crate directory as cwd.)

### `CARGO_MANIFEST_DIR` policy

`CARGO_MANIFEST_DIR` is a **compile-time** path. It may be used **only** to supply a *built-in
default* fixture path (a file shipped inside this crate). It must **never** be used to rewrite or
resolve a **user-supplied** `--data` path: a user path is a runtime string and belongs to the
user's filesystem, not to the build machine's source tree. Concretely, `--data` is opened
verbatim, cwd-relative, like any other CLI file argument.

### Subcommands

```bash
# Fetch OHLCV and cache it to a CSV, then print its dataset hash. Network only here.
# Refuses any range overlapping the OOS window until the freeze lock exists (--lock).
backtest fetch-data --symbol XRP-USDT --interval 4hour \
    --start-s <unix_seconds> --end-s <unix_seconds> --out data/xrp_4h.csv [--lock lock.json]

# Seal the pre-registered freeze lock (git commit + dataset hashes + boundary + constants).
backtest freeze-lock --data XRP-USDT=data/xrp_4h.csv --price-scale XRP-USDT=4 \
    --mode both --out lock.json

# Count-only in-sample sizing run (§10.4 Step 0): per-symbol signal counts and reason counts.
# Refuses any row at/after the in-sample/OOS boundary. No PnL, no trades, no verdict.
backtest count-signals --data XRP-USDT=data/xrp_4h.csv --price-scale XRP-USDT=4

# Validate a 1m series' 4H aggregate against an independently fetched 4H series (offline).
# 1m candles are bucketed by their 4H grid cell (open_time_ms / 4h), so a mid-bucket start
# still aligns. Refuses the run above AGG_TOLERANCE; never silently patches.
backtest calibrate --one-minute data/xrp_1m.csv --fetched-4h data/xrp_4h.csv

# Run the backtest offline against one or more cached datasets.
backtest backtest --data XRP-USDT=data/xrp_4h.csv --data BTC-USDT=data/btc_4h.csv \
    --price-scale XRP-USDT=4 --price-scale BTC-USDT=2 \
    [--policy both|low_first|high_first] [--mode both|fixed_r|structural] \
    [--lock lock.json] [--oos] [--oos-consumed]
```

`--data SYMBOL=PATH` and `--price-scale SYMBOL=DP` are **repeatable**, and every symbol needs an
explicit price scale (there is **no** silent default: a missing scale is an error, and neither the
symbol set nor the scale is hard-coded). `--policy both` and `--mode both` (the defaults) run both
path policies and both target modes side by side. `fetch-data` is throttled and retries with
exponential backoff on rate limits, and writes a provenance sidecar (`<out>.provenance.json`:
symbol, interval, range, row count, dataset hash) next to the cache; a malformed exchange timestamp
is an error (never silently coerced to `0`).

**Price increment is core instance data, not (yet) a CLI flag.** `Config::price_increment` (the
symbol's actual price step, e.g. `0.0005`) is set per symbol when the core is embedded; the
`backtest`/`count-signals` CLI does not expose a `--price-increment`, so those runs use dp
`--price-scale` rounding. The Phase 1 adapter fills the increment from `/api/v2/symbols`
`priceIncrement` (live spec §3b/§7); a dedicated flag is a small follow-up if the harness needs it.

**Resolution: `backtest`/`count-signals` are 4H-synthesis-only in Phase 0** (spec §8.1's
cross-resolution comparison is not exposed on the CLI, so there is no `--resolution` flag). The
`OneMinuteExpansion` `TickSource` exists in `harness::tick_source` and 1m data is used by
`calibrate`; wiring 1m into the headline backtest CLI is deferred (it would need diagnostics to drop
back to 4H synthesis to stay within budget, per spec §10.4).

## Numeric & determinism policy (short form)

- **No floats in the core.** CI runs a `syn`-based token-level purity test over `src/strategy/`
  that fails on any `f32`/`f64` token; fast greps (comment-stripped) are kept only as pre-filters
  for `f32|f64` and `HashMap|HashSet`. The token test catches a real `f64` on a line that also
  carries a trailing comment, while an `f64` mentioned only in a comment or a string literal is
  not flagged. Use `BTreeMap`/`Vec`; randomized iteration order is hidden nondeterminism.
- **One rounding helper:** `decimal::round_price(value, price_scale)`, half-up
  (`rust_decimal`'s default is half-even, so it is requested explicitly). Price scale is
  **per-symbol** (ideally the exchange tick size), applied only to levels that appear in signals.
  When a symbol's actual **`price_increment`** is configured (e.g. KuCoin `0.0005`, a step-5 grid a
  dp scale cannot express), the core snaps stop/take-profit/reference levels to it
  (`decimal::snap_to_increment`): stop **away from** entry, take-profit **toward** entry, so
  published prices are valid exchange multiples; with no increment it uses dp `round_price`.
- **Ratios are compared unrounded**, at working scale; they are rounded (toward zero, 4 dp) only
  when serialized or reported. This avoids `2.99996` rounding up to pass a `3.0` floor.
- **Self-consistent payload:** stop / take-profit / ratios derive from the *already-rounded*
  trigger and stop, so the serialized fields reconcile exactly.
- **ATR:** Wilder smoothing seeded with the simple mean of the first 14 true ranges; the first
  candle's true range is `high − low`. `ATR == 0` on a flat series is **not** an error — it is
  emitted in-band as a `StateChange` so the stream does not stall.
- **Serialization:** `Decimal`s serialize as fixed-scale **strings**, never JSON numbers.
  Timestamps are `i64` UTC milliseconds internally.
- **`config_version`** = hash of the canonical **strategy parameters** (per-symbol `price_scale` /
  `price_increment` instance data excluded); it feeds the freeze lock.
- **Replay:** no RNG; replaying the same events is byte-identical, and a mid-stream state
  snapshot that is serialized and resumed produces the same final state.

## OOS freeze lock & the one-shot guard

The offline backtest bar is pre-registered. The lock is a canonical JSON document — not a bare
hash — so it **includes** the git commit, the per-symbol dataset hashes, the in-sample/OOS boundary
timestamp, and the pre-registered constants (min executed trades, bootstrap block/resamples/seed,
aggregate tolerance, perturbation thresholds/draws/seed). Its identity is the sha256 of that JSON,
so changing any of those inputs changes the lock.

- **The boundary binds.** A pre-registered UTC boundary (`lock::OOS_BOUNDARY_MS`) splits in-sample
  from OOS. `count-signals` refuses any row at/after it, `fetch-data` refuses ranges overlapping it
  until the lock exists, and `backtest` **refuses to print a verdict** for any dataset whose time
  range touches the OOS window unless `--oos` and a matching `--lock` are supplied.
- **Non-OOS runs are labelled `EXPLORATORY`** so a `Pass` cannot be mistaken for a confirmatory
  result.
- **OOS names exactly one mode.** `--oos` requires a single explicit `--mode` (`fixed_r` or
  `structural`) that **matches the frozen lock**; `--mode both` is refused (each mode is compared
  in-sample only), and a mode that differs from the locked one is refused by name.
- **OOS is one-shot and append-only.** Every OOS invocation is appended to `<lock>.oos.log`
  (recording the dataset hashes and verdict), so the first confirmatory run's record is preserved.
  A second OOS run on the same data is **refused** unless `--oos-consumed` is passed; an overridden
  run is stamped **NON-CONFIRMATORY** and reports the earlier run's verdict. After a failure,
  further tuning requires fresh data.
- **The regime fails closed.** `verdict()` takes a three-state regime (`Both`/`OneRegimeOnly`/
  `Unknown`); anything but `Both` yields `VerdictOutcome::Inconclusive`, never a pass.

## CI: a two-architecture matrix

The Rust job runs on a two-leg **architecture** matrix — `x86_64` (`ubuntu-latest`) and `arm64`
(`ubuntu-24.04-arm`) — so the cross-platform golden hash (`replay_equivalence`) is compared on both
architectures on every run. The golden file `tests/golden/outs.hash` is committed and is **never
written by the test**: a missing golden file fails the leg.

Caveat: `ubuntu-24.04-arm` is a GitHub-hosted arm64 runner, and its availability depends on the
account/plan (it may also need enabling for forks or private repositories). If the arm64 runner is
unavailable in a given context, that leg must be enabled or replaced with an equivalent arm64
runner — the cross-platform golden-hash check is not meaningful with a single architecture.

## Open item to confirm (§6.3)

The §6.3 **valid high / valid low** definition (which confirmed pivot anchors a trend, and the
fallback when none exists) is implemented as the spec's working definition, but is marked
**TO-CONFIRM**. This is a **human gate**: watch the source video at **00:29–04:30** (or use a
transcript) **before** the OOS run. If the video differs, capture the difference as a config
option and document the deviation. Phase 0 does not perform that video check.
