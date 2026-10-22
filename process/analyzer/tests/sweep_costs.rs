#![cfg(feature = "harness")]

use analyzer::harness::sweep::costs::{net_open_pnl, net_trade_pnl, CostScenario};
use analyzer::strategy::types::Direction;
use rust_decimal::Decimal;

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

#[test]
fn base_costs_are_applied_to_both_notionals_long_winner() {
    let scenario = CostScenario::base();
    // E=500, fill=100, exit=110 -> q=5, exit_notional=550, gross=+50
    let net = net_trade_pnl(&scenario, Direction::Long, d(500), d(100), d(110), d(50));
    // entry_cost=500*15bps=0.75, exit_cost=550*15bps=0.825 -> 50-1.575
    assert_eq!(net, d(50) - Decimal::new(75, 2) - Decimal::new(825, 3));
}

#[test]
fn short_loser_still_pays_both_sides() {
    let scenario = CostScenario::base();
    // E=500, fill=100, exit=105, gross=-25
    let net = net_trade_pnl(&scenario, Direction::Short, d(500), d(100), d(105), d(-25));
    assert_eq!(net, d(-25) - Decimal::new(75, 2) - Decimal::new(7875, 4));
}

#[test]
fn stress_scenario_doubles_the_assumption() {
    assert_eq!(CostScenario::stress().per_side_rate(), Decimal::new(30, 4));
    assert_eq!(CostScenario::base().per_side_rate(), Decimal::new(15, 4));
}

#[test]
fn open_position_cost_uses_entry_and_estimated_exit() {
    let scenario = CostScenario::base();
    // E=500, fill=100, mark=90, gross=-50
    let net = net_open_pnl(&scenario, d(500), d(100), d(90), d(-50));
    assert_eq!(net, d(-50) - Decimal::new(75, 2) - Decimal::new(675, 3));
}
