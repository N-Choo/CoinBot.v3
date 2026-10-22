#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::sweep::confirmation::{decide, evaluate_full_period};
use analyzer_rs::harness::sweep::evaluate::PartitionMetrics;
use analyzer_rs::harness::sweep::execution::{EntryMode, ExecPartition};
use analyzer_rs::harness::sweep::freeze::{self, DecisionRule};
use analyzer_rs::harness::sweep::sampling;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `process/analyzer-rs`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lock_path() -> PathBuf {
    repo_root().join("freeze/confirmation-lock.json")
}

fn empty_partition() -> ExecPartition {
    ExecPartition {
        metrics: PartitionMetrics::default(),
        signals: 0,
        filled: 0,
        expired: 0,
        skipped_funds: 0,
        invalid_levels: 0,
        capital_fill_rate: Decimal::ZERO,
        execution_fill_rate: Decimal::ZERO,
    }
}

fn symbol_partition(expectancy: i64) -> ExecPartition {
    let mut partition = empty_partition();
    partition.metrics.trades = 1;
    partition.metrics.net_expectancy = Decimal::from(expectancy);
    partition
}

fn pooled_partition(trades: usize, expectancy: i64, profit_factor: Option<i64>) -> ExecPartition {
    let mut partition = empty_partition();
    partition.metrics.trades = trades;
    partition.metrics.net_expectancy = Decimal::from(expectancy);
    partition.metrics.net_profit_factor = profit_factor.map(Decimal::from);
    partition.metrics.gross_profit = Decimal::from(10);
    partition
}

fn decision_rule() -> DecisionRule {
    DecisionRule {
        min_pooled_trades: 50,
        require_positive_net_expectancy: true,
        require_profit_factor_above: 1.0,
        min_positive_symbols: 2,
    }
}

#[test]
fn lock_loads_and_hash_matches_the_repository() {
    let lock = freeze::load(&lock_path()).expect("lock loads");
    let actual = freeze::compute_hash(&repo_root(), &lock.frozen_files).expect("hash computes");

    assert_eq!(actual, lock.signal_generator_sha256);
    freeze::verify(&repo_root(), &lock).expect("frozen files match the lock");
}

#[test]
fn wrong_expected_hash_fails_verification() {
    let mut lock = freeze::load(&lock_path()).expect("lock loads");
    lock.signal_generator_sha256 =
        "0000000000000000000000000000000000000000000000000000000000000000".into();

    let error = freeze::verify(&repo_root(), &lock).expect_err("mismatch must fail");

    assert!(error.contains("frozen-file hash mismatch"), "{error}");
}

#[test]
fn locked_config_is_the_baseline_with_max_age_four() {
    let lock = freeze::load(&lock_path()).expect("lock loads");
    let config = lock.config.to_config().expect("frozen config resolves");

    let baseline = sampling::baseline().to_config();
    assert_eq!(config, baseline);
    assert_eq!(lock.config.max_age, Some(4));
    assert_eq!(config.window, 100);
}

#[test]
fn locked_model_is_retest_2c_b2() {
    let lock = freeze::load(&lock_path()).expect("lock loads");
    let model = lock.model.to_exec_model().expect("frozen model resolves");

    assert_eq!(model.label, "retest_2c_b2");
    match model.entry {
        EntryMode::Limit {
            valid_candles,
            buffer_bps,
            offset_bps,
        } => {
            assert_eq!(valid_candles, 2);
            assert_eq!(buffer_bps, Decimal::from(2));
            assert_eq!(offset_bps, Decimal::from(25));
        }
        other => panic!("expected limit entry, got {other:?}"),
    }
    assert_eq!(model.maker_bps, Decimal::from(2));
    assert_eq!(model.taker_bps, Decimal::from(10));
    assert_eq!(model.slip_bps, Decimal::from(5));
}

#[test]
fn decision_passes_when_all_pre_registered_conditions_hold() {
    let pooled = pooled_partition(60, 2, Some(3));
    let symbols = [
        symbol_partition(1),
        symbol_partition(2),
        symbol_partition(-1),
    ];

    let decision = decide(&pooled, &symbols, &decision_rule());

    assert!(decision.passed);
    assert_eq!(decision.positive_symbols, 2);
    assert_eq!(decision.symbol_count, 3);
    assert!(decision.reasons.is_empty());
}

#[test]
fn decision_fails_with_too_few_trades() {
    let pooled = pooled_partition(49, 2, Some(3));
    let symbols = [
        symbol_partition(1),
        symbol_partition(2),
        symbol_partition(-1),
    ];

    let decision = decide(&pooled, &symbols, &decision_rule());

    assert!(!decision.passed);
    assert!(decision.reasons.iter().any(|reason| reason.contains("49")));
}

#[test]
fn decision_fails_on_non_positive_expectancy() {
    let pooled = pooled_partition(60, 0, Some(3));
    let symbols = [
        symbol_partition(1),
        symbol_partition(2),
        symbol_partition(-1),
    ];

    let decision = decide(&pooled, &symbols, &decision_rule());

    assert!(!decision.passed);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.contains("expectancy")));
}

#[test]
fn decision_fails_when_profit_factor_is_not_above_one() {
    let pooled = pooled_partition(60, 2, Some(1));
    let symbols = [
        symbol_partition(1),
        symbol_partition(2),
        symbol_partition(-1),
    ];

    let decision = decide(&pooled, &symbols, &decision_rule());

    assert!(!decision.passed);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.contains("profit factor")));
}

#[test]
fn decision_fails_without_two_positive_symbols() {
    let pooled = pooled_partition(60, 2, Some(3));
    let symbols = [
        symbol_partition(1),
        symbol_partition(-2),
        symbol_partition(-1),
    ];

    let decision = decide(&pooled, &symbols, &decision_rule());

    assert!(!decision.passed);
    assert_eq!(decision.positive_symbols, 1);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.contains("positive symbols")));
}

#[test]
fn evaluate_full_period_produces_finite_consistent_metrics() {
    let lock = freeze::load(&lock_path()).expect("lock loads");
    let config = lock.config.to_config().expect("frozen config resolves");
    let model = lock.model.to_exec_model().expect("frozen model resolves");
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

    let partition = evaluate_full_period(&dataset, &config, lock.config.max_age, 100, 1000, &model)
        .expect("full-period evaluation runs");

    assert_eq!(
        partition.filled + partition.expired + partition.skipped_funds + partition.invalid_levels,
        partition.signals
    );
    assert!(partition.metrics.trades <= partition.filled);
    assert_eq!(
        partition.metrics.net_pnl,
        partition.metrics.closed_net_pnl + partition.metrics.open_net_pnl
    );
    assert!(partition.capital_fill_rate >= Decimal::ZERO);
    assert!(partition.capital_fill_rate <= Decimal::ONE);
    assert!(partition.execution_fill_rate >= Decimal::ZERO);
    assert!(partition.execution_fill_rate <= Decimal::ONE);
    assert!(partition.execution_fill_rate >= partition.capital_fill_rate);

    let metrics = &partition.metrics;
    for (name, value) in [
        ("gross_profit", metrics.gross_profit),
        ("gross_loss", metrics.gross_loss),
        ("gross_pnl", metrics.gross_pnl),
        ("closed_net_pnl", metrics.closed_net_pnl),
        ("open_net_pnl", metrics.open_net_pnl),
        ("net_pnl", metrics.net_pnl),
        ("net_expectancy", metrics.net_expectancy),
        ("net_expectancy_r", metrics.net_expectancy_r),
        ("win_rate", metrics.win_rate),
        ("max_drawdown", metrics.max_drawdown),
        ("max_drawdown_pct", metrics.max_drawdown_pct),
        ("time_in_market_pct", metrics.time_in_market_pct),
        ("expectancy_stderr", metrics.expectancy_stderr),
        ("capital_fill_rate", partition.capital_fill_rate),
        ("execution_fill_rate", partition.execution_fill_rate),
    ] {
        let value = value
            .to_f64()
            .unwrap_or_else(|| panic!("{name} is not finite"));
        assert!(value.is_finite(), "{name} is not finite");
    }
    if let Some(profit_factor) = metrics.net_profit_factor {
        assert!(profit_factor.to_f64().is_some_and(f64::is_finite));
    }
}
