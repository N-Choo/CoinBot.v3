#![cfg(feature = "harness")]

use analyzer::harness::sweep::cost_ladder::{break_even_bps, frozen_candidates, scenarios};
use rust_decimal::Decimal;

fn d(value: i64, scale: u32) -> Decimal {
    Decimal::new(value, scale)
}

#[test]
fn frozen_candidates_are_baseline_first_and_include_the_three_winners() {
    let candidates = frozen_candidates();
    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].label, "baseline");
    let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "baseline",
            "long65-short50",
            "long60-short45",
            "long70-short50"
        ]
    );
    assert_eq!(candidates[0].origin, "baseline");
    let expected = [(60, 50), (65, 50), (60, 45), (70, 50)];
    for (candidate, (long, short)) in candidates.iter().zip(expected) {
        assert_eq!(candidate.rsi_period, 21);
        assert_eq!(candidate.rsi_long_threshold, Decimal::from(long));
        assert_eq!(candidate.rsi_short_threshold, Decimal::from(short));
    }
    for candidate in &candidates[1..] {
        assert_eq!(candidate.origin, "screening-4hour");
    }
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
