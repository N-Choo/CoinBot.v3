#![cfg(feature = "harness")]
use analyzer::harness::sweep::sampling::{generate, Candidate, SEED_DEFAULT};
use std::collections::HashSet;

#[test]
fn generates_exactly_thirty_unique_candidates_including_baseline() {
    let c = generate(SEED_DEFAULT);
    assert_eq!(c.len(), 30);
    let keys: HashSet<_> = c.iter().map(Candidate::key).collect();
    assert_eq!(keys.len(), 30, "no duplicates");
    assert_eq!(c[0].label, "baseline");
    assert!(c[0].sma_enabled && c[0].rsi_period == 14 && c[0].rsi_order == 5);
    assert_eq!(c[0].max_age, Some(12));
}

#[test]
fn sma_period_is_always_twenty() {
    for cand in generate(SEED_DEFAULT) {
        assert_eq!(cand.sma_period, 20);
    }
}

#[test]
fn is_deterministic_for_a_fixed_seed() {
    assert_eq!(generate(SEED_DEFAULT), generate(SEED_DEFAULT));
    assert_ne!(generate(SEED_DEFAULT), generate(1));
}
