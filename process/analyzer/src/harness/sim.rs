use crate::strategy::types::{Candle, Direction, Signal};
use rust_decimal::Decimal;

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
    let mut last_accepted_side: Option<Direction> = None;

    for (index, candle) in candles.iter().enumerate() {
        if index < window {
            continue;
        }

        if let Some(sim_signal) = signals.iter().find(|s| s.candle_index + 1 == index) {
            let signal = &sim_signal.signal;

            // Defensive strict-alternation guard. The generator now enforces
            // alternation in the live/replay path before emission, but the
            // simulator still rejects same-side repeats so direct SimSignal
            // inputs (notably in tests) cannot affect positions.
            // A side may only be opened when the last accepted signal was the
            // opposite side.
            if last_accepted_side == Some(signal.action) {
                // Ignore this signal entirely.
            } else {
                last_accepted_side = Some(signal.action);

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
                            entry_index: index,
                        });
                    }
                }
            }
        }

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
        reason: reason.to_string(),
        entry_index: position.entry_index,
        exit_index: index,
    });
}

pub fn unrealized_pnl(side: Direction, fill: Decimal, close: Decimal, e: Decimal) -> Decimal {
    let ret = (close - fill) / fill;
    let signed = match side {
        Direction::Long => ret,
        Direction::Short => -ret,
    };
    e * signed
}
