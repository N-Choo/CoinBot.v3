use crate::strategy::types::{Candle, Direction, Signal};
use rust_decimal::Decimal;

pub const REASON_TP: &str = "TP";
pub const REASON_GAP_TP: &str = "gap_tp";
pub const REASON_SL: &str = "SL";
pub const REASON_GAP_SL: &str = "gap_sl";
pub const REASON_REVERSAL: &str = "reversal";

#[derive(Debug, Clone)]
pub struct SimSignal {
    pub candle_index: usize,
    pub signal: Signal,
    pub signal_id: String,
}

#[derive(Debug, Clone)]
pub struct OpenStep {
    pub signal_id: String,
    pub unrealized: Decimal,
}

#[derive(Debug, Clone)]
pub struct AccountStep {
    pub index: usize,
    pub available_fund: Decimal,
    pub reserved: Decimal,
    pub unrealized: Decimal,
    pub equity: Decimal,
    pub open: Vec<OpenStep>,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub side: Direction,
    pub signal_id: String,
    pub signal_entry: Decimal,
    pub fill: Decimal,
    pub e: Decimal,
    pub sl: Option<Decimal>,
    pub tp: Option<Decimal>,
    pub entry_index: usize,
}

#[derive(Debug, Clone)]
pub struct ClosedTrade {
    pub side: Direction,
    pub signal_id: String,
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
    pub account: Vec<AccountStep>,
    pub available_fund: Decimal,
    pub skipped_funds: usize,
    pub unfilled_at_end: usize,
    pub invalid_levels: usize,
}

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
    let mut account: Vec<AccountStep> = Vec::new();
    let mut skipped_funds = 0usize;

    for (index, candle) in candles.iter().enumerate() {
        if index < window {
            continue;
        }

        if let Some(sim_signal) = signals.iter().find(|s| s.candle_index + 1 == index) {
            let signal = &sim_signal.signal;

            let mut kept = Vec::new();
            for position in positions.drain(..) {
                if position.side == signal.action {
                    kept.push(position);
                } else {
                    close_position(
                        &mut trades,
                        &mut available,
                        position,
                        candle.open,
                        REASON_REVERSAL,
                        index,
                    );
                }
            }
            positions = kept;

            let already_target = positions.iter().any(|p| p.side == signal.action);
            if !already_target {
                if available < budget {
                    skipped_funds += 1;
                } else {
                    available -= budget;
                    positions.push(Position {
                        side: signal.action,
                        signal_id: sim_signal.signal_id.clone(),
                        signal_entry: signal.entry_price,
                        fill: candle.open,
                        e: budget,
                        sl: signal.sl_price,
                        tp: signal.tp_price,
                        entry_index: index,
                    });
                }
            }
        }

        let mut still_open = Vec::new();
        for position in positions.drain(..) {
            match exit_for(&position, candle) {
                Some((reason, exit)) => {
                    close_position(&mut trades, &mut available, position, exit, &reason, index);
                }
                None => still_open.push(position),
            }
        }
        positions = still_open;

        let close = candle.close;
        let mut reserved = Decimal::ZERO;
        let mut unrealized = Decimal::ZERO;
        let mut open = Vec::with_capacity(positions.len());
        for position in &positions {
            reserved += position.e;
            let value = unrealized_pnl(position.side, position.fill, close, position.e);
            unrealized += value;
            open.push(OpenStep {
                signal_id: position.signal_id.clone(),
                unrealized: value,
            });
        }
        account.push(AccountStep {
            index,
            available_fund: available,
            reserved,
            unrealized,
            equity: available + reserved + unrealized,
            open,
        });
    }

    let unfilled_at_end = signals
        .iter()
        .filter(|s| s.candle_index + 1 >= candles.len())
        .count();

    SimOutcome {
        trades,
        open_positions: positions,
        account,
        available_fund: available,
        skipped_funds,
        unfilled_at_end,
        invalid_levels: 0,
    }
}

fn close_position(
    trades: &mut Vec<ClosedTrade>,
    available: &mut Decimal,
    position: Position,
    exit: Decimal,
    reason: &str,
    index: usize,
) {
    let proceeds = position.e * (exit / position.fill);
    let mut pnl = match position.side {
        Direction::Long => proceeds - position.e,
        Direction::Short => position.e - proceeds,
    };
    if pnl < -position.e {
        pnl = -position.e;
    }
    *available += position.e + pnl;
    trades.push(ClosedTrade {
        side: position.side,
        signal_id: position.signal_id.clone(),
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
        reason: reason.to_string(),
        entry_index: position.entry_index,
        exit_index: index,
    });
}

fn exit_for(position: &Position, candle: &Candle) -> Option<(String, Decimal)> {
    let (Some(sl), Some(tp)) = (position.sl, position.tp) else {
        return None;
    };
    let open = candle.open;
    match position.side {
        Direction::Long => {
            if open <= sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open >= tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.low <= sl {
                Some((REASON_SL.into(), sl))
            } else if candle.high >= tp {
                Some((REASON_TP.into(), tp))
            } else {
                None
            }
        }
        Direction::Short => {
            if open >= sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open <= tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.high >= sl {
                Some((REASON_SL.into(), sl))
            } else if candle.low <= tp {
                Some((REASON_TP.into(), tp))
            } else {
                None
            }
        }
    }
}

fn risk_reward_for(
    side: Direction,
    sl: Option<Decimal>,
    tp: Option<Decimal>,
    fill: Decimal,
) -> Option<Decimal> {
    let (Some(sl), Some(tp)) = (sl, tp) else {
        return None;
    };
    match side {
        Direction::Long if fill > sl && tp > fill => Some((tp - fill) / (fill - sl)),
        Direction::Short if sl > fill && fill > tp => Some((fill - tp) / (sl - fill)),
        _ => None,
    }
}

pub fn unrealized_pnl(side: Direction, fill: Decimal, close: Decimal, e: Decimal) -> Decimal {
    let ret = (close - fill) / fill;
    let signed = match side {
        Direction::Long => ret,
        Direction::Short => -ret,
    };
    e * signed
}
