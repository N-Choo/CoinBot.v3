use crate::harness::report::TradeRecord;
use crate::harness::sim::{simulate, SimOutcome, SimParams};
use crate::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::step::step;
use crate::strategy::types::{
    Candle, ConsumeReason, Input, Output, Signal, StrategyState, TerminalReason, Tick, ZoneEvent,
    ZoneEventReason,
};
use std::collections::BTreeMap;

pub struct RunResult {
    pub trades: Vec<TradeRecord>,
    pub signals: usize,
    pub skipped_open_position: usize,
    pub missed_touch: u64,
    /// Signals whose position never resolved within the data. Excluded from `trades` (and so from
    /// the verdict trade set) and reported separately.
    pub unresolved: usize,
    /// Signals whose entry was already at/beyond the stop or target: never filled, never traded.
    pub unfillable: usize,
    /// Every `Output` reason seen across **both** tick and candle steps, keyed by its snake_case tag:
    /// `StateChange { reason }` tags plus `ZoneEvent` terminal/consumed reasons.
    pub reason_counts: BTreeMap<String, usize>,
}

fn terminal_reason_tag(r: TerminalReason) -> &'static str {
    match r {
        TerminalReason::TrendFlip => "trend_flip",
        TerminalReason::CloseBeyondEdge => "close_beyond_edge",
        TerminalReason::Expired => "expired",
    }
}

fn consume_reason_tag(r: ConsumeReason) -> &'static str {
    match r {
        ConsumeReason::SignalPublished => "signal_published",
        ConsumeReason::MissedTouch => "missed_touch",
        ConsumeReason::EngineAccepted => "engine_accepted",
        ConsumeReason::EngineRejectedStrategy => "engine_rejected_strategy",
        ConsumeReason::EngineRejectedTransient => "engine_rejected_transient",
        ConsumeReason::ExpiredUndelivered => "expired_undelivered",
        ConsumeReason::VetoedObstacleTooClose => "vetoed_obstacle_too_close",
        ConsumeReason::VetoedNoObstacle => "vetoed_no_obstacle",
        ConsumeReason::VetoedInvalidLevels => "vetoed_invalid_levels",
    }
}

fn zone_event_reason_tag(ev: &ZoneEvent) -> Option<&'static str> {
    match &ev.reason {
        Some(ZoneEventReason::Terminal(t)) => Some(terminal_reason_tag(*t)),
        Some(ZoneEventReason::Consumed(c)) => Some(consume_reason_tag(*c)),
        None => None,
    }
}

/// Record every reason an output carries (used for both tick and candle steps).
fn absorb_reasons(counts: &mut BTreeMap<String, usize>, outs: &[Output]) {
    for o in outs {
        match o {
            Output::StateChange { reason } => {
                *counts.entry((*reason).to_string()).or_insert(0) += 1;
            }
            Output::ZoneEvent(ev) => {
                if let Some(tag) = zone_event_reason_tag(ev) {
                    *counts.entry(tag.to_string()).or_insert(0) += 1;
                }
            }
            Output::EntrySignal(_) => {}
        }
    }
}

/// Count `Consumed(MissedTouch)` zone events in a step's outputs. Missed touches arise from both
/// tick steps (the first-observation rule, §7.2) and candle steps (the candle-close wick
/// reconciliation, §7.2), so both must be counted here.
fn count_missed(outs: &[Output]) -> u64 {
    outs.iter()
        .filter(|o| {
            matches!(
                o,
                Output::ZoneEvent(ev)
                    if ev.reason == Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch))
            )
        })
        .count() as u64
}

/// Output of the count-only pass (ITEM 6): how many signals were produced and why zones ended, with
/// no PnL, no trades, and no verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCount {
    pub signals: usize,
    /// Every `Output` reason seen, keyed by its snake_case tag (same map `run` returns).
    pub reason_counts: BTreeMap<String, usize>,
}

/// Pass 1: drive the core through the given policy, collecting signals (with their flat-tick index
/// so pass 2 can simulate from the touching tick) and every output reason. Shared by `run` and
/// `count_signals`.
struct Pass1 {
    flat_ticks: Vec<Tick>,
    signals: Vec<(Signal, usize)>,
    missed_touch: u64,
    reason_counts: BTreeMap<String, usize>,
}

fn drive_pass1(
    symbol: &str,
    candles: &[Candle],
    policy: PathPolicy,
    cfg: &Config,
) -> Result<Pass1, StrategyError> {
    let mut state = StrategyState::new(symbol, crate::strategy::STATE_SCHEMA_VERSION);
    let mut src = Ohlcv4hSynthesis::new(candles.to_vec(), policy);
    let mut flat_ticks: Vec<Tick> = Vec::new();
    let mut signals: Vec<(Signal, usize)> = Vec::new();
    let mut missed_touch = 0u64;
    let mut reason_counts: BTreeMap<String, usize> = BTreeMap::new();

    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let idx = flat_ticks.len();
            let (ns, outs) = step(&state, Input::PriceTick(t), cfg)?;
            state = ns;
            flat_ticks.push(t);
            for o in &outs {
                if let Output::EntrySignal(s) = o {
                    signals.push((s.clone(), idx));
                }
            }
            missed_touch += count_missed(&outs);
            absorb_reasons(&mut reason_counts, &outs);
        }
        let (ns, outs) = step(&state, Input::ClosedCandle(candle), cfg)?;
        state = ns;
        missed_touch += count_missed(&outs);
        absorb_reasons(&mut reason_counts, &outs);
    }

    Ok(Pass1 {
        flat_ticks,
        signals,
        missed_touch,
        reason_counts,
    })
}

/// Load and drive a dataset for **counts only** (ITEM 6): the signal count and every reason, with no
/// simulation, no PnL, and no verdict. The caller is responsible for refusing OOS rows.
pub fn count_signals(
    symbol: &str,
    candles: &[Candle],
    policy: PathPolicy,
    cfg: &Config,
) -> Result<SignalCount, StrategyError> {
    let pass1 = drive_pass1(symbol, candles, policy, cfg)?;
    Ok(SignalCount {
        signals: pass1.signals.len(),
        reason_counts: pass1.reason_counts,
    })
}

/// Run one (policy) over a dataset. Signals are collected in pass 1; pass 2 simulates them
/// in time order, opening at most one position at a time (later signals while a position is
/// open are counted as `skipped_open_position`, mirroring the engine's caps). Core errors are
/// propagated rather than panicked.
pub fn run(
    symbol: &str,
    candles: &[Candle],
    policy: PathPolicy,
    cfg: &Config,
    params: SimParams,
) -> Result<RunResult, StrategyError> {
    let pass1 = drive_pass1(symbol, candles, policy, cfg)?;
    let Pass1 {
        flat_ticks,
        signals,
        missed_touch,
        reason_counts,
    } = pass1;

    let n_signals = signals.len();
    let mut trades = Vec::new();
    let mut skipped_open_position = 0usize;
    let mut unresolved = 0usize;
    let mut unfillable = 0usize;
    let mut open_until_ms: Option<i64> = None;

    for (sig, idx) in signals {
        // `ticks[0]` of the simulation is the touching tick, whose flat index is `idx`.
        let entry_ms = flat_ticks
            .get(idx)
            .map(|t| t.time_ms)
            .unwrap_or(sig.triggered_at_ms);
        if open_until_ms.is_some_and(|x| entry_ms < x) {
            skipped_open_position += 1; // a position is still open; the engine would block this
            continue;
        }
        let res = simulate(&sig, &flat_ticks[idx..], params);
        match res.outcome {
            SimOutcome::TakeProfit(r) | SimOutcome::StopLoss(r) => {
                open_until_ms = Some(res.exit_ms);
                trades.push(TradeRecord {
                    direction: sig.direction,
                    r,
                    pair: symbol.to_string(),
                    entry_time_ms: res.entry_ms,
                    exit_time_ms: res.exit_ms,
                });
            }
            SimOutcome::Unfillable => {
                // No fill and no position: the slot stays free for the next signal.
                unfillable += 1;
            }
            SimOutcome::Unresolved => {
                // Still open at end of data: excluded from `trades` (and the verdict set), reported
                // separately, and the slot stays occupied to the end (the engine caps one position).
                unresolved += 1;
                open_until_ms = Some(i64::MAX);
            }
        }
    }

    Ok(RunResult {
        trades,
        signals: n_signals,
        skipped_open_position,
        missed_touch,
        unresolved,
        unfillable,
        reason_counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-reason report tags the CLI surfaces (`count-signals` / `backtest`) are the
    /// `reason_counts` keys: pin the renamed first-touch vetoes so a serde/report rename cannot
    /// silently drop them from the output.
    #[test]
    fn first_touch_veto_tags_are_stable() {
        assert_eq!(
            consume_reason_tag(ConsumeReason::VetoedObstacleTooClose),
            "vetoed_obstacle_too_close"
        );
        assert_eq!(
            consume_reason_tag(ConsumeReason::VetoedNoObstacle),
            "vetoed_no_obstacle"
        );
        assert_eq!(
            consume_reason_tag(ConsumeReason::VetoedInvalidLevels),
            "vetoed_invalid_levels"
        );
    }
}
