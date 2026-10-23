#![cfg(feature = "harness")]
use analyzer::harness::sweep::sampling::{generate, Candidate, SEED_DEFAULT};
use rust_decimal::Decimal;
use std::collections::HashSet;

#[test]
fn generates_exactly_thirty_unique_candidates_including_baseline() {
    let c = generate(SEED_DEFAULT);
    assert_eq!(c.len(), 30);
    let keys: HashSet<_> = c.iter().map(Candidate::key).collect();
    assert_eq!(keys.len(), 30, "no duplicates");
    assert_eq!(c[0].label, "baseline");
}

#[test]
fn baseline_uses_the_new_defaults() {
    let c = generate(SEED_DEFAULT);
    assert_eq!(c[0].rsi_period, 21);
    assert_eq!(c[0].rsi_long_threshold, Decimal::from(60));
    assert_eq!(c[0].rsi_short_threshold, Decimal::from(50));
    assert!(!c[0].atr_enabled);
}

#[test]
fn is_deterministic_for_a_fixed_seed() {
    assert_eq!(generate(SEED_DEFAULT), generate(SEED_DEFAULT));
    assert_ne!(generate(SEED_DEFAULT), generate(1));
}
