use crate::strategy::types::Direction;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CostScenario {
    pub label: String,
    pub fee_bps: Decimal,
    pub slip_bps: Decimal,
}

impl CostScenario {
    pub fn base() -> Self {
        Self {
            label: "base".into(),
            fee_bps: Decimal::from(10),
            slip_bps: Decimal::from(5),
        }
    }

    pub fn stress() -> Self {
        Self {
            label: "stress".into(),
            fee_bps: Decimal::from(20),
            slip_bps: Decimal::from(10),
        }
    }

    /// Per-side rate as a fraction (bps / 10_000).
    pub fn per_side_rate(&self) -> Decimal {
        (self.fee_bps + self.slip_bps) / Decimal::from(10_000)
    }
}

/// Net PnL for a closed trade: gross minus entry and exit costs on their notionals.
pub fn net_trade_pnl(
    scenario: &CostScenario,
    _side: Direction,
    e: Decimal,
    fill: Decimal,
    exit: Decimal,
    gross_pnl: Decimal,
) -> Decimal {
    let rate = scenario.per_side_rate();
    let entry_cost = rate * e;
    let exit_notional = e * exit / fill;
    let exit_cost = rate * exit_notional;
    gross_pnl - entry_cost - exit_cost
}

/// Net unrealized for an open position marked at `mark`.
pub fn net_open_pnl(
    scenario: &CostScenario,
    e: Decimal,
    fill: Decimal,
    mark: Decimal,
    gross_unrealized: Decimal,
) -> Decimal {
    let rate = scenario.per_side_rate();
    let entry_cost = rate * e;
    let exit_notional = e * mark / fill;
    gross_unrealized - entry_cost - rate * exit_notional
}
