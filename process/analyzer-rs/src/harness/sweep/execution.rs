use crate::harness::dataset::Dataset;
use crate::harness::replay::{build_replay, ReplaySignal};
use crate::harness::sim::{
    ClosedTrade, SimOutcome, REASON_GAP_SL, REASON_GAP_TP, REASON_SL, REASON_TP,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::{metrics_from_outcome, pool, PartitionMetrics};
use crate::harness::sweep::partition::{slice_with_warmup, Splits};
use crate::strategy::config::Config;
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

const REASON_EOD: &str = "eod";

#[derive(Debug, Clone, Serialize)]
pub enum EntryMode {
    Market,
    Limit {
        valid_candles: usize,
        buffer_bps: Decimal,
        offset_bps: Decimal,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecModel {
    pub label: String,
    pub entry: EntryMode,
    pub maker_bps: Decimal,
    pub taker_bps: Decimal,
    pub slip_bps: Decimal,
}

impl ExecModel {
    /// The four pre-registered models for the focused execution experiment.
    pub fn focused() -> Vec<Self> {
        let m = |label: &str, entry: EntryMode| ExecModel {
            label: label.into(),
            entry,
            maker_bps: Decimal::from(2),
            taker_bps: Decimal::from(10),
            slip_bps: Decimal::from(5),
        };
        vec![
            m("market", EntryMode::Market),
            m(
                "limit_1c_b2",
                EntryMode::Limit {
                    valid_candles: 1,
                    buffer_bps: Decimal::from(2),
                    offset_bps: Decimal::ZERO,
                },
            ),
            m(
                "limit_2c_b2",
                EntryMode::Limit {
                    valid_candles: 2,
                    buffer_bps: Decimal::from(2),
                    offset_bps: Decimal::ZERO,
                },
            ),
            m(
                "retest_2c_b2",
                EntryMode::Limit {
                    valid_candles: 2,
                    buffer_bps: Decimal::from(2),
                    offset_bps: Decimal::from(25),
                },
            ),
        ]
    }

    /// Per-side entry cost as a fraction of the entry notional.
    pub fn entry_rate(&self) -> Decimal {
        match &self.entry {
            EntryMode::Market => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
            EntryMode::Limit { .. } => self.maker_bps / Decimal::from(10_000),
        }
    }

    /// Per-side exit cost as a fraction of the exit notional.
    pub fn exit_rate(&self, reason: &str) -> Decimal {
        match &self.entry {
            EntryMode::Market => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
            EntryMode::Limit { .. } if reason == REASON_TP => {
                self.maker_bps / Decimal::from(10_000)
            }
            EntryMode::Limit { .. } => (self.taker_bps + self.slip_bps) / Decimal::from(10_000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecTrade {
    pub signal_id: String,
    pub side: Direction,
    pub signal_entry: Decimal,
    pub fill: Decimal,
    pub exit: Decimal,
    pub e: Decimal,
    pub gross_pnl: Decimal,
    pub entry_cost: Decimal,
    pub exit_cost: Decimal,
    pub net_pnl: Decimal,
    pub reason: String,
    pub entry_index: usize,
    pub exit_index: usize,
    pub risk_reward: Option<Decimal>,
}

#[derive(Debug, Clone, Default)]
pub struct ExecOutcome {
    pub trades: Vec<ExecTrade>,
    pub available_fund: Decimal,
    pub signals_total: usize,
    pub filled: usize,
    pub expired: usize,
    pub skipped_funds: usize,
    pub invalid_levels: usize,
}

impl ExecOutcome {
    /// Converts the net-valued trades into the shape consumed by `metrics_from_outcome`.
    ///
    /// Every position is closed at partition end, so `open_positions` is empty and the
    /// metrics are computed with a zero-cost scenario to avoid charging costs twice.
    pub fn to_sim_outcome(&self) -> SimOutcome {
        SimOutcome {
            trades: self
                .trades
                .iter()
                .map(|t| ClosedTrade {
                    side: t.side,
                    signal_entry: t.signal_entry,
                    fill: t.fill,
                    exit: t.exit,
                    e: t.e,
                    pnl: t.net_pnl,
                    pnl_pct: if t.e.is_zero() {
                        Decimal::ZERO
                    } else {
                        t.net_pnl / t.e * Decimal::from(100)
                    },
                    risk_reward: t.risk_reward,
                    reason: t.reason.clone(),
                    signal_id: t.signal_id.clone(),
                    entry_index: t.entry_index,
                    exit_index: t.exit_index,
                })
                .collect(),
            open_positions: Vec::new(),
            available_fund: self.available_fund,
            skipped_funds: self.skipped_funds,
            unfilled_at_end: self.expired,
            invalid_levels: self.invalid_levels,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
struct Pending {
    signal_id: String,
    side: Direction,
    signal_entry: Decimal,
    sl: Decimal,
    tp: Decimal,
    signal_index: usize,
}

#[derive(Debug, Clone)]
struct OpenPosition {
    signal_id: String,
    side: Direction,
    signal_entry: Decimal,
    fill: Decimal,
    e: Decimal,
    sl: Decimal,
    tp: Decimal,
    entry_index: usize,
    entry_cost: Decimal,
    allow_same_candle_exit: bool,
}

/// Simulates the execution models over `candles`.
///
/// Signals are queued at their own candle and can only be filled from the next candle onward.
/// Limit orders remain pending until they fill, are skipped for insufficient funds, or exhaust
/// their validity window. `window` warm-up candles are skipped, matching the sweep harness.
pub fn simulate_execution(
    candles: &[Candle],
    signals: &[ReplaySignal],
    init_fund: i64,
    window: usize,
    model: &ExecModel,
) -> ExecOutcome {
    let init_fund = Decimal::from(init_fund);
    let budget = init_fund / Decimal::from(2);
    let mut available = init_fund;
    let mut pending: Vec<Pending> = Vec::new();
    let mut positions: Vec<OpenPosition> = Vec::new();
    let mut trades: Vec<ExecTrade> = Vec::new();
    let mut signals_total = 0usize;
    let mut filled = 0usize;
    let mut expired = 0usize;
    let mut skipped_funds = 0usize;
    let mut invalid_levels = 0usize;
    let mut cursor = 0usize;

    for (index, candle) in candles.iter().enumerate() {
        if index < window {
            continue;
        }

        // Signals outside the simulated range (warm-up) are not part of this partition.
        while cursor < signals.len() && signals[cursor].candle_index < index {
            cursor += 1;
        }

        // 1-2. Work pending entries and open any position that fills.
        let mut still_pending = Vec::with_capacity(pending.len());
        for order in pending.drain(..) {
            match &model.entry {
                EntryMode::Market => {
                    if index == order.signal_index + 1 {
                        if available >= budget {
                            available -= budget;
                            positions.push(open_position(
                                order,
                                candle.open,
                                index,
                                budget,
                                model,
                                true,
                            ));
                            filled += 1;
                        } else {
                            skipped_funds += 1;
                        }
                    } else if index > order.signal_index + 1 {
                        expired += 1;
                    } else {
                        still_pending.push(order);
                    }
                }
                EntryMode::Limit {
                    valid_candles,
                    buffer_bps,
                    offset_bps,
                } => {
                    let last_valid = order.signal_index + valid_candles;
                    if index > last_valid {
                        expired += 1;
                    } else if index <= order.signal_index {
                        still_pending.push(order);
                    } else {
                        let fill = limit_fill(
                            order.side,
                            order.signal_entry,
                            *offset_bps,
                            *buffer_bps,
                            candle,
                        );
                        match fill {
                            Some(price) => {
                                if available >= budget {
                                    available -= budget;
                                    positions.push(open_position(
                                        order, price, index, budget, model, false,
                                    ));
                                    filled += 1;
                                } else {
                                    // The touch consumed the opportunity; the order does not
                                    // wait for funds to free up later in its validity window.
                                    skipped_funds += 1;
                                }
                            }
                            None => still_pending.push(order),
                        }
                    }
                }
            }
        }
        pending = still_pending;

        // 3. Exits. Limit entries are not allowed to exit on their entry candle.
        let mut still_open = Vec::with_capacity(positions.len());
        for position in positions.drain(..) {
            if !position.allow_same_candle_exit && position.entry_index == index {
                still_open.push(position);
                continue;
            }
            match exit_for(&position, candle) {
                Some((reason, exit)) => close_position(
                    position,
                    exit,
                    index,
                    &reason,
                    model,
                    &mut available,
                    &mut trades,
                ),
                None => still_open.push(position),
            }
        }
        positions = still_open;

        // 4. Queue this candle's signals. A signal on the final candle can never fill.
        while cursor < signals.len() && signals[cursor].candle_index == index {
            let replay_signal = &signals[cursor];
            cursor += 1;
            signals_total += 1;
            let signal = &replay_signal.signal;
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
            } else if index + 1 >= candles.len() {
                expired += 1;
            } else {
                pending.push(Pending {
                    signal_id: replay_signal.signal_id.clone(),
                    side: signal.action,
                    signal_entry: signal.entry_price,
                    sl: signal.sl_price,
                    tp: signal.tp_price,
                    signal_index: index,
                });
            }
        }
    }

    // Orders still pending when the data ends never got a valid fill candle.
    expired += pending.len();

    // Close every remaining position at the partition's final close.
    if let Some(last) = candles.last() {
        let last_index = candles.len().saturating_sub(1);
        for position in positions.drain(..) {
            close_position(
                position,
                last.close,
                last_index,
                REASON_EOD,
                model,
                &mut available,
                &mut trades,
            );
        }
    }

    ExecOutcome {
        trades,
        available_fund: available,
        signals_total,
        filled,
        expired,
        skipped_funds,
        invalid_levels,
    }
}

fn open_position(
    order: Pending,
    fill: Decimal,
    entry_index: usize,
    e: Decimal,
    model: &ExecModel,
    allow_same_candle_exit: bool,
) -> OpenPosition {
    OpenPosition {
        signal_id: order.signal_id,
        side: order.side,
        signal_entry: order.signal_entry,
        fill,
        e,
        sl: order.sl,
        tp: order.tp,
        entry_index,
        entry_cost: model.entry_rate() * e,
        allow_same_candle_exit,
    }
}

fn limit_fill(
    side: Direction,
    entry: Decimal,
    offset_bps: Decimal,
    buffer_bps: Decimal,
    candle: &Candle,
) -> Option<Decimal> {
    let offset = offset_bps / Decimal::from(10_000);
    let buffer = buffer_bps / Decimal::from(10_000);
    match side {
        Direction::Long => {
            let limit = entry * (Decimal::ONE - offset);
            let threshold = limit * (Decimal::ONE - buffer);
            if candle.open <= threshold {
                Some(candle.open)
            } else if candle.low <= threshold {
                Some(limit)
            } else {
                None
            }
        }
        Direction::Short => {
            let limit = entry * (Decimal::ONE + offset);
            let threshold = limit * (Decimal::ONE + buffer);
            if candle.open >= threshold {
                Some(candle.open)
            } else if candle.high >= threshold {
                Some(limit)
            } else {
                None
            }
        }
    }
}

fn exit_for(position: &OpenPosition, candle: &Candle) -> Option<(String, Decimal)> {
    let open = candle.open;
    match position.side {
        Direction::Long => {
            if open <= position.sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open >= position.tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.low <= position.sl {
                Some((REASON_SL.into(), position.sl))
            } else if candle.high >= position.tp {
                Some((REASON_TP.into(), position.tp))
            } else {
                None
            }
        }
        Direction::Short => {
            if open >= position.sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open <= position.tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.high >= position.sl {
                Some((REASON_SL.into(), position.sl))
            } else if candle.low <= position.tp {
                Some((REASON_TP.into(), position.tp))
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

#[allow(clippy::too_many_arguments)]
fn close_position(
    position: OpenPosition,
    exit: Decimal,
    exit_index: usize,
    reason: &str,
    model: &ExecModel,
    available: &mut Decimal,
    trades: &mut Vec<ExecTrade>,
) {
    // Match `sim.rs` exactly: multiply `E` by the price ratio rather than evaluating
    // `E * exit / fill` left to right.
    let proceeds = position.e * (exit / position.fill);
    let mut gross_pnl = match position.side {
        Direction::Long => proceeds - position.e,
        Direction::Short => position.e - proceeds,
    };
    if gross_pnl < -position.e {
        gross_pnl = -position.e;
    }
    let exit_notional = proceeds;
    let exit_cost = model.exit_rate(reason) * exit_notional;
    let net_pnl = gross_pnl - position.entry_cost - exit_cost;
    *available += position.e + gross_pnl;

    trades.push(ExecTrade {
        signal_id: position.signal_id,
        side: position.side,
        signal_entry: position.signal_entry,
        fill: position.fill,
        exit,
        e: position.e,
        gross_pnl,
        entry_cost: position.entry_cost,
        exit_cost,
        net_pnl,
        reason: reason.to_string(),
        entry_index: position.entry_index,
        exit_index,
        risk_reward: risk_reward_for(position.side, position.sl, position.tp, position.fill),
    });
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecPartition {
    pub metrics: PartitionMetrics,
    pub signals: usize,
    pub filled: usize,
    pub expired: usize,
    pub skipped_funds: usize,
    pub invalid_levels: usize,
    pub fill_rate: Decimal,
}

/// Pools per-symbol execution partitions, summing signal counts and recomputing the fill rate.
pub fn pool_exec(parts: &[ExecPartition]) -> ExecPartition {
    let metrics: Vec<PartitionMetrics> = parts.iter().map(|part| part.metrics.clone()).collect();
    let mut pooled = ExecPartition {
        metrics: pool(&metrics),
        signals: 0,
        filled: 0,
        expired: 0,
        skipped_funds: 0,
        invalid_levels: 0,
        fill_rate: Decimal::ZERO,
    };
    for part in parts {
        pooled.signals += part.signals;
        pooled.filled += part.filled;
        pooled.expired += part.expired;
        pooled.skipped_funds += part.skipped_funds;
        pooled.invalid_levels += part.invalid_levels;
    }
    if pooled.signals != 0 {
        pooled.fill_rate = Decimal::from(pooled.filled) / Decimal::from(pooled.signals);
    }
    pooled
}

/// Evaluates every model on the validation and holdout partitions.
///
/// Returns `model label -> partition -> partition result`. Costs are deducted by the execution
/// simulator, so the metrics use a zero-cost scenario to avoid double charging.
pub fn evaluate_execution_config(
    dataset: &Dataset,
    cfg: &Config,
    max_age: Option<usize>,
    splits: &Splits,
    window: usize,
    init_fund: i64,
    models: &[ExecModel],
) -> Result<BTreeMap<String, BTreeMap<String, ExecPartition>>, String> {
    if window != cfg.window {
        return Err(format!("window {window} != cfg.window {}", cfg.window));
    }
    let init_fund_decimal = Decimal::from(init_fund);
    let zero_cost = CostScenario {
        label: "net".into(),
        fee_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    };
    let mut result: BTreeMap<String, BTreeMap<String, ExecPartition>> = BTreeMap::new();

    for (name, slice) in [
        ("validation", splits.validation),
        ("holdout", splits.holdout),
    ] {
        let partition = slice_with_warmup(&dataset.candles, slice, window);
        let eval_start = window.max(partition.partition_start);
        let n_candles = partition.candles.len();
        let last_index = n_candles.saturating_sub(1);
        let replayed_candles = n_candles.saturating_sub(window);
        let slice_dataset = Dataset {
            symbol: dataset.symbol.clone(),
            candles: partition.candles,
            provenance: dataset.provenance.clone(),
        };
        let last_close = slice_dataset
            .candles
            .last()
            .ok_or_else(|| format!("partition {name} has no candles"))?
            .close;
        let replay = build_replay(&slice_dataset, cfg, max_age)
            .map_err(|error| format!("partition {name}: {error}"))?;
        let signals: Vec<ReplaySignal> = replay
            .signals
            .into_iter()
            .filter(|signal| signal.candle_index >= eval_start)
            .collect();

        for model in models {
            let outcome =
                simulate_execution(&slice_dataset.candles, &signals, init_fund, window, model);
            let sim_outcome = outcome.to_sim_outcome();
            let metrics = metrics_from_outcome(
                &sim_outcome,
                last_close,
                last_index,
                replayed_candles,
                init_fund_decimal,
                &zero_cost,
            );
            let fill_rate = if outcome.signals_total == 0 {
                Decimal::ZERO
            } else {
                Decimal::from(outcome.filled) / Decimal::from(outcome.signals_total)
            };
            result.entry(model.label.clone()).or_default().insert(
                name.to_string(),
                ExecPartition {
                    metrics,
                    signals: outcome.signals_total,
                    filled: outcome.filled,
                    expired: outcome.expired,
                    skipped_funds: outcome.skipped_funds,
                    invalid_levels: outcome.invalid_levels,
                    fill_rate,
                },
            );
        }
    }

    Ok(result)
}
