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
        ConsumeReason::VetoedObstacleNone => "vetoed_obstacle_none",
        ConsumeReason::VetoedRiskNonPositive => "vetoed_risk_non_positive",
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
                match o {
                    Output::EntrySignal(s) => signals.push((s.clone(), idx)),
                    // `missed_touch` only ever arises from tick-step outputs.
                    Output::ZoneEvent(ev) => {
                        if matches!(
                            ev.reason,
                            Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch))
                        ) {
                            missed_touch += 1;
                        }
                    }
                    Output::StateChange { .. } => {}
                }
            }
            absorb_reasons(&mut reason_counts, &outs);
        }
        let (ns, outs) = step(&state, Input::ClosedCandle(candle), cfg)?;
        state = ns;
        absorb_reasons(&mut reason_counts, &outs);
    }

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
