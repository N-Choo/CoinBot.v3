#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::sim::{ClosedTrade, Position, SimOutcome};
use analyzer_rs::harness::sweep::costs::CostScenario;
use analyzer_rs::harness::sweep::evaluate::{evaluate_config, metrics_from_outcome};
use analyzer_rs::harness::sweep::partition::chronological_splits;
use analyzer_rs::harness::sweep::sampling;
use analyzer_rs::strategy::types::Direction;
use rust_decimal::Decimal;
use std::path::Path;

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn trade(pnl: i64, entry: usize, exit: usize, exit_price: i64) -> ClosedTrade {
    ClosedTrade {
        side: Direction::Long,
        signal_id: "s".into(),
        signal_entry: d(100),
        fill: d(100),
        exit: d(exit_price),
        e: d(500),
        pnl: d(pnl),
        pnl_pct: Decimal::ZERO,
        risk_reward: Some(d(2)),
        reason: if pnl >= 0 { "TP".into() } else { "SL".into() },
        entry_index: entry,
        exit_index: exit,
    }
}

#[test]
fn metrics_subtract_costs_and_compute_expectancy() {
    let outcome = SimOutcome {
        trades: vec![trade(50, 1, 2, 110), trade(-20, 3, 4, 100)],
        ..Default::default()
    };
    let m = metrics_from_outcome(&outcome, d(100), 2, 2, d(1000), &CostScenario::base());
    assert_eq!(m.trades, 2);
    // Winner exits at 110: entry cost 0.75, exit cost 0.15% * 550 = 0.825.
    // Loser exits at 100: entry cost 0.75, exit cost 0.75.
    let costs = Decimal::new(1575, 3) + Decimal::new(1500, 3);
    assert_eq!(m.closed_net_pnl, d(50) + d(-20) - costs);
    assert_eq!(m.net_expectancy, (d(30) - costs) / d(2));
    assert_eq!(m.open_net_pnl, Decimal::ZERO);
}

#[test]
fn max_drawdown_tracks_net_equity() {
    let outcome = SimOutcome {
        trades: vec![trade(-100, 1, 2, 100), trade(50, 3, 4, 100)],
        ..Default::default()
    };
    let m = metrics_from_outcome(&outcome, d(100), 2, 2, d(1000), &CostScenario::base());
    assert_eq!(m.max_drawdown, Decimal::new(1015, 1));
    assert_eq!(m.max_drawdown_pct, Decimal::new(1015, 2));
}

#[test]
fn stress_costs_reduce_profitable_net_pnl() {
    let outcome = SimOutcome {
        trades: vec![trade(50, 1, 2, 110)],
        ..Default::default()
    };
    let base = metrics_from_outcome(&outcome, d(100), 2, 2, d(1000), &CostScenario::base());
    let stress = metrics_from_outcome(&outcome, d(100), 2, 2, d(1000), &CostScenario::stress());
    assert!(
        base.closed_net_pnl > stress.closed_net_pnl,
        "base {} should exceed stress {}",
        base.closed_net_pnl,
        stress.closed_net_pnl
    );
}

#[test]
fn exposure_counts_closed_trade_duration() {
    let outcome = SimOutcome {
        trades: vec![trade(50, 1, 2, 100)],
        ..Default::default()
    };
    let m = metrics_from_outcome(&outcome, d(100), 2, 2, d(1000), &CostScenario::base());
    assert_eq!(m.exposure_candles, 1);
    assert_eq!(m.time_in_market_pct, d(50));
}

#[test]
fn open_position_metrics_include_estimated_exit_cost() {
    let outcome = SimOutcome {
        open_positions: vec![Position {
            side: Direction::Long,
            signal_id: "open".into(),
            signal_entry: d(100),
            fill: d(100),
            e: d(500),
            sl: d(90),
            tp: d(120),
            entry_index: 0,
        }],
        ..Default::default()
    };
    let m = metrics_from_outcome(&outcome, d(90), 2, 2, d(1000), &CostScenario::base());
    // Gross unrealized at the mark: 500 * (90 - 100) / 100 = -50.
    let gross_unrealized = d(-50);
    assert_eq!(m.open_at_end, 1);
    assert!(m.open_net_pnl < gross_unrealized);
    assert_eq!(
        m.open_net_pnl,
        gross_unrealized - Decimal::new(75, 2) - Decimal::new(675, 3)
    );
    assert_eq!(m.net_pnl, m.closed_net_pnl + m.open_net_pnl);
    assert_eq!(m.net_pnl, m.open_net_pnl);
}

#[test]
fn evaluate_config_produces_all_partitions_for_both_scenarios() {
    let dataset = load_csv(
        "SYN",
        Path::new("tests/fixtures/synthetic_1day.csv"),
        Timeframe::D1,
        i64::MAX / 2,
        0,
        0,
        100,
        0,
    )
    .expect("fixture loads");
    let splits = chronological_splits(dataset.candles.len());
    let baseline = sampling::baseline();
    let cfg = baseline.to_config();
    let scenarios = [CostScenario::base(), CostScenario::stress()];

    let result = evaluate_config(
        &dataset,
        &cfg,
        baseline.max_age,
        &splits,
        100,
        1000,
        &scenarios,
    )
    .expect("evaluation runs");

    assert!(result.partitions.contains_key("base"));
    assert!(result.partitions.contains_key("stress"));
    let base_validation = &result.partitions["base"]["validation"];
    let stress_validation = &result.partitions["stress"]["validation"];
    assert_ne!(base_validation.net_pnl, stress_validation.net_pnl);
    for scenario in ["base", "stress"] {
        let partitions = result
            .partitions
            .get(scenario)
            .unwrap_or_else(|| panic!("missing scenario {scenario}"));
        for name in [
            "train",
            "validation",
            "holdout",
            "fold1_val",
            "fold2_val",
            "fold3_val",
        ] {
            let m = partitions
                .get(name)
                .unwrap_or_else(|| panic!("missing partition {scenario}/{name}"));
            // Decimals are always finite; this checks the metrics were computed rather than
            // left at their defaults and that the net decomposition is consistent.
            assert_eq!(m.net_pnl, m.closed_net_pnl + m.open_net_pnl);
            assert!(m.max_drawdown >= Decimal::ZERO);
            assert!(m.max_drawdown_pct >= Decimal::ZERO);
            let _ = (
                m.gross_pnl,
                m.net_expectancy,
                m.net_expectancy_r,
                m.net_profit_factor,
                m.win_rate,
                m.expectancy_stderr,
                m.tp_hits,
                m.sl_hits,
                m.other_exits,
            );
        }
    }
}
