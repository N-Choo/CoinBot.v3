use crate::harness::report::TradeRecord;
use crate::harness::sim::{simulate, SimOutcome, SimParams};
use crate::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use crate::strategy::config::Config;
use crate::strategy::step::step;
use crate::strategy::types::{
    Candle, ConsumeReason, Input, Output, Signal, StrategyState, Tick, ZoneEvent, ZoneEventReason,
};
use rust_decimal::Decimal;

pub struct RunResult {
    pub trades: Vec<TradeRecord>,
    pub signals: usize,
    pub skipped_open_position: usize,
    pub missed_touch: u64,
}

/// Run one (policy) over a dataset. Signals are collected in pass 1; pass 2 simulates them
/// in time order, opening at most one position at a time (later signals while a position is
/// open are counted as `skipped_open_position`, mirroring the engine's caps).
pub fn run(
    symbol: &str,
    candles: &[Candle],
    policy: PathPolicy,
    cfg: &Config,
    params: SimParams,
) -> RunResult {
    let mut state = StrategyState::new(symbol, crate::strategy::STATE_SCHEMA_VERSION);
    let mut src = Ohlcv4hSynthesis::new(candles.to_vec(), policy);
    let mut flat_ticks: Vec<Tick> = Vec::new();
    let mut signals: Vec<(Signal, usize)> = Vec::new();
    let mut missed_touch = 0u64;

    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            let idx = flat_ticks.len();
            let (ns, outs) = step(&state, Input::PriceTick(t), cfg).expect("core tick step");
            state = ns;
            flat_ticks.push(t);
            for o in outs {
                if let Output::EntrySignal(s) = o {
                    signals.push((s, idx));
                }
            }
        }
        let (ns, outs) = step(&state, Input::ClosedCandle(candle), cfg).expect("core candle step");
        state = ns;
        missed_touch += outs
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    Output::ZoneEvent(ZoneEvent {
                        reason: Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch)),
                        ..
                    })
                )
            })
            .count() as u64;
    }

    let n_signals = signals.len();
    let mut trades = Vec::new();
    let mut skipped_open_position = 0usize;
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
            SimOutcome::Unresolved => {
                // Position never resolved within the data: counts as a trade of 0R, keeps the slot open to the end.
                open_until_ms = Some(i64::MAX);
                trades.push(TradeRecord {
                    direction: sig.direction,
                    r: Decimal::ZERO,
                    pair: symbol.to_string(),
                    entry_time_ms: res.entry_ms,
                    exit_time_ms: res.exit_ms,
                });
            }
            SimOutcome::Unfillable => {
                // The entry was already at/beyond the stop or target: no fill, no position, so the
                // slot stays free for the next signal.
            }
        }
    }

    RunResult {
        trades,
        signals: n_signals,
        skipped_open_position,
        missed_touch,
    }
}
