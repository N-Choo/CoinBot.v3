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
backtest fetch-data --symbol XRP-USDT --interval 4hour \
    --start-s <unix_seconds> --end-s <unix_seconds> --out data/xrp_4h.csv

# Run the backtest offline against a cached dataset.
backtest backtest --data <path> [--policy both|low_first|high_first] \
    [--mode both|fixed_r|structural] [--lock <path>] [--oos] [--oos-consumed]
```

`--policy both` and `--mode both` (the defaults) run both path policies and both target modes
side by side. `fetch-data` is throttled and retries with exponential backoff on rate limits.

## Numeric & determinism policy (short form)

- **No floats in the core.** CI runs a `syn`-based token-level purity test over `src/strategy/`
  that fails on any `f32`/`f64` token; fast greps (comment-stripped) are kept only as pre-filters
  for `f32|f64` and `HashMap|HashSet`. The token test catches a real `f64` on a line that also
  carries a trailing comment, while an `f64` mentioned only in a comment or a string literal is
  not flagged. Use `BTreeMap`/`Vec`; randomized iteration order is hidden nondeterminism.
- **One rounding helper:** `decimal::round_price(value, price_scale)`, half-up
  (`rust_decimal`'s default is half-even, so it is requested explicitly). Price scale is
  **per-symbol** (ideally the exchange tick size), applied only to levels that appear in signals.
- **Ratios are compared unrounded**, at working scale; they are rounded (toward zero, 4 dp) only
  when serialized or reported. This avoids `2.99996` rounding up to pass a `3.0` floor.
- **Self-consistent payload:** stop / take-profit / ratios derive from the *already-rounded*
  trigger and stop, so the serialized fields reconcile exactly.
- **ATR:** Wilder smoothing seeded with the simple mean of the first 14 true ranges; the first
  candle's true range is `high − low`. `ATR == 0` on a flat series is **not** an error — it is
  emitted in-band as a `StateChange` so the stream does not stall.
- **Serialization:** `Decimal`s serialize as fixed-scale **strings**, never JSON numbers.
  Timestamps are `i64` UTC milliseconds internally.
- **`config_version`** = hash of the canonical `Config` JSON; it feeds the freeze lock.
- **Replay:** no RNG; replaying the same events is byte-identical, and a mid-stream state
  snapshot that is serialized and resumed produces the same final state.

## OOS freeze lock & the one-shot guard

The offline backtest bar is pre-registered. OOS refuses to start without a matching
`lock = hash(config_version + backtest_params)` file. Every OOS invocation is logged; a second
run on the same data is **refused** unless `--oos-consumed` is passed, and an overridden run is
stamped **NON-CONFIRMATORY** and records the earlier run's hash. After a failure, further tuning
requires fresh data.

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
