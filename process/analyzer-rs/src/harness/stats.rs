use crate::harness::sim::{
    unrealized_pnl, ClosedTrade, SimOutcome, REASON_GAP_SL, REASON_GAP_TP, REASON_SL, REASON_TP,
};
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
    pub profit_factor_is_infinite: bool,
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
    pub tp_hits: usize,
    pub sl_hits: usize,
    pub other_exits: usize,
    pub avg_risk_reward: Option<Decimal>,
}

pub fn summarize(
    init_fund: Decimal,
    outcome: &SimOutcome,
    last_close: Decimal,
    last_index: usize,
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
    let profit_factor_is_infinite = losses.is_empty() && gross_profit > Decimal::ZERO;
    let win_rate = if trades.is_empty() {
        Decimal::ZERO
    } else {
        Decimal::from(wins.len()) / Decimal::from(trades.len()) * Decimal::from(100)
    };
    let avg_win = if wins.is_empty() {
        Decimal::ZERO
    } else {
        gross_profit / Decimal::from(wins.len())
    };
    let avg_loss = if losses.is_empty() {
        Decimal::ZERO
    } else {
        gross_loss / Decimal::from(losses.len())
    };

    let tp_hits = trades
        .iter()
        .filter(|t| t.reason == REASON_TP || t.reason == REASON_GAP_TP)
        .count();
    let sl_hits = trades
        .iter()
        .filter(|t| t.reason == REASON_SL || t.reason == REASON_GAP_SL)
        .count();
    let other_exits = trades.len() - tp_hits - sl_hits;

    let eligible: Vec<Decimal> = trades.iter().filter_map(|t| t.risk_reward).collect();
    let avg_risk_reward = if eligible.is_empty() {
        None
    } else {
        Some(eligible.iter().copied().sum::<Decimal>() / Decimal::from(eligible.len()))
    };

    // max_dd_pct is measured from the peak at the largest absolute drop; ties keep the first peak.
    let mut equity = init_fund;
    let mut peak = init_fund;
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_pct = Decimal::ZERO;
    for trade in trades {
        equity += trade.pnl;
        if equity > peak {
            peak = equity;
        }
        let dd = peak - equity;
        if dd > max_dd {
            max_dd = dd;
            max_dd_pct = if peak.is_zero() {
                Decimal::ZERO
            } else {
                dd / peak * Decimal::from(100)
            };
        }
    }

    let mut reserved = Decimal::ZERO;
    let mut unrealized = Decimal::ZERO;
    let mut exposure_candles = 0usize;
    for trade in trades {
        exposure_candles += trade.exit_index.saturating_sub(trade.entry_index);
    }
    for position in &outcome.open_positions {
        reserved += position.e;
        unrealized += unrealized_pnl(position.side, position.fill, last_close, position.e);
        exposure_candles += last_index.saturating_sub(position.entry_index);
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
        profit_factor_is_infinite,
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
        tp_hits,
        sl_hits,
        other_exits,
        avg_risk_reward,
    }
}
