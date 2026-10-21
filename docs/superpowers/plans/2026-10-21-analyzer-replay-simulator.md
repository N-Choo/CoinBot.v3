# Analyzer-RS Terminal Replay Simulator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `analyzer-replay`, a Rust binary that replays the live `CandleDriver` per symbol and writes one self-contained Terminal HTML report with simulated trades (engine-mirroring, no fees).

**Architecture:** Extract shared `.env`/identity code without changing live behavior; add a validated offline dataset layer; build the replay event stream from the existing driver; run a direction-aware trade simulator; serialize everything into a single HTML file whose UI is the approved Terminal demo.

**Tech Stack:** Rust 2021, `rust_decimal`, `tokio` (live only), `ureq`/`chrono` (exchange), `serde`/`serde_json`, `clap`, `csv`, `sha2`, `thiserror`.

**Scope:** Replay + simulation + report only. No fees, no live gRPC changes, no trade-engine changes.

**Reference spec:** `docs/superpowers/specs/2026-10-21-analyzer-replay-simulator-design.md`

---

## File map

| File | Responsibility |
|---|---|
| `src/config_env.rs` | `.env` loading + `strategy_from_env` (shared by live and harness) |
| `src/identity.rs` | `config_version` + stable `signal_id` (shared by live::grpc and harness) |
| `src/live/config.rs` | Re-export/consume `config_env`; no behavior change |
| `src/live/grpc.rs` | Consume `identity`; no behavior change |
| `src/harness/mod.rs` | Harness exports |
| `src/harness/dataset.rs` | CSV/fetch loading, validation, provenance |
| `src/harness/replay.rs` | Driver replay → candles/RSI/SMA/events/signals |
| `src/harness/sim.rs` | Direction-aware engine-mirroring trade simulator |
| `src/harness/stats.rs` | Drawdown, profit factor, exposure, averages |
| `src/harness/report.rs` | Payload + self-contained HTML writer |
| `src/bin/analyzer-replay.rs` | CLI |
| `templates/replay.html` | Terminal UI (from `design-demos/simulator/a-terminal.html`) |
| `tests/identity.rs` | Identity stability |
| `tests/dataset.rs` | Validation + provenance |
| `tests/replay_payload.rs` | Replay events + signal parity |
| `tests/sim.rs` | Execution, accounting, boundary tests |
| `tests/replay_report.rs` | Payload + HTML report |

---

### Task 1: Extract shared config and identity (no behavior change)

**Files:**
- Create: `process/analyzer-rs/src/config_env.rs`
- Create: `process/analyzer-rs/src/identity.rs`
- Modify: `process/analyzer-rs/src/lib.rs`
- Modify: `process/analyzer-rs/src/live/config.rs`
- Modify: `process/analyzer-rs/src/live/grpc.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Test: `process/analyzer-rs/tests/identity.rs`

- [ ] **Step 1: Write the failing test**

`tests/identity.rs`:

```rust
use analyzer_rs::identity::{config_version, signal_id};
use analyzer_rs::strategy::config::Config;
use analyzer_rs::strategy::types::{Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;

fn sample() -> Signal {
    Signal {
        ticker: "BTC-USDT".into(),
        action: Direction::Long,
        confidence: Decimal::new(8, 1),
        entry_price: Decimal::from(100),
        reason: "regular_bullish_divergence".into(),
        sl_price: Decimal::from(98),
        tp_price: Decimal::from(104),
        divergence_index: 12,
        divergence_kind: DivergenceKind::RegularBullish,
        divergence_open_time_ms: 1_700_000_000_000,
        divergence_age_candles: 5,
        signal_candle_open_time_ms: 1_700_086_400_000,
    }
}

#[test]
fn config_version_changes_with_strategy_fields() {
    let base = config_version(&Config::default());
    let mut changed = Config::default();
    changed.atr_sl_multiplier = Decimal::new(25, 1);
    assert_ne!(base, config_version(&changed));
    assert_eq!(base, config_version(&Config::default()));
}

#[test]
fn signal_id_is_stable_and_kind_sensitive() {
    let signal = sample();
    let version = config_version(&Config::default());
    let id = signal_id(&signal, &version);
    assert!(id.starts_with("v1:"));
    assert_eq!(id, signal_id(&signal, &version));

    let mut other = signal.clone();
    other.divergence_kind = DivergenceKind::HiddenBullish;
    assert_ne!(id, signal_id(&other, &version));
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --test identity
```

Expected: compile error (`identity` module missing).

- [ ] **Step 3: Add the shared modules**

`Cargo.toml`: make `sha2` **and `dotenvy` normal dependencies** (the shared `config_env` module is compiled unconditionally, so it cannot rely on an optional dependency). Add `harness` and `clap`:

```toml
[features]
default = []
exchange = ["dep:ureq", "dep:serde_json", "dep:chrono"]
live = ["exchange", "dep:tokio", "dep:chrono"]
grpc = ["live", "dep:tonic", "dep:prost"]
harness = ["exchange", "dep:clap"]

[dependencies]
sha2 = "0.10"
dotenvy = { workspace = true }
clap = { version = "4", features = ["derive"], optional = true }
```

Remove `dep:dotenvy` from `live` and `dep:sha2` from `grpc`; both are now always available.

`src/lib.rs`:

```rust
pub mod strategy;
pub mod driver;
pub mod config_env;
pub mod identity;
#[cfg(feature = "exchange")]
pub mod exchange;
#[cfg(feature = "live")]
pub mod live;
```

`src/config_env.rs`:

```rust
use crate::strategy::config::Config;
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum ConfigEnvError {
    #[error("invalid {0}={1}")]
    Invalid(&'static str, String),
    #[error("invalid strategy config: {0}")]
    Strategy(String),
}

pub fn load_dotenv() {
    let candidates = [
        std::env::current_dir().ok().map(|dir| dir.join(".env")),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env")),
    ];
    for path in candidates.into_iter().flatten() {
        if path.is_file() {
            let _ = dotenvy::from_path(path);
            return;
        }
    }
}

pub fn env_string(key: &'static str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn strategy_from_env() -> Result<Config, ConfigEnvError> {
    let mut cfg = Config::default();
    cfg.rsi_period = parse("RSI_PERIOD", cfg.rsi_period)?;
    cfg.rsi_order = parse("RSI_DIVERGENCE_ORDER", cfg.rsi_order)?;
    cfg.price_diff_threshold =
        parse_decimal("PRICE_DIFF_THRESHOLD", cfg.price_diff_threshold)?;
    cfg.sma_enabled = parse_bool("SMA_ENABLED", cfg.sma_enabled)?;
    cfg.sma_period = parse("SMA_PERIOD", cfg.sma_period)?;
    cfg.atr_period = parse("ATR_PERIOD", cfg.atr_period)?;
    cfg.atr_sl_multiplier = parse_decimal("ATR_SL_MULTIPLIER", cfg.atr_sl_multiplier)?;
    cfg.atr_tp_multiplier = parse_decimal("ATR_TP_MULTIPLIER", cfg.atr_tp_multiplier)?;
    cfg.atr_pct_high = parse_decimal("ATR_PCT_HIGH", cfg.atr_pct_high)?;
    cfg.atr_high_adjust = parse_decimal("ATR_HIGH_ADJUST", cfg.atr_high_adjust)?;
    cfg.atr_sl_max_pct = parse_decimal("ATR_SL_MAX_PCT", cfg.atr_sl_max_pct)?;
    cfg.window = parse("ANALYZER_WINDOW_CANDLES", cfg.window)?;
    cfg.validate()
        .map_err(|error| ConfigEnvError::Strategy(error.to_string()))?;
    Ok(cfg)
}

pub fn parse_bool_str(key: &'static str, value: &str) -> Result<bool, ConfigEnvError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigEnvError::Invalid(key, value.to_string())),
    }
}

pub fn parse_decimal_str(key: &'static str, value: &str) -> Result<Decimal, ConfigEnvError> {
    Decimal::from_str(value.trim()).map_err(|_| ConfigEnvError::Invalid(key, value.to_string()))
}

pub fn parse_optional_usize_str(
    key: &'static str,
    value: &str,
) -> Result<Option<usize>, ConfigEnvError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse()
        .map(Some)
        .map_err(|_| ConfigEnvError::Invalid(key, value.to_string()))
}

fn parse<T: FromStr>(key: &'static str, default: T) -> Result<T, ConfigEnvError> {
    match env_string(key) {
        Some(value) => value
            .parse()
            .map_err(|_| ConfigEnvError::Invalid(key, value)),
        None => Ok(default),
    }
}

fn parse_decimal(key: &'static str, default: Decimal) -> Result<Decimal, ConfigEnvError> {
    match env_string(key) {
        Some(value) => parse_decimal_str(key, &value),
        None => Ok(default),
    }
}

fn parse_bool(key: &'static str, default: bool) -> Result<bool, ConfigEnvError> {
    match env_string(key) {
        Some(value) => parse_bool_str(key, &value),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_values_are_trimmed() {
        assert_eq!(
            parse_decimal_str("X", " 1.5 ").unwrap(),
            Decimal::new(15, 1)
        );
    }

    #[test]
    fn booleans_are_case_and_space_insensitive() {
        assert!(parse_bool_str("X", " ON ").unwrap());
        assert!(!parse_bool_str("X", " Off ").unwrap());
    }

    #[test]
    fn empty_optional_is_none() {
        assert_eq!(parse_optional_usize_str("X", "  ").unwrap(), None);
        assert_eq!(parse_optional_usize_str("X", " 7 ").unwrap(), Some(7));
    }

    #[test]
    fn env_string_trims_and_treats_empty_as_unset() {
        std::env::set_var("ANALYZER_TEST_TRIMMED", " 42 ");
        std::env::set_var("ANALYZER_TEST_EMPTY", "   ");
        assert_eq!(env_string("ANALYZER_TEST_TRIMMED"), Some("42".into()));
        assert_eq!(env_string("ANALYZER_TEST_EMPTY"), None);
        std::env::remove_var("ANALYZER_TEST_TRIMMED");
        std::env::remove_var("ANALYZER_TEST_EMPTY");
    }

    #[test]
    fn parse_uses_trimmed_env_and_empty_falls_back() {
        std::env::set_var("ANALYZER_TEST_INT", " 14 ");
        std::env::set_var("ANALYZER_TEST_INT_EMPTY", " ");
        assert_eq!(parse("ANALYZER_TEST_INT", 8usize).unwrap(), 14);
        assert_eq!(parse("ANALYZER_TEST_INT_EMPTY", 8usize).unwrap(), 8);
        std::env::remove_var("ANALYZER_TEST_INT");
        std::env::remove_var("ANALYZER_TEST_INT_EMPTY");
    }

    #[test]
    fn decimal_and_bool_env_are_trimmed() {
        std::env::set_var("ANALYZER_TEST_DEC", " 1.5 ");
        std::env::set_var("ANALYZER_TEST_BOOL", " ON ");
        assert_eq!(
            parse_decimal("ANALYZER_TEST_DEC", Decimal::ZERO).unwrap(),
            Decimal::new(15, 1)
        );
        assert!(parse_bool("ANALYZER_TEST_BOOL", false).unwrap());
        std::env::remove_var("ANALYZER_TEST_DEC");
        std::env::remove_var("ANALYZER_TEST_BOOL");
    }
}
```

`src/identity.rs`:

```rust
use crate::strategy::config::Config;
use crate::strategy::types::{Direction, Signal};
use sha2::{Digest, Sha256};

pub fn config_version(cfg: &Config) -> String {
    format!(
        "rsi-v1|w={}|rsi={}:{}|pdt={}|sma={}:{}|atr={}:{}:{}:{}:{}:{}",
        cfg.window,
        cfg.rsi_period,
        cfg.rsi_order,
        cfg.price_diff_threshold,
        cfg.sma_enabled,
        cfg.sma_period,
        cfg.atr_period,
        cfg.atr_sl_multiplier,
        cfg.atr_tp_multiplier,
        cfg.atr_pct_high,
        cfg.atr_high_adjust,
        cfg.atr_sl_max_pct,
    )
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn signal_id(signal: &Signal, config_version: &str) -> String {
    const SEP: u8 = 0;
    let mut hasher = Sha256::new();
    hasher.update(b"analyzer-signal-v1");
    hasher.update([SEP]);
    hasher.update(signal.ticker.as_bytes());
    hasher.update([SEP]);
    hasher.update([match signal.action {
        Direction::Long => b'B',
        Direction::Short => b'S',
    }]);
    hasher.update([SEP]);
    hasher.update(signal.divergence_kind.as_str().as_bytes());
    hasher.update([SEP]);
    hasher.update(signal.divergence_open_time_ms.to_be_bytes());
    hasher.update([SEP]);
    hasher.update(config_version.as_bytes());
    let digest = hasher.finalize();
    let mut id = String::from("v1:");
    for byte in digest {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}
```

- [ ] **Step 4: Rewire live to the shared modules**

`src/live/config.rs`: delete the local `load_dotenv`, `strategy_from_env`, and the `parse_*` helpers. Re-export and adapt:

```rust
pub use crate::config_env::load_dotenv;

pub fn strategy_from_env() -> Result<Config, LiveConfigError> {
    crate::config_env::strategy_from_env()
        .map_err(|error| LiveConfigError::Invalid("strategy", error.to_string()))
}
```

Keep `LiveConfig` and its parsing as-is. Remove now-unused imports.

`src/live/grpc.rs`: delete the local `config_version` and `signal_id`; import:

```rust
pub use crate::identity::config_version;
use crate::identity::signal_id;
```

Remove the local `Sha256`/`Digest` imports if unused. `to_proto_signal` keeps calling `signal_id(signal, config_version)`.

- [ ] **Step 5: Capture the live baseline, then run the tests**

Before changing code, capture the live baseline so the extraction is provably behavior-preserving:

```bash
cargo test -p analyzer-rs --features grpc 2>&1 | tee /tmp/analyzer-rs-live-before.txt
cargo test -p analyzer-rs --features grpc --test signal_parity 2>&1 | tee /tmp/analyzer-rs-parity-before.txt
```

After the extraction:

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --test identity
cargo test -p analyzer-rs
cargo test -p analyzer-rs --features grpc 2>&1 | tee /tmp/analyzer-rs-live-after.txt
cargo test -p analyzer-rs --features grpc --test signal_parity 2>&1 | tee /tmp/analyzer-rs-parity-after.txt
cargo clippy -p analyzer-rs --features grpc --all-targets --no-deps -- -D warnings
```

Then diff the summaries:

```bash
diff <(grep -E 'test result|running [0-9]+ tests' /tmp/analyzer-rs-live-before.txt)      <(grep -E 'test result|running [0-9]+ tests' /tmp/analyzer-rs-live-after.txt)
diff <(grep -E 'test result' /tmp/analyzer-rs-parity-before.txt)      <(grep -E 'test result' /tmp/analyzer-rs-parity-after.txt)
```

Expected: no diff; the live-feature and signal-parity suites still pass with the same counts. The harness test alone is not sufficient evidence that live behavior is unchanged.

- [ ] **Step 6: Commit**

```bash
git add process/analyzer-rs
git commit -m "refactor(analyzer-rs): share config env and signal identity"
```

---

### Task 2: Validated dataset layer

**Files:**
- Create: `process/analyzer-rs/src/harness/mod.rs`
- Create: `process/analyzer-rs/src/harness/dataset.rs`
- Modify: `process/analyzer-rs/src/lib.rs`
- Test: `process/analyzer-rs/tests/dataset.rs`

- [ ] **Step 1: Write the failing test**

`tests/dataset.rs`:

```rust
#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, DatasetError};
use rust_decimal::Decimal;

fn write_csv(name: &str, rows: &[(&str, &str, &str, &str, &str)]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let mut text = String::from("open_time_ms,Open,High,Low,Close,Volume\n");
    for (t, o, h, l, c) in rows {
        text.push_str(&format!("{t},{o},{h},{l},{c},1\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn loads_and_hashes_a_valid_csv() {
    let path = write_csv(
        "analyzer-rs-valid.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 3);
    assert_eq!(dataset.provenance.gaps, 0);
    assert_eq!(dataset.provenance.content_sha256.len(), 64);
    let again = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.provenance.content_sha256, again.provenance.content_sha256);
}

#[test]
fn rejects_bad_ohlc() {
    let path = write_csv(
        "analyzer-rs-bad-ohlc.csv",
        &[("1700000000000", "100", "99", "101", "100")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::InvalidOhlc { .. }));
}

#[test]
fn rejects_a_timeframe_mismatch() {
    let path = write_csv(
        "analyzer-rs-tf.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000900000", "101", "103", "100", "102"),
        ],
    );
    // The rows are 15 minutes apart, so they cannot form a D1 series.
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::TimeframeMismatch { .. }));
}

#[test]
fn counts_gaps_and_enforces_the_gap_limit() {
    let path = write_csv(
        "analyzer-rs-gap.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    // Duplicate row is rejected first.
    assert!(matches!(
        load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err(),
        DatasetError::Duplicate { .. }
    ));

    let path = write_csv(
        "analyzer-rs-gap2.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700259200000", "101", "103", "100", "102"),
        ],
    );
    let dataset = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.provenance.gaps, 2);
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 1, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::GapTooLarge { .. }));
}

#[test]
fn loads_the_lowercase_synthetic_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/synthetic_1day.csv");
    let dataset = load_csv("SYNTH-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 400);
    assert_eq!(dataset.provenance.gaps, 0);
}

#[test]
fn rejects_a_reversed_csv() {
    let path = write_csv(
        "analyzer-rs-reversed.csv",
        &[
            ("1700086400000", "101", "103", "100", "102"),
            ("1700000000000", "100", "102", "99", "101"),
        ],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(error, DatasetError::OutOfOrder { .. }));
}

#[test]
fn drops_candles_at_the_close_boundary() {
    let third = 1700172800000i64;
    let path = write_csv(
        "analyzer-rs-boundary.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let boundary = third + Timeframe::D1.millis();
    let dropped = load_csv("TEST-USDT", &path, Timeframe::D1, boundary, 0, 0, 0, 0).unwrap();
    assert_eq!(dropped.candles.len(), 2);
    assert_eq!(dropped.candles.last().unwrap().open_time_ms, 1700086400000);

    let kept = load_csv(
        "TEST-USDT",
        &path,
        Timeframe::D1,
        boundary + 1,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    assert_eq!(kept.candles.len(), 3);
    assert_eq!(kept.candles.last().unwrap().open_time_ms, third);
}

#[test]
fn enforces_the_warmup_window() {
    let path = write_csv(
        "analyzer-rs-short.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
            ("1700172800000", "102", "104", "101", "103"),
        ],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 3, 0).unwrap_err();
    assert!(matches!(
        error,
        DatasetError::TooShort { got: 3, need: 5 }
    ));
}

#[test]
fn reads_columns_by_header_name() {
    let path = std::env::temp_dir().join("analyzer-rs-reordered.csv");
    std::fs::write(
        &path,
        "\u{feff} Volume ,High, open_time_ms , Low ,Close,Open\n\
         1,102,1700000000000,99,101,100\n\
         1,103,1700086400000,100,102,101\n",
    )
    .unwrap();
    let dataset =
        load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(dataset.candles.len(), 2);
    assert_eq!(dataset.candles[0].open_time_ms, 1700000000000);
    assert_eq!(dataset.candles[0].open, Decimal::from(100));
    assert_eq!(dataset.candles[0].high, Decimal::from(102));
    assert_eq!(dataset.candles[0].low, Decimal::from(99));
    assert_eq!(dataset.candles[0].close, Decimal::from(101));
    assert_eq!(dataset.candles[0].volume, Decimal::from(1));
}

#[test]
fn hashes_ignore_decimal_scale_but_not_values() {
    let scaled = write_csv(
        "analyzer-rs-hash-scale-a.csv",
        &[
            ("1700000000000", "100.0", "102.00", "99.000", "101.0"),
            ("1700086400000", "101.00", "103.0", "100.000", "102.00"),
        ],
    );
    let plain = write_csv(
        "analyzer-rs-hash-scale-b.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102"),
        ],
    );
    let scaled = load_csv("TEST-USDT", &scaled, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    let plain = load_csv("TEST-USDT", &plain, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap();
    assert_eq!(
        scaled.provenance.content_sha256,
        plain.provenance.content_sha256
    );

    let changed = write_csv(
        "analyzer-rs-hash-changed.csv",
        &[
            ("1700000000000", "100", "102", "99", "101"),
            ("1700086400000", "101", "103", "100", "102.5"),
        ],
    );
    let changed = load_csv(
        "TEST-USDT",
        &changed,
        Timeframe::D1,
        i64::MAX,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    assert_ne!(
        scaled.provenance.content_sha256,
        changed.provenance.content_sha256
    );
}

#[test]
fn rejects_non_positive_prices() {
    let path = write_csv(
        "analyzer-rs-non-positive.csv",
        &[("1700000000000", "0", "102", "99", "101")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(
        error,
        DatasetError::InvalidOhlc { why, .. } if why == "non-positive price"
    ));
}

#[test]
fn rejects_low_above_high() {
    let path = write_csv(
        "analyzer-rs-low-high.csv",
        &[("1700000000000", "100", "101", "102", "100")],
    );
    let error = load_csv("TEST-USDT", &path, Timeframe::D1, i64::MAX, 0, 0, 0, 0).unwrap_err();
    assert!(matches!(
        error,
        DatasetError::InvalidOhlc { why, .. } if why.contains("bound")
    ));
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --features harness --test dataset
```

Expected: compile error (`harness` missing).

- [ ] **Step 3: Implement the dataset layer**

`src/lib.rs`: add the harness module declaration:

```rust
#[cfg(feature = "harness")]
pub mod harness;
```

`src/harness/mod.rs`:

```rust
pub mod dataset;
```

`src/harness/dataset.rs`:

```rust
use crate::exchange::kucoin::{fetch_klines, ExchangeError};
use crate::exchange::timeframes::Timeframe;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("csv error: {0}")]
    Csv(String),
    #[error("exchange error: {0}")]
    Exchange(#[from] ExchangeError),
    #[error("invalid OHLC at {open_time_ms}: {why}")]
    InvalidOhlc { open_time_ms: i64, why: String },
    #[error("out-of-order candle at {got_ms} after {prev_ms}")]
    OutOfOrder { got_ms: i64, prev_ms: i64 },
    #[error("duplicate candle at {0}")]
    Duplicate(i64),
    #[error(
        "timeframe mismatch: spacing {spacing_ms} is not a multiple of the interval {interval_ms}"
    )]
    TimeframeMismatch { spacing_ms: i64, interval_ms: i64 },
    #[error("gap too large at {open_time_ms}: {missing} missing candles")]
    GapTooLarge { open_time_ms: i64, missing: i64 },
    #[error("only {got} closed candles (need >= {need})")]
    TooShort { got: usize, need: usize },
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestedRange {
    pub limit: usize,
    pub timeframe: String,
    pub as_of_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveRange {
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub source: String,
    pub requested: RequestedRange,
    pub effective_range: EffectiveRange,
    pub content_sha256: String,
    pub fetched_at_ms: Option<i64>,
    pub gaps: usize,
}

#[derive(Debug, Clone)]
pub struct Dataset {
    pub symbol: String,
    pub candles: Vec<Candle>,
    pub provenance: Provenance,
}

#[allow(clippy::too_many_arguments)]
pub fn load_csv(
    symbol: &str,
    path: &Path,
    timeframe: Timeframe,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    window: usize,
    requested_limit: usize,
) -> Result<Dataset, DatasetError> {
    let mut reader =
        csv::Reader::from_path(path).map_err(|error| DatasetError::Csv(error.to_string()))?;
    let headers = reader
        .headers()
        .map_err(|error| DatasetError::Csv(error.to_string()))?
        .clone();
    let index = |name: &str| -> Result<usize, DatasetError> {
        headers
            .iter()
            .position(|header| {
                header
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .eq_ignore_ascii_case(name)
            })
            .ok_or_else(|| DatasetError::Csv(format!("missing column: {name}")))
    };
    let open_time_index = index("open_time_ms")?;
    let open_index = index("open")?;
    let high_index = index("high")?;
    let low_index = index("low")?;
    let close_index = index("close")?;
    let volume_index = index("volume")?;
    let mut candles = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|error| DatasetError::Csv(error.to_string()))?;
        let field = |index: usize, name: &str| -> Result<Decimal, DatasetError> {
            Decimal::from_str(row.get(index).unwrap_or_default()).map_err(|error| {
                DatasetError::Csv(format!("column {name} at index {index}: {error}"))
            })
        };
        candles.push(Candle {
            open_time_ms: row
                .get(open_time_index)
                .unwrap_or_default()
                .parse()
                .map_err(|error| {
                    DatasetError::Csv(format!(
                        "column open_time_ms at index {open_time_index}: {error}"
                    ))
                })?,
            open: field(open_index, "open")?,
            high: field(high_index, "high")?,
            low: field(low_index, "low")?,
            close: field(close_index, "close")?,
            volume: field(volume_index, "volume")?,
        });
    }
    validate(
        symbol,
        candles,
        timeframe,
        as_of_ms,
        settle_ms,
        max_gap_candles,
        window,
        format!("csv:{}", path.display()),
        None,
        requested_limit,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn fetch_dataset(
    symbol: &str,
    timeframe: Timeframe,
    limit: usize,
    window: usize,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    base_url: &str,
) -> Result<Dataset, DatasetError> {
    let candles = fetch_klines(base_url, symbol, timeframe, window + limit)?;
    validate(
        symbol,
        candles,
        timeframe,
        as_of_ms,
        settle_ms,
        max_gap_candles,
        window,
        "kucoin".into(),
        Some(crate::identity::now_ms()),
        limit,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate(
    symbol: &str,
    mut candles: Vec<Candle>,
    timeframe: Timeframe,
    as_of_ms: i64,
    settle_ms: i64,
    max_gap_candles: i64,
    window: usize,
    source: String,
    fetched_at_ms: Option<i64>,
    requested_limit: usize,
) -> Result<Dataset, DatasetError> {
    for candle in &candles {
        if candle.open <= Decimal::ZERO
            || candle.high <= Decimal::ZERO
            || candle.low <= Decimal::ZERO
            || candle.close <= Decimal::ZERO
        {
            return Err(DatasetError::InvalidOhlc {
                open_time_ms: candle.open_time_ms,
                why: "non-positive price".into(),
            });
        }
        if candle.high < candle.open
            || candle.high < candle.close
            || candle.low > candle.open
            || candle.low > candle.close
            || candle.high < candle.low
        {
            return Err(DatasetError::InvalidOhlc {
                open_time_ms: candle.open_time_ms,
                why: "high/low do not bound open/close".into(),
            });
        }
    }
    for pair in candles.windows(2) {
        let prev = &pair[0];
        let next = &pair[1];
        if next.open_time_ms == prev.open_time_ms {
            return Err(DatasetError::Duplicate(next.open_time_ms));
        }
        if next.open_time_ms < prev.open_time_ms {
            return Err(DatasetError::OutOfOrder {
                got_ms: next.open_time_ms,
                prev_ms: prev.open_time_ms,
            });
        }
        let spacing = next.open_time_ms - prev.open_time_ms;
        if spacing % timeframe.millis() != 0 {
            return Err(DatasetError::TimeframeMismatch {
                spacing_ms: spacing,
                interval_ms: timeframe.millis(),
            });
        }
    }
    candles.retain(|candle| timeframe.is_closed(candle.open_time_ms, as_of_ms, settle_ms));
    if candles.len() < window + 2 {
        return Err(DatasetError::TooShort {
            got: candles.len(),
            need: window + 2,
        });
    }
    let mut gaps = 0usize;
    for pair in candles.windows(2) {
        let spacing = pair[1].open_time_ms - pair[0].open_time_ms;
        let missing = spacing / timeframe.millis() - 1;
        if missing > 0 {
            gaps += missing as usize;
            if max_gap_candles > 0 && missing > max_gap_candles {
                return Err(DatasetError::GapTooLarge {
                    open_time_ms: pair[1].open_time_ms,
                    missing,
                });
            }
        }
    }
    let content_sha256 = hash_candles(&candles);
    let provenance = Provenance {
        source,
        requested: RequestedRange {
            limit: requested_limit,
            timeframe: timeframe.as_kucoin().to_string(),
            as_of_ms,
        },
        effective_range: EffectiveRange {
            first_open_time_ms: candles[0].open_time_ms,
            last_open_time_ms: candles[candles.len() - 1].open_time_ms,
            candles: candles.len(),
        },
        content_sha256,
        fetched_at_ms,
        gaps,
    };
    Ok(Dataset {
        symbol: symbol.to_string(),
        candles,
        provenance,
    })
}

fn hash_candles(candles: &[Candle]) -> String {
    let mut hasher = Sha256::new();
    for candle in candles {
        hasher.update(candle.open_time_ms.to_be_bytes());
        hasher.update(candle.open.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.high.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.low.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.close.normalize().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(candle.volume.normalize().to_string().as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut text = String::new();
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
```

Note: `crate::live::grpc::now_ms()` is only available under `grpc`. Move `now_ms` to `identity.rs` as `pub fn now_ms()` and use it here, so harness does not depend on `grpc`. Update `live::grpc` to re-export `now_ms` from `identity`.

- [ ] **Step 4: Run the tests**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test dataset
cargo test -p analyzer-rs --features grpc
```

Expected: dataset tests pass; grpc tests still pass.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): add validated dataset layer"
```

---

### Task 3: Replay builder and signal parity

**Files:**
- Create: `process/analyzer-rs/src/harness/replay.rs`
- Modify: `process/analyzer-rs/src/harness/mod.rs`
- Test: `process/analyzer-rs/tests/replay_payload.rs`

- [ ] **Step 1: Write the failing test**

`tests/replay_payload.rs`:

```rust
#![cfg(feature = "harness")]

use analyzer_rs::driver::CandleDriver;
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::replay::{build_replay, ReplayEvent};
use analyzer_rs::identity::{config_version, signal_id};
use analyzer_rs::strategy::config::Config;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn replay_matches_the_driver_and_uses_live_identity() {
    let cfg = Config::default();
    let dataset = load_csv(
        "SYNTH-USDT",
        &fixture("synthetic_1day.csv"),
        Timeframe::D1,
        i64::MAX,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    let replay = build_replay(&dataset, &cfg).unwrap();

    // Same warm-up and emitted sequence as a direct driver run.
    let mut driver = CandleDriver::new("SYNTH-USDT", cfg.clone());
    let mut expected_signals = 0usize;
    for candle in &dataset.candles {
        for event in driver.push_closed(candle.clone()).unwrap() {
            if matches!(event, analyzer_rs::driver::DriverEvent::Signal { .. }) {
                expected_signals += 1;
            }
        }
    }
    let actual_signals = replay
        .signals
        .iter()
        .filter(|signal| signal.candle_index >= cfg.window)
        .count();
    assert_eq!(actual_signals, expected_signals);
    assert!(actual_signals > 0);

    let version = config_version(&cfg);
    for signal in &replay.signals {
        assert!(signal.signal_id.starts_with("v1:"));
        assert_eq!(
            signal.signal_id,
            signal_id(&signal.signal, &version),
            "identity must match live"
        );
    }

    assert!(replay
        .events
        .iter()
        .any(|event| matches!(event, ReplayEvent::Warmup { .. })));
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --features harness --test replay_payload
```

Expected: compile error (`replay` missing).

- [ ] **Step 3: Implement the replay builder**

`src/harness/replay.rs`:

```rust
use crate::driver::{CandleDriver, DriverEvent, SuppressReason};
use crate::harness::dataset::Dataset;
use crate::identity::{config_version, signal_id};
use crate::strategy::config::Config;
use crate::strategy::error::Result as StrategyResult;
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::indicators::sma::sma;
use crate::strategy::types::{Candle, Signal};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayEvent {
    Warmup { candle_index: usize, seeded_keys: usize },
    Signal { candle_index: usize, signal_index: usize },
    Suppressed {
        candle_index: usize,
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct ReplaySignal {
    pub candle_index: usize,
    pub signal: Signal,
    pub signal_id: String,
}

#[derive(Debug, Clone)]
pub struct Replay {
    pub symbol: String,
    pub candles: Vec<Candle>,
    pub rsi: Vec<Option<Decimal>>,
    pub sma: Vec<Option<Decimal>>,
    pub events: Vec<ReplayEvent>,
    pub signals: Vec<ReplaySignal>,
    pub provenance: crate::harness::dataset::Provenance,
}

fn suppress_reason(reason: SuppressReason) -> &'static str {
    match reason {
        SuppressReason::Duplicate => "duplicate",
        SuppressReason::BelowWatermark => "below_watermark",
        SuppressReason::TooOld => "too_old",
    }
}

pub fn build_replay(dataset: &Dataset, cfg: &Config) -> StrategyResult<Replay> {
    let window = cfg.window;
    let mut driver = CandleDriver::new(&dataset.symbol, cfg.clone());
    let mut events = Vec::new();
    let mut signals = Vec::new();
    let version = config_version(cfg);

    for (index, candle) in dataset.candles.iter().enumerate() {
        let driver_events = driver.push_closed(candle.clone())?;
        for event in driver_events {
            match event {
                DriverEvent::Warmed { seeded_keys } => events.push(ReplayEvent::Warmup {
                    candle_index: index,
                    seeded_keys,
                }),
                DriverEvent::Signal { signal, .. } => {
                    signals.push(ReplaySignal {
                        candle_index: index,
                        signal_id: signal_id(&signal, &version),
                        signal,
                    });
                    events.push(ReplayEvent::Signal {
                        candle_index: index,
                        signal_index: signals.len() - 1,
                    });
                }
                DriverEvent::Suppressed { reason, .. } => events.push(ReplayEvent::Suppressed {
                    candle_index: index,
                    reason: suppress_reason(reason).to_string(),
                }),
            }
        }
    }

    let closes: Vec<Option<Decimal>> = dataset.candles.iter().map(|c| Some(c.close)).collect();
    let rsi_values = rsi(&closes, cfg.rsi_period)?;
    let sma_values = sma(&closes, cfg.sma_period)?;

    let _ = window;
    Ok(Replay {
        symbol: dataset.symbol.clone(),
        candles: dataset.candles.clone(),
        rsi: rsi_values,
        sma: sma_values,
        events,
        signals,
        provenance: dataset.provenance.clone(),
    })
}
```

Note: `DriverEvent::Suppressed` carries the key, not the divergence index. The report only needs the reason; the `divergence_index` field can be derived from `key.divergence_open_time_ms` if needed later, or the event can drop it. Keep the struct simple: use `reason` and omit `divergence_index` to avoid fabricating data. Adjust the test accordingly (no assertion on it).

- [ ] **Step 4: Run the tests**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test replay_payload
cargo test -p analyzer-rs --features grpc
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): build replay events and signals from the live driver"
```

---

### Task 4: Trade simulator and statistics

**Files:**
- Create: `process/analyzer-rs/src/harness/sim.rs`
- Create: `process/analyzer-rs/src/harness/stats.rs`
- Modify: `process/analyzer-rs/src/harness/mod.rs`
- Test: `process/analyzer-rs/tests/sim.rs`

- [ ] **Step 1: Write the failing test**

`tests/sim.rs`:

```rust
#![cfg(feature = "harness")]

use analyzer_rs::harness::sim::{simulate, SimSignal};
use analyzer_rs::strategy::types::{Candle, Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;

fn d(value: i64) -> Decimal { Decimal::from(value) }

fn candle(i: usize, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        open_time_ms: i as i64 * 86_400_000,
        open: d(open),
        high: d(high),
        low: d(low),
        close: d(close),
        volume: d(1),
    }
}

fn signal(index: usize, side: Direction, entry: i64, sl: i64, tp: i64) -> SimSignal {
    SimSignal {
        candle_index: index,
        signal: Signal {
            ticker: "TEST-USDT".into(),
            action: side,
            confidence: Decimal::new(6, 1),
            entry_price: d(entry),
            reason: "regular_bullish_divergence".into(),
            sl_price: d(sl),
            tp_price: d(tp),
            divergence_index: 0,
            divergence_kind: DivergenceKind::RegularBullish,
            divergence_open_time_ms: index as i64 * 86_400_000,
            divergence_age_candles: 0,
            signal_candle_open_time_ms: index as i64 * 86_400_000,
        },
    }
}

#[test]
fn long_entry_fills_at_next_open_and_exits_at_tp() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
    ];
    let outcome = simulate(&candles, &[signal(0, Direction::Long, 100, 98, 104)], 1000, 0);
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].fill, d(101));
    assert_eq!(outcome.trades[0].exit, d(104));
    assert_eq!(outcome.trades[0].reason, "TP");
}

#[test]
fn short_pnl_is_profitable_and_losing() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 94, 95),
        candle(2, 95, 100, 94, 99),
        candle(3, 100, 108, 99, 107),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Short, 100, 105, 95),
            signal(2, Direction::Short, 99, 105, 94),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len(), 2);
    assert!(outcome.trades[0].pnl > Decimal::ZERO);
    assert!(outcome.trades[1].pnl < Decimal::ZERO);
}

#[test]
fn sl_first_when_both_levels_are_in_range() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 106, 97, 105),
    ];
    let outcome = simulate(&candles, &[signal(0, Direction::Long, 100, 98, 104)], 1000, 0);
    assert_eq!(outcome.trades[0].reason, "SL");
    assert_eq!(outcome.trades[0].exit, d(98));
}

#[test]
fn invalid_geometry_is_rejected() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
    ];
    let outcome = simulate(&candles, &[signal(0, Direction::Long, 100, 102, 104)], 1000, 0);
    assert!(outcome.trades.is_empty());
    assert_eq!(outcome.invalid_levels, 1);
}

#[test]
fn funds_limit_concurrent_positions_and_release_on_next_candle() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 90, 120),
            signal(1, Direction::Long, 100, 90, 120),
            signal(2, Direction::Long, 100, 90, 120),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len() + outcome.open_positions.len(), 2);
    assert_eq!(outcome.skipped_funds, 1);
}

#[test]
fn summary_stats_use_realized_pnl() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 94, 99),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 98, 104),
            signal(1, Direction::Short, 105, 108, 95),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(99), 4);
    assert_eq!(summary.trades, 2);
    assert_eq!(summary.wins + summary.losses + summary.breakeven, 2);
    assert!(summary.max_drawdown >= Decimal::ZERO);
    assert!(summary.time_in_market_pct >= Decimal::ZERO);
}

#[test]
fn gap_beyond_the_stop_fills_at_the_open_for_long_and_short() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 95, 96, 90, 92), // long opens below SL 98
    ];
    let long = simulate(&candles, &[signal(0, Direction::Long, 100, 98, 110)], 1000, 0);
    assert_eq!(long.trades[0].reason, "gap_sl");
    assert_eq!(long.trades[0].fill, d(95));
    assert_eq!(long.trades[0].exit, d(95));

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 106, 107, 105, 106), // short opens above SL 105
    ];
    let short = simulate(&candles, &[signal(0, Direction::Short, 100, 105, 90)], 1000, 0);
    assert_eq!(short.trades[0].reason, "gap_sl");
    assert_eq!(short.trades[0].fill, d(106));
    assert_eq!(short.trades[0].exit, d(106));
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --features harness --test sim
```

Expected: compile error (`sim` missing).

- [ ] **Step 3: Implement the simulator**

`src/harness/sim.rs`:

```rust
use crate::strategy::types::{Candle, Direction, Signal};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct SimSignal {
    pub candle_index: usize,
    pub signal: Signal,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub side: Direction,
    pub signal_entry: Decimal,
    pub fill: Decimal,
    pub e: Decimal,
    pub sl: Decimal,
    pub tp: Decimal,
    pub entry_index: usize,
}

#[derive(Debug, Clone)]
pub struct ClosedTrade {
    pub side: Direction,
    pub signal_entry: Decimal,
    pub fill: Decimal,
    pub exit: Decimal,
    pub e: Decimal,
    pub pnl: Decimal,
    pub pnl_pct: Decimal,
    pub risk_reward: Option<Decimal>,
    pub reason: String,
    pub entry_index: usize,
    pub exit_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SimOutcome {
    pub trades: Vec<ClosedTrade>,
    pub open_positions: Vec<Position>,
    pub available_fund: Decimal,
    pub skipped_funds: usize,
    pub unfilled_at_end: usize,
    pub invalid_levels: usize,
}

/// One account per symbol; engine sizing; no fees.
pub fn simulate(
    candles: &[Candle],
    signals: &[SimSignal],
    init_fund: i64,
    window: usize,
) -> SimOutcome {
    let init_fund = Decimal::from(init_fund);
    let budget = init_fund / Decimal::from(2);
    let mut available = init_fund;
    let mut positions: Vec<Position> = Vec::new();
    let mut trades: Vec<ClosedTrade> = Vec::new();
    let mut skipped_funds = 0usize;
    let mut invalid_levels = 0usize;

    for (index, candle) in candles.iter().enumerate() {
        if index < window {
            continue;
        }

        // 1. Fill the pending entry from the signal at index - 1 at this candle's open.
        if let Some(sim_signal) = signals.iter().find(|s| s.candle_index + 1 == index) {
            let signal = &sim_signal.signal;
            let geometry_ok = match signal.action {
                Direction::Long => {
                    signal.sl_price < signal.entry_price && signal.tp_price > signal.entry_price
                }
                Direction::Short => {
                    signal.sl_price > signal.entry_price && signal.tp_price < signal.entry_price
                }
            };
            if !geometry_ok {
                invalid_levels += 1;
            } else if available < budget {
                skipped_funds += 1;
            } else {
                available -= budget;
                positions.push(Position {
                    side: signal.action,
                    signal_entry: signal.entry_price,
                    fill: candle.open,
                    e: budget,
                    sl: signal.sl_price,
                    tp: signal.tp_price,
                    entry_index: index,
                });
            }
        }

        // 2. Exits, including a position just filled.
        let mut still_open = Vec::new();
        for position in positions.drain(..) {
            match exit_for(&position, candle) {
                Some((reason, exit)) => {
                    let proceeds = position.e * (exit / position.fill);
                    let mut pnl = match position.side {
                        Direction::Long => proceeds - position.e,
                        Direction::Short => position.e - proceeds,
                    };
                    if pnl < -position.e {
                        pnl = -position.e;
                    }
                    available += position.e + pnl;
                    trades.push(ClosedTrade {
                        side: position.side,
                        signal_entry: position.signal_entry,
                        fill: position.fill,
                        exit,
                        e: position.e,
                        pnl,
                        pnl_pct: if position.e.is_zero() {
                            Decimal::ZERO
                        } else {
                            pnl / position.e * Decimal::from(100)
                        },
                        risk_reward: risk_reward_for(position.side, position.sl, position.tp, position.fill),
                        reason,
                        entry_index: position.entry_index,
                        exit_index: index,
                    });
                }
                None => still_open.push(position),
            }
        }
        positions = still_open;
    }

    let unfilled_at_end = signals
        .iter()
        .filter(|s| s.candle_index + 1 >= candles.len())
        .count();

    SimOutcome {
        trades,
        open_positions: positions,
        available_fund: available,
        skipped_funds,
        unfilled_at_end,
        invalid_levels,
    }
}

fn exit_for(position: &Position, candle: &Candle) -> Option<(String, Decimal)> {
    let open = candle.open;
    match position.side {
        Direction::Long => {
            if open <= position.sl {
                Some(("gap_sl".into(), open))
            } else if open >= position.tp {
                Some(("gap_tp".into(), open))
            } else if candle.low <= position.sl {
                Some(("SL".into(), position.sl))
            } else if candle.high >= position.tp {
                Some(("TP".into(), position.tp))
            } else {
                None
            }
        }
        Direction::Short => {
            if open >= position.sl {
                Some(("gap_sl".into(), open))
            } else if open <= position.tp {
                Some(("gap_tp".into(), open))
            } else if candle.high >= position.sl {
                Some(("SL".into(), position.sl))
            } else if candle.low <= position.tp {
                Some(("TP".into(), position.tp))
            } else {
                None
            }
        }
    }
}

fn risk_reward_for(side: Direction, sl: Decimal, tp: Decimal, fill: Decimal) -> Option<Decimal> {
    match side {
        Direction::Long if fill > sl && tp > fill => Some((tp - fill) / (fill - sl)),
        Direction::Short if sl > fill && fill > tp => Some((fill - tp) / (sl - fill)),
        _ => None,
    }
}
```

- [ ] **Step 4: Implement statistics**

`src/harness/stats.rs`:

```rust
use crate::harness::sim::{ClosedTrade, Position, SimOutcome};
use crate::strategy::types::Direction;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Default)]
pub struct SimSummary {
    pub init_fund: Decimal,
    pub available_fund: Decimal,
    pub reserved: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub equity: Decimal,
    pub gross_profit: Decimal,
    pub gross_loss: Decimal,
    pub profit_factor: Option<Decimal>,
    pub trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakeven: usize,
    pub win_rate: Decimal,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub max_drawdown: Decimal,
    pub max_drawdown_pct: Decimal,
    pub exposure_candles: usize,
    pub time_in_market_pct: Decimal,
    pub open_count: usize,
    pub skipped_funds: usize,
    pub unfilled_at_end: usize,
    pub invalid_levels: usize,
}

pub fn summarize(
    init_fund: Decimal,
    outcome: &SimOutcome,
    last_close: Decimal,
    replayed_candles: usize,
) -> SimSummary {
    let trades = &outcome.trades;
    let realized: Decimal = trades.iter().map(|t| t.pnl).sum();
    let wins: Vec<&ClosedTrade> = trades.iter().filter(|t| t.pnl > Decimal::ZERO).collect();
    let losses: Vec<&ClosedTrade> = trades.iter().filter(|t| t.pnl < Decimal::ZERO).collect();
    let breakeven = trades.len() - wins.len() - losses.len();
    let gross_profit: Decimal = wins.iter().map(|t| t.pnl).sum();
    let gross_loss: Decimal = losses.iter().map(|t| t.pnl).sum();
    let profit_factor = if losses.is_empty() {
        None
    } else {
        Some(gross_profit / gross_loss.abs())
    };
    let win_rate = if trades.is_empty() {
        Decimal::ZERO
    } else {
        Decimal::from(wins.len()) / Decimal::from(trades.len()) * Decimal::from(100)
    };
    let avg_win = if wins.is_empty() { Decimal::ZERO } else { gross_profit / Decimal::from(wins.len()) };
    let avg_loss = if losses.is_empty() { Decimal::ZERO } else { gross_loss / Decimal::from(losses.len()) };

    let mut equity = init_fund;
    let mut peak = init_fund;
    let mut max_dd = Decimal::ZERO;
    for trade in trades {
        equity += trade.pnl;
        if equity > peak {
            peak = equity;
        }
        let dd = peak - equity;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    let max_dd_pct = if peak.is_zero() { Decimal::ZERO } else { max_dd / peak * Decimal::from(100) };

    let mut reserved = Decimal::ZERO;
    let mut unrealized = Decimal::ZERO;
    let mut exposure_candles = 0usize;
    for trade in trades {
        exposure_candles += trade.exit_index.saturating_sub(trade.entry_index);
    }
    for position in &outcome.open_positions {
        reserved += position.e;
        let ret = (last_close - position.fill) / position.fill;
        let ret = match position.side {
            Direction::Long => ret,
            Direction::Short => -ret,
        };
        unrealized += position.e * ret;
        exposure_candles += replayed_candles.saturating_sub(position.entry_index);
    }
    let time_in_market_pct = if replayed_candles == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(exposure_candles) / Decimal::from(replayed_candles) * Decimal::from(100)
    };

    SimSummary {
        init_fund,
        available_fund: outcome.available_fund,
        reserved,
        realized_pnl: realized,
        unrealized_pnl: unrealized,
        equity: outcome.available_fund + reserved + unrealized,
        gross_profit,
        gross_loss,
        profit_factor,
        trades: trades.len(),
        wins: wins.len(),
        losses: losses.len(),
        breakeven,
        win_rate,
        avg_win,
        avg_loss,
        max_drawdown: max_dd,
        max_drawdown_pct: max_dd_pct,
        exposure_candles,
        time_in_market_pct,
        open_count: outcome.open_positions.len(),
        skipped_funds: outcome.skipped_funds,
        unfilled_at_end: outcome.unfilled_at_end,
        invalid_levels: outcome.invalid_levels,
    }
}
```

- [ ] **Step 5: Run the tests**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test sim
cargo test -p analyzer-rs --features harness --test replay_payload
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): add engine-mirroring trade simulator and stats"
```

### Task 5: Report payload and HTML

**Files:**
- Create: `process/analyzer-rs/src/harness/report.rs`
- Create: `process/analyzer-rs/templates/replay.html`
- Modify: `process/analyzer-rs/src/harness/mod.rs`
- Test: `process/analyzer-rs/tests/replay_report.rs`

- [ ] **Step 1: Write the failing test**

`tests/replay_report.rs`:

```rust
#![cfg(feature = "harness")]

use analyzer_rs::harness::report::{build_payload, write_report, PayloadInput};

#[test]
fn payload_has_one_tab_per_symbol_and_is_self_contained() {
    // Uses the synthetic fixture for one symbol and a fabricated failure for another.
    // See the helper below in the implementation for constructing `PayloadInput`.
    let payload = build_payload(&[], &analyzer_rs::strategy::config::Config::default(), "2026-10-21T00:00:00Z".into());
    assert_eq!(payload["symbols"].as_array().unwrap().len(), 0);
    assert_eq!(payload["config"]["fees"], false);
}
```

The full test is completed after the implementation, because it needs a real replay and sim. The implementer should extend it to:
- run `load_csv` + `build_replay` + `simulate` + `summarize` for the synthetic fixture,
- call `build_payload`,
- assert `symbols[0].provenance.content_sha256` is 64 hex chars,
- assert `symbols[0].summary.returns_label` contains "gross simulated",
- write the report to a temp path and assert the file starts with `<!doctype html>`, contains `const REPLAY =`, contains the symbol name, and contains no `http://`/`https://` script/link references.

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test -p analyzer-rs --features harness --test replay_report
```

Expected: compile error (`report` missing).

- [ ] **Step 3: Implement the payload and writer**

`src/harness/report.rs`:

```rust
use crate::harness::dataset::Provenance;
use crate::harness::replay::Replay;
use crate::harness::sim::SimOutcome;
use crate::strategy::config::Config;
use serde_json::{json, Value};

pub struct PayloadInput<'a> {
    pub symbol: &'a str,
    pub replay: &'a Replay,
    pub sim: &'a SimOutcome,
    pub summary: &'a crate::harness::stats::SimSummary,
    pub provenance: &'a Provenance,
}

pub fn build_payload(
    inputs: &[PayloadInput<'_>],
    config: &Config,
    generated_at: String,
    failures: Vec<Value>,
) -> Value {
    let symbols: Vec<Value> = inputs
        .iter()
        .map(|input| {
            json!({
                "symbol": input.symbol,
                "candles": input.replay.candles.iter().map(|c| json!({
                    "t": c.open_time_ms,
                    "o": c.open.to_string(),
                    "h": c.high.to_string(),
                    "l": c.low.to_string(),
                    "c": c.close.to_string(),
                })).collect::<Vec<_>>(),
                "rsi": input.replay.rsi.iter().map(|v| v.map(|d| d.to_string())).collect::<Vec<_>>(),
                "sma": input.replay.sma.iter().map(|v| v.map(|d| d.to_string())).collect::<Vec<_>>(),
                "events": input.replay.events.iter().map(event_json).collect::<Vec<_>>(),
                "signals": input.replay.signals.iter().map(|s| json!({
                    "candle_index": s.candle_index,
                    "signal_id": s.signal_id,
                    "action": format!("{:?}", s.signal.action),
                    "reason": s.signal.reason,
                    "signal_entry": s.signal.entry_price.to_string(),
                    "sl": s.signal.sl_price.to_string(),
                    "tp": s.signal.tp_price.to_string(),
                    "divergence_index": s.signal.divergence_index,
                    "divergence_open_time_ms": s.signal.divergence_open_time_ms,
                    "age": s.signal.divergence_age_candles,
                })).collect::<Vec<_>>(),
                "trades": input.sim.trades.iter().map(|t| json!({
                    "entry_index": t.entry_index,
                    "exit_index": t.exit_index,
                    "side": format!("{:?}", t.side),
                    "signal_entry": t.signal_entry.to_string(),
                    "fill": t.fill.to_string(),
                    "exit": t.exit.to_string(),
                    "E": t.e.to_string(),
                    "pnl": t.pnl.to_string(),
                    "pnl_pct": t.pnl_pct.to_string(),
                    "risk_reward": t.risk_reward.map(|v| v.to_string()),
                    "reason": t.reason,
                })).collect::<Vec<_>>(),
                "open_positions": input.sim.open_positions.iter().map(|p| json!({
                    "entry_index": p.entry_index,
                    "side": format!("{:?}", p.side),
                    "signal_entry": p.signal_entry.to_string(),
                    "fill": p.fill.to_string(),
                    "E": p.e.to_string(),
                    "sl": p.sl.to_string(),
                    "tp": p.tp.to_string(),
                })).collect::<Vec<_>>(),
                "provenance": input.provenance,
                "summary": summary_json(input.summary, input.provenance.gaps),
            })
        })
        .collect();

    json!({
        "generated_at": generated_at,
        "config": {
            "timeframe": "", // filled by the caller from the CLI timeframe
            "limit": 0,
            "window": config.window,
            "init_fund": "1000",
            "fees": false,
            "rsi_period": config.rsi_period,
            "rsi_order": config.rsi_order,
            "price_diff_threshold": config.price_diff_threshold.to_string(),
            "sma_enabled": config.sma_enabled,
            "sma_period": config.sma_period,
            "atr_period": config.atr_period,
            "atr_sl_multiplier": config.atr_sl_multiplier.to_string(),
            "atr_tp_multiplier": config.atr_tp_multiplier.to_string(),
            "atr_pct_high": config.atr_pct_high.to_string(),
            "atr_high_adjust": config.atr_high_adjust.to_string(),
            "atr_sl_max_pct": config.atr_sl_max_pct.to_string(),
        },
        "symbols": symbols,
        "failures": failures,
    })
}

fn event_json(event: &crate::harness::replay::ReplayEvent) -> Value {
    match event {
        crate::harness::replay::ReplayEvent::Warmup { candle_index, seeded_keys } => {
            json!({"i": candle_index, "kind": "warmup", "seeded_keys": seeded_keys})
        }
        crate::harness::replay::ReplayEvent::Signal { candle_index, signal_index } => {
            json!({"i": candle_index, "kind": "signal", "signal_index": signal_index})
        }
        crate::harness::replay::ReplayEvent::Suppressed { candle_index, reason, .. } => {
            json!({"i": candle_index, "kind": "suppressed", "reason": reason})
        }
    }
}

fn summary_json(summary: &crate::harness::stats::SimSummary, gaps: usize) -> Value {
    let profit_factor = match summary.profit_factor {
        Some(ratio) => ratio.to_string(),
        None if summary.gross_profit > rust_decimal::Decimal::ZERO => "inf".to_string(),
        None => "null".to_string(),
    };
    json!({
        "init_fund": summary.init_fund.to_string(),
        "available_fund": summary.available_fund.to_string(),
        "reserved": summary.reserved.to_string(),
        "realized_pnl": summary.realized_pnl.to_string(),
        "unrealized_pnl": summary.unrealized_pnl.to_string(),
        "equity": summary.equity.to_string(),
        "gross_realized_pnl": summary.realized_pnl.to_string(),
        "profit_factor": profit_factor,
        "max_drawdown": summary.max_drawdown.to_string(),
        "max_drawdown_pct": summary.max_drawdown_pct.to_string(),
        "trades": summary.trades,
        "wins": summary.wins,
        "losses": summary.losses,
        "breakeven": summary.breakeven,
        "win_rate": summary.win_rate.to_string(),
        "avg_win": summary.avg_win.to_string(),
        "avg_loss": summary.avg_loss.to_string(),
        "exposure_candles": summary.exposure_candles,
        "time_in_market_pct": summary.time_in_market_pct.to_string(),
        "open_count": summary.open_count,
        "skipped_funds": summary.skipped_funds,
        "unfilled_at_end": summary.unfilled_at_end,
        "invalid_levels": summary.invalid_levels,
        "gaps": gaps,
        "returns_label": "gross simulated, excluding fees and slippage",
    })
}

pub fn write_report(payload: &Value, out: &std::path::Path) -> std::io::Result<()> {
    let template = include_str!("../../templates/replay.html");
    let json = serde_json::to_string(payload)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
    let escaped = json.replace("</", "<\\/");
    let html = template.replace("__REPLAY_DATA__", &escaped);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, html)
}
```

Add `pub gross_profit: Decimal` and `pub gross_loss: Decimal` to `SimSummary` and set them in `stats.rs`.

The payload `config.timeframe`, `limit`, and `init_fund` are placeholders in this snippet; the CLI fills them by mutating the returned `Value` before writing:

```rust
let mut payload = build_payload(&inputs, &config, generated_at);
payload["config"]["timeframe"] = json!(timeframe.as_kucoin());
payload["config"]["limit"] = json!(limit);
payload["config"]["init_fund"] = json!(init_fund.to_string());
```

- [ ] **Step 4: Create the HTML template**

Copy the approved Terminal demo and its assets into one file:

- Start from `design-demos/simulator/a-terminal.html`.
- Inline `design-demos/simulator/shared.css` into a `<style>` block.
- Replace the mock `shared.js` data block with a data adapter that reads `const REPLAY = __REPLAY_DATA__;` and builds the per-tab model from `REPLAY.symbols`.
- Keep the layout, controls, event stream, open positions, ledger, stats, and chart/RSI drawing; the drawing functions already exist in `shared.js`.
- Add the symbol tab bar and the failure list.
- Add the header banner `signals live-exact · trades simulated · fees excluded · not a profitability claim`.
- The file must not reference `shared.css`, `shared.js`, or any network URL.

The adapter function:

```js
function buildTabs(payload) {
  return payload.symbols.map((symbol) => ({
    symbol: symbol.symbol,
    candles: symbol.candles.map((c) => ({ i: 0, t: c.t, o: num(c.o), h: num(c.h), l: num(c.l), c: num(c.c) }))
      .map((c, i) => ({ ...c, i })),
    rsi: symbol.rsi.map((v) => (v === null ? null : num(v))),
    sma: symbol.sma.map((v) => (v === null ? null : num(v))),
    events: symbol.events,
    trades: symbol.trades,
    openPositions: symbol.open_positions,
    summary: symbol.summary,
    provenance: symbol.provenance,
  }));
}
function num(value) { return Number(value); }
```

The playback, chart, RSI, event stream, ledger, and stats rendering come from the demo's `shared.js` functions, adapted to the tab model. The mock `RAW_SIGNALS`, `suppressed`, and generated candles are removed.

- [ ] **Step 5: Run the tests**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness --test replay_report
cargo test -p analyzer-rs --features harness
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add process/analyzer-rs
git commit -m "feat(analyzer-rs): add replay payload and self-contained report"
```

---

### Task 6: CLI, Makefile, and acceptance

**Files:**
- Create: `process/analyzer-rs/src/bin/analyzer-replay.rs`
- Modify: `process/analyzer-rs/Cargo.toml`
- Modify: `Makefile`
- Modify: `process/analyzer-rs/README.md`
- Test: covered by the acceptance commands below

- [ ] **Step 1: Add the binary**

`Cargo.toml`:

```toml
[[bin]]
name = "analyzer-replay"
path = "src/bin/analyzer-replay.rs"
required-features = ["harness"]
```

- [ ] **Step 2: Implement the CLI**

`src/bin/analyzer-replay.rs`:

```rust
use analyzer_rs::config_env::{load_dotenv, strategy_from_env};
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{fetch_dataset, load_csv, Dataset};
use analyzer_rs::harness::replay::build_replay;
use analyzer_rs::harness::report::{build_payload, write_report, PayloadInput};
use analyzer_rs::harness::sim::{simulate, SimSignal};
use analyzer_rs::harness::stats::summarize;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Symbol to replay; repeat for multiple symbols.
    #[arg(long = "symbol")]
    symbols: Vec<String>,
    #[arg(long)]
    timeframe: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
    #[arg(long)]
    init_fund: Option<i64>,
    #[arg(long, default_value = "process/analyzer-rs/out/replay.html")]
    out: PathBuf,
    /// Offline CSV: SYMBOL=path (repeatable).
    #[arg(long = "data")]
    data: Vec<String>,
    #[arg(long)]
    as_of: Option<i64>,
    #[arg(long)]
    generated_at: Option<String>,
    #[arg(long, default_value_t = 0)]
    max_gap_candles: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    let cli = Cli::parse();
    let strategy = strategy_from_env()?;
    let timeframe_name = cli
        .timeframe
        .clone()
        .or_else(|| std::env::var("KUCOIN_TIMEFRAME").ok())
        .unwrap_or_else(|| "15min".into());
    let timeframe = Timeframe::from_kucoin(&timeframe_name)
        .ok_or_else(|| format!("unsupported timeframe {timeframe_name:?}"))?;
    let init_fund = cli
        .init_fund
        .or_else(|| std::env::var("BENCHMARK_INIT_FUND").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(1000);
    let as_of = cli.as_of.unwrap_or(i64::MAX);
    let base_url = std::env::var("KUCOIN_BASE_URL")
        .unwrap_or_else(|_| "https://api.kucoin.com".into());

    let symbols = if cli.symbols.is_empty() {
        std::env::var("BENCHMARK_TICKERS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        cli.symbols.clone()
    };

    let offline: std::collections::BTreeMap<String, PathBuf> = cli
        .data
        .iter()
        .filter_map(|entry| entry.split_once('='))
        .map(|(symbol, path)| (symbol.to_string(), PathBuf::from(path)))
        .collect();

    let mut inputs = Vec::new();
    let mut failures = Vec::new();
    for symbol in symbols {
        let dataset: Result<Dataset, String> = if let Some(path) = offline.get(&symbol) {
            load_csv(
                &symbol,
                path,
                timeframe,
                as_of,
                5_000,
                cli.max_gap_candles,
                strategy.window,
                cli.limit,
            )
            .map_err(|error| error.to_string())
        } else {
            fetch_dataset(
                &symbol,
                timeframe,
                cli.limit,
                strategy.window,
                as_of,
                5_000,
                cli.max_gap_candles,
                &base_url,
            )
            .map_err(|error| error.to_string())
        };
        let dataset = match dataset {
            Ok(dataset) => dataset,
            Err(error) => {
                failures.push(serde_json::json!({"symbol": symbol, "error": error}));
                continue;
            }
        };
        let replay = build_replay(&dataset, &strategy)?;
        let sim_signals: Vec<SimSignal> = replay
            .signals
            .iter()
            .map(|s| SimSignal { candle_index: s.candle_index, signal: s.signal.clone() })
            .collect();
        let outcome = simulate(&dataset.candles, &sim_signals, init_fund, strategy.window);
        let last_close = dataset.candles.last().map(|c| c.close).unwrap_or_default();
        let summary = summarize(
            rust_decimal::Decimal::from(init_fund),
            &outcome,
            last_close,
            dataset.candles.len().saturating_sub(strategy.window),
        );
        inputs.push((symbol, replay, outcome, summary, dataset.provenance));
    }

    let generated_at = cli
        .generated_at
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let payload_inputs: Vec<PayloadInput<'_>> = inputs
        .iter()
        .map(|(symbol, replay, sim, summary, provenance)| PayloadInput {
            symbol,
            replay,
            sim,
            summary,
            provenance,
        })
        .collect();
    let mut payload = build_payload(&payload_inputs, &strategy, generated_at, failures);
    payload["config"]["timeframe"] = serde_json::json!(timeframe.as_kucoin());
    payload["config"]["limit"] = serde_json::json!(cli.limit);
    payload["config"]["init_fund"] = serde_json::json!(init_fund.to_string());
    payload["config"]["as_of_ms"] = serde_json::json!(as_of);
    write_report(&payload, &cli.out)?;
    println!("wrote {}", cli.out.display());
    Ok(())
}
```

The `simulate` call needs the `summarize` integration; keep it simple by having `simulate` return trades/open positions and a base summary, then `summarize` recomputes the full summary from the trades. Adjust the `simulate` signature to return only `trades` and `open_positions`, or have `summarize` take the `SimOutcome`. Keep the CLI code above and adjust `simulate` accordingly during implementation.

- [ ] **Step 3: Makefile and README**

Makefile:

```make
analyzer-replay:
	cargo run -p analyzer-rs --features harness --bin analyzer-replay -- $(ARGS)
```

Add `analyzer-replay` to `.PHONY` and to `make help`.

README: document the CLI, the `.env` settings, the offline `--data` mode, and the label that results exclude fees and are not a profitability claim.

- [ ] **Step 4: Run the acceptance commands**

```bash
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
cargo run -p analyzer-rs --features harness --bin analyzer-replay -- \
  --timeframe 1day \
  --data BTC-USDT=tests/fixtures/btc_usdt_1day.csv \
  --data XRP-USDT=tests/fixtures/xrp_usdt_1day.csv \
  --out /tmp/replay.html
```

Expected: `/tmp/replay.html` exists, is self-contained, and contains two symbol tabs.

- [ ] **Step 5: Commit**

```bash
git add process/analyzer-rs Makefile
git commit -m "feat(analyzer-rs): add analyzer-replay CLI and report target"
```

---

## Self-review

- **Spec coverage:** config/identity extraction (Task 1), dataset validation/provenance (Task 2), replay events/signals and parity (Task 3), simulator with direction-aware execution and stats (Task 4), payload + HTML (Task 5), CLI/acceptance (Task 6). All spec sections map to a task.
- **Placeholder scan:** the payload `config.timeframe`/`limit`/`init_fund` values are filled by the CLI in Task 6; the Task 5 report tests are final test-first assertions (multi-symbol, escaping, determinism, failures). No `TBD`/`TODO`.
- **Dependency consistency:** `sha2` and `dotenvy` are normal dependencies because `identity` and `config_env` are unconditional modules; `clap` is harness-only.
- **Live regression:** Task 1 captures the live/grpc and signal-parity results before and after the extraction and diffs the summaries.
- **Type consistency:** `Config`, `Signal`, `Candle`, `ReplayEvent`, `ReplaySignal`, `SimSignal`, `SimSummary`, `Provenance`, `PayloadInput` are defined before use; `simulate(candles, signals, init_fund, window)` and `summarize(...)` signatures are consistent between Tasks 4–6.
