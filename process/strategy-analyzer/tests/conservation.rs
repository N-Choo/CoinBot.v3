//! Counter conservation tests (Task: investigate `mitigated == signals`).
//!
//! Two partitions must hold for any run of the pure core, and they are what catch a spuriously
//! incremented counter (e.g. `mitigated` once per signal, or a single zone-creation rejection
//! double-counted across tick + candle steps):
//!
//! 1. **Zone-creation partition.** Each BOS fires at most one `create_zone` attempt, and each
//!    attempt ends in exactly one outcome: a `Created` zone, or one rejection tag from
//!    [`REJECTION_TAGS`]. So `attempt_candles == created + Σ rejection_counts`, and no candle may
//!    carry more than one creation outcome.
//! 2. **Zone-outcome partition.** Every created zone ends in exactly one of: a published signal, a
//!    first-touch veto, a `missed_touch`, an invalidation (`trend_flip`/`close_beyond_edge`), an
//!    expiry, or it is still `Active` at the end of the data. So
//!    `created == signals + vetoed + missed_touch + invalidated + expired + still_active`.
//!
//! The trace is derived directly from the `Output` stream and the final `StrategyState`, so it is
//! independent of the harness's `absorb_reasons` counter. The trace is also cross-checked against
//! `count_signals`' `reason_counts` (the counter under suspicion).
#![cfg(feature = "harness")]

use std::collections::BTreeMap;
use std::path::Path;
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::run::count_signals;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::step::step;
use strategy_analyzer::strategy::types::{
    ConsumeReason, Input, Output, StrategyState, TerminalReason, ZoneEventKind, ZoneEventReason,
    ZoneState,
};

/// Zone-creation rejection tags emitted by `strategy::zones::create_zone`. `atr_zero`/`no_candles`
/// are guarded call preconditions (ATR == 0 skips creation; the window is non-empty when a BOS
/// fires), so the harness observes only these five. `atr_zero` is deliberately excluded from this
/// set: the core also emits an in-band `atr_zero` `StateChange` with the same tag, so including it
/// would make the partition ambiguous; the tests assert no `atr_zero` occurs instead.
const REJECTION_TAGS: [&str; 5] = [
    "no_opposite_candle",
    "weak_impulse",
    "zone_too_small",
    "zone_too_large",
    "mitigated",
];

/// A `ConsumeReason` veto tag (the first-touch vetoes only; the core emits these as `ZoneEvent`s).
fn veto_tag(c: ConsumeReason) -> Option<&'static str> {
    match c {
        ConsumeReason::VetoedObstacleTooClose => Some("vetoed_obstacle_too_close"),
        ConsumeReason::VetoedNoObstacle => Some("vetoed_no_obstacle"),
        ConsumeReason::VetoedInvalidLevels => Some("vetoed_invalid_levels"),
        _ => None,
    }
}

/// Every counter a conservation argument needs, derived from the output stream + final state.
#[derive(Default)]
struct Trace {
    /// Zones created (`ZoneEventKind::Created` count).
    created: usize,
    /// Per created zone id -> number of `Created` events (must be 1 for every id).
    created_ids: BTreeMap<String, usize>,
    /// Zone-creation rejection tag -> count.
    rejections: BTreeMap<&'static str, usize>,
    /// Published signals (`EntrySignal` count; each consumes exactly one zone).
    signals: usize,
    /// First-touch veto tag -> count.
    vetoed: BTreeMap<&'static str, usize>,
    /// `missed_touch` consumptions (tick first-observation + candle-close reconciliation).
    missed_touch: usize,
    /// Invalidation tag (`trend_flip`/`close_beyond_edge`) -> count.
    invalidated: BTreeMap<&'static str, usize>,
    /// Expired zones.
    expired: usize,
    /// Zones still `Active` in the final state.
    still_active: usize,
    /// In-band `atr_zero` `StateChange`s. Must be 0 for the creation partition to be unambiguous.
    atr_zero: usize,
    /// Candles on which at least one creation outcome occurred (the independent attempt count).
    attempt_candles: usize,
    /// Total creation outcomes, for the per-candle <= 1 structural check.
    creation_outcomes: usize,
    /// Terminal (`Consumed`/`Invalidated`/`Expired`) event count per zone id (must be <= 1).
    terminal_outcomes: BTreeMap<String, usize>,
}

impl Trace {
    /// Absorb one step's outputs; return the number of zone-creation outcomes in this step.
    fn absorb(&mut self, outs: &[Output]) -> usize {
        let mut creation = 0usize;
        for o in outs {
            match o {
                Output::EntrySignal(_) => self.signals += 1,
                Output::StateChange { reason } => {
                    match REJECTION_TAGS.iter().find(|t| *t == reason) {
                        Some(t) => {
                            *self.rejections.entry(*t).or_insert(0) += 1;
                            self.creation_outcomes += 1;
                            creation += 1;
                        }
                        // `atr_zero` is the only other `StateChange` the core emits; it is counted
                        // separately because a zone-creation attempt at ATR 0 would carry the same tag
                        // and make the creation partition ambiguous. `assert_conserved` requires zero.
                        None => {
                            assert_eq!(
                                *reason, "atr_zero",
                                "unexpected StateChange reason `{reason}`"
                            );
                            self.atr_zero += 1;
                        }
                    }
                }
                Output::ZoneEvent(ev) => match &ev.kind {
                    ZoneEventKind::Created => {
                        self.created += 1;
                        self.creation_outcomes += 1;
                        creation += 1;
                        *self.created_ids.entry(ev.zone_id.clone()).or_insert(0) += 1;
                    }
                    ZoneEventKind::Consumed => {
                        *self
                            .terminal_outcomes
                            .entry(ev.zone_id.clone())
                            .or_insert(0) += 1;
                        match ev.reason {
                            Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch)) => {
                                self.missed_touch += 1;
                            }
                            Some(ZoneEventReason::Consumed(c)) => {
                                let tag = veto_tag(c)
                                    .unwrap_or_else(|| panic!("unexpected consumed reason {c:?}"));
                                *self.vetoed.entry(tag).or_insert(0) += 1;
                            }
                            other => panic!("consumed event without a reason: {other:?}"),
                        }
                    }
                    ZoneEventKind::Invalidated => {
                        *self
                            .terminal_outcomes
                            .entry(ev.zone_id.clone())
                            .or_insert(0) += 1;
                        let tag = match ev.reason {
                            Some(ZoneEventReason::Terminal(TerminalReason::TrendFlip)) => {
                                "trend_flip"
                            }
                            Some(ZoneEventReason::Terminal(TerminalReason::CloseBeyondEdge)) => {
                                "close_beyond_edge"
                            }
                            other => panic!("unexpected invalidation reason: {other:?}"),
                        };
                        *self.invalidated.entry(tag).or_insert(0) += 1;
                    }
                    ZoneEventKind::Expired => {
                        *self
                            .terminal_outcomes
                            .entry(ev.zone_id.clone())
                            .or_insert(0) += 1;
                        self.expired += 1;
                    }
                },
            }
        }
        creation
    }

    fn rejection_sum(&self) -> usize {
        self.rejections.values().sum()
    }
    fn vetoed_sum(&self) -> usize {
        self.vetoed.values().sum()
    }
    fn invalidated_sum(&self) -> usize {
        self.invalidated.values().sum()
    }
    fn attempts(&self) -> usize {
        self.attempt_candles
    }

    /// Assert both conservation partitions plus the structural invariants. Returns nothing; panics
    /// with the full trace on the first violation.
    fn assert_conserved(&self, label: &str) {
        // No in-band `atr_zero` => the rejection tags are unambiguous.
        assert_eq!(
            self.atr_zero, 0,
            "[{label}] an `atr_zero` StateChange occurred; the creation partition is ambiguous"
        );
        // Each created zone id is created exactly once, and no zone ends twice.
        for (id, n) in &self.created_ids {
            assert_eq!(*n, 1, "[{label}] zone `{id}` created {n} times");
            assert!(
                self.terminal_outcomes.get(id).copied().unwrap_or(0) <= 1,
                "[{label}] zone `{id}` has multiple terminal outcomes: {:?}",
                self.terminal_outcomes.get(id)
            );
        }
        for (id, n) in &self.terminal_outcomes {
            assert_eq!(*n, 1, "[{label}] zone `{id}` has {n} terminal outcomes");
        }
        // Partition 1: at least one creation outcome heard of every attempt, at most one per candle.
        assert_eq!(
            self.creation_outcomes, self.attempt_candles,
            "[{label}] a candle emitted more than one zone-creation outcome"
        );
        assert_eq!(
            self.attempt_candles,
            self.created + self.rejection_sum(),
            "[{label}] zone-creation partition broken: attempts={} created={} rejections={:?}",
            self.attempt_candles,
            self.created,
            self.rejections
        );
        // Partition 2: every created zone ends exactly once.
        assert_eq!(
            self.created,
            self.signals + self.vetoed_sum() + self.missed_touch + self.invalidated_sum() + self.expired + self.still_active,
            "[{label}] zone-outcome partition broken: created={} signals={} vetoed={:?} missed={} invalidated={:?} expired={} still_active={}",
            self.created,
            self.signals,
            self.vetoed,
            self.missed_touch,
            self.invalidated,
            self.expired,
            self.still_active
        );
    }
}

/// Replay a cached OHLCV file under one policy, deriving the full conservation trace.
fn trace(path: &str, policy: PathPolicy) -> Trace {
    let candles = load_csv(path).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let mut st = StrategyState::new("XRP-USDT", 1);
    let mut src = Ohlcv4hSynthesis::new(candles, policy);
    let mut tr = Trace::default();
    while let Some((ticks, candle)) = src.next_candle() {
        let mut creation_this_candle = 0usize;
        for t in ticks {
            let (ns, outs) = step(&st, Input::PriceTick(t), &cfg).unwrap();
            st = ns;
            creation_this_candle += tr.absorb(&outs);
        }
        let (ns, outs) = step(&st, Input::ClosedCandle(candle), &cfg).unwrap();
        st = ns;
        creation_this_candle += tr.absorb(&outs);
        if creation_this_candle > 0 {
            tr.attempt_candles += 1;
        }
    }
    tr.still_active = st
        .zones
        .iter()
        .filter(|z| z.state == ZoneState::Active)
        .count();
    tr
}

/// Cross-check the trace-derived counters against `count_signals`' `reason_counts` (the harness
/// counter under suspicion). A doubled `absorb_reasons` increment would show up here.
fn assert_matches_reason_counts(tr: &Trace, path: &str, policy: PathPolicy, label: &str) {
    let candles = load_csv(path).unwrap();
    let cfg = Config::default_for(&["XRP-USDT"]);
    let c = count_signals("XRP-USDT", &candles, policy, &cfg).unwrap();
    let get = |k: &str| c.reason_counts.get(k).copied().unwrap_or(0);
    for tag in REJECTION_TAGS {
        assert_eq!(
            get(tag),
            tr.rejections.get(tag).copied().unwrap_or(0),
            "[{label}] reason_counts[{tag}] disagrees with the trace"
        );
    }
    assert_eq!(get("expired"), tr.expired, "[{label}] expired mismatch");
    assert_eq!(
        get("missed_touch"),
        tr.missed_touch,
        "[{label}] missed_touch mismatch"
    );
    assert_eq!(
        get("trend_flip"),
        tr.invalidated.get("trend_flip").copied().unwrap_or(0),
        "[{label}] trend_flip mismatch"
    );
    assert_eq!(
        get("close_beyond_edge"),
        tr.invalidated
            .get("close_beyond_edge")
            .copied()
            .unwrap_or(0),
        "[{label}] close_beyond_edge mismatch"
    );
    for tag in [
        "vetoed_obstacle_too_close",
        "vetoed_no_obstacle",
        "vetoed_invalid_levels",
    ] {
        assert_eq!(
            get(tag),
            tr.vetoed.get(tag).copied().unwrap_or(0),
            "[{label}] veto `{tag}` mismatch"
        );
    }
    assert_eq!(c.signals, tr.signals, "[{label}] signals mismatch");
}

/// Print the full trace (the counter trace the investigation asks for).
fn report(label: &str, tr: &Trace) {
    let attempts = tr.attempts();
    let mitigated = tr.rejections.get("mitigated").copied().unwrap_or(0);
    let frac = if attempts == 0 {
        0.0
    } else {
        mitigated as f64 / attempts as f64
    };
    println!("=== conservation trace: {label} ===");
    println!(
        "  create_zone attempts={attempts} created={} rejections={{ {} }} (sum={})",
        tr.created,
        tr.rejections
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" "),
        tr.rejection_sum()
    );
    println!(
        "  zones: signals={} vetoed={{ {} }} (sum={}) missed_touch={} invalidated={{ {} }} (sum={}) expired={} still_active={}",
        tr.signals,
        tr.vetoed
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" "),
        tr.vetoed_sum(),
        tr.missed_touch,
        tr.invalidated
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" "),
        tr.invalidated_sum(),
        tr.expired,
        tr.still_active
    );
    println!(
        "  mitigation: rejected={mitigated} attempts={attempts} fraction={frac:.4} (signals={})",
        tr.signals
    );
    let mut shares: Vec<(&&str, &usize)> = tr.rejections.iter().collect();
    shares.sort_by(|a, b| b.1.cmp(a.1));
    let breakdown: Vec<String> = shares
        .iter()
        .map(|(k, v)| {
            let f = if attempts == 0 {
                0.0
            } else {
                **v as f64 / attempts as f64
            };
            format!("{k}={v} ({:.2}%)", f * 100.0)
        })
        .collect();
    println!(
        "  rejection breakdown (share of attempts): created={} ({:.2}%), {}",
        tr.created,
        if attempts == 0 {
            0.0
        } else {
            tr.created as f64 / attempts as f64 * 100.0
        },
        breakdown.join(", ")
    );
}

#[test]
fn conservation_holds_on_signal_fixture() {
    let tr = trace("tests/fixtures/signal_4h.csv", PathPolicy::LowFirst);
    report("signal_4h.csv low_first", &tr);
    tr.assert_conserved("signal_4h.csv low_first");
    assert_matches_reason_counts(
        &tr,
        "tests/fixtures/signal_4h.csv",
        PathPolicy::LowFirst,
        "signal_4h.csv",
    );
}

#[test]
fn conservation_holds_on_signal_fixtures_both_policies() {
    for policy in [PathPolicy::LowFirst, PathPolicy::HighFirst] {
        let tr = trace("tests/fixtures/signal_4h.csv", policy);
        tr.assert_conserved("signal_4h.csv");
        assert_matches_reason_counts(&tr, "tests/fixtures/signal_4h.csv", policy, "signal_4h.csv");
    }
    // The mitigated fixture: the only zone-creating BOS is rejected `mitigated`, so 0 created.
    let tr = trace(
        "tests/fixtures/signal_mitigated_4h.csv",
        PathPolicy::LowFirst,
    );
    report("signal_mitigated_4h.csv low_first", &tr);
    tr.assert_conserved("signal_mitigated_4h.csv");
    assert_matches_reason_counts(
        &tr,
        "tests/fixtures/signal_mitigated_4h.csv",
        PathPolicy::LowFirst,
        "signal_mitigated_4h.csv",
    );
}

/// The in-sample XRP 4h file (untracked test data). Skipped when absent so CI without the cache
/// stays green; when present it asserts the same partitions and prints the counter trace.
#[test]
fn conservation_holds_on_in_sample_xrp() {
    let path = "data/xrp_4h.csv";
    if !Path::new(path).exists() {
        eprintln!("skipping: {path} not present");
        return;
    }
    for policy in [PathPolicy::LowFirst, PathPolicy::HighFirst] {
        let tr = trace(path, policy);
        report(&format!("data/xrp_4h.csv {:?}", policy), &tr);
        tr.assert_conserved("data/xrp_4h.csv");
        assert_matches_reason_counts(&tr, path, policy, "data/xrp_4h.csv");
    }
}
