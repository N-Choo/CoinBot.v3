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
                        risk_reward: risk_reward_for(
                            position.side,
                            position.sl,
                            position.tp,
                            position.fill,
                        ),
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
