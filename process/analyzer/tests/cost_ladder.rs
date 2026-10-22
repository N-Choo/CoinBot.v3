#![cfg(feature = "harness")]

use analyzer::harness::sweep::cost_ladder::{break_even_bps, frozen_candidates, scenarios};
use rust_decimal::Decimal;

fn d(value: i64, scale: u32) -> Decimal {
    Decimal::new(value, scale)
}

#[test]
fn frozen_candidates_are_baseline_first_and_include_the_four_winners() {
    let candidates = frozen_candidates();

    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[0].label, "baseline");
    let labels: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate.label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "baseline",
            "joint-13",
            "joint-24",
            "max_age=4",
            "atr_tp=4.0"
        ]
    );

    // The frozen winners come from the 4hour screening and hold SMA at period 20.
    for candidate in &candidates[1..] {
        assert_eq!(candidate.origin, "screening-4hour");
        assert!(candidate.sma_enabled);
        assert_eq!(candidate.sma_period, 20);
    }
    assert_eq!(
        (
            candidates[1].rsi_period,
            candidates[1].rsi_order,
            candidates[1].max_age,
            candidates[1].atr_sl_multiplier,
            candidates[1].atr_tp_multiplier,
        ),
        (10, 5, None, d(25, 1), d(40, 1))
    );
    assert_eq!(
        (
            candidates[2].rsi_period,
            candidates[2].rsi_order,
            candidates[2].max_age,
            candidates[2].atr_sl_multiplier,
            candidates[2].atr_tp_multiplier,
        ),
        (21, 5, Some(12), d(20, 1), d(40, 1))
    );
    assert_eq!(
        (
            candidates[3].rsi_period,
            candidates[3].rsi_order,
            candidates[3].max_age,
            candidates[3].atr_sl_multiplier,
            candidates[3].atr_tp_multiplier,
        ),
        (14, 5, Some(4), d(15, 1), d(20, 1))
    );
    assert_eq!(
        (
            candidates[4].rsi_period,
            candidates[4].rsi_order,
            candidates[4].max_age,
            candidates[4].atr_sl_multiplier,
            candidates[4].atr_tp_multiplier,
        ),
        (14, 5, Some(12), d(15, 1), d(40, 1))
    );
}

#[test]
fn scenarios_cover_seven_per_side_cost_levels() {
    let scenarios = scenarios();

    let labels: Vec<&str> = scenarios
        .iter()
        .map(|scenario| scenario.label.as_str())
        .collect();
    assert_eq!(
        labels,
        ["0.0bps", "2.5bps", "5.0bps", "7.5bps", "10.0bps", "15.0bps", "30.0bps"]
    );

    let levels: Vec<Decimal> = scenarios
        .iter()
        .map(|scenario| scenario.fee_bps + scenario.slip_bps)
        .collect();
    assert_eq!(
        levels,
        [
            d(0, 1),
            d(25, 1),
            d(50, 1),
            d(75, 1),
            d(100, 1),
            d(150, 1),
            d(300, 1)
        ]
    );
    for scenario in &scenarios {
        assert_eq!(scenario.slip_bps, Decimal::ZERO);
    }
}

#[test]
fn break_even_is_zero_when_the_first_level_is_not_profitable() {
    let points = [(d(0, 1), d(-2, 0)), (d(25, 1), d(-4, 0))];

    assert_eq!(break_even_bps(&points), Some(Decimal::ZERO));
}

#[test]
fn break_even_interpolates_between_positive_and_non_positive_levels() {
    let points = [(d(50, 1), d(2, 0)), (d(100, 1), d(-2, 0))];

    assert_eq!(break_even_bps(&points), Some(d(75, 1)));
}

#[test]
fn break_even_is_none_when_still_profitable_at_the_highest_level() {
    let points = [(d(0, 1), d(2, 0)), (d(300, 1), d(1, 0))];

    assert_eq!(break_even_bps(&points), None);
}

#[test]
fn break_even_is_none_for_empty_input() {
    assert_eq!(break_even_bps(&[]), None);
}
