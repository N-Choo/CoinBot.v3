#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::sweep::costs::CostScenario;
use analyzer_rs::harness::sweep::evaluate::{evaluate_config, pool};
use analyzer_rs::harness::sweep::partition::chronological_splits;
use analyzer_rs::harness::sweep::ranking::{rank, CandidateOutcome, Thresholds};
use analyzer_rs::harness::sweep::report::{
    write_leaderboard_csv, write_markdown, write_results_json, BaselineConfig, CandidatePlan,
    DatasetSummary, SplitSummary, SweepResults,
};
use analyzer_rs::harness::sweep::sampling;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("analyzer-rs-sweep-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn baseline_sweep_writes_all_artifacts() {
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
    let scenarios = [CostScenario::base(), CostScenario::stress()];

    let result = evaluate_config(
        &dataset,
        &baseline.to_config(),
        baseline.max_age,
        &splits,
        100,
        1000,
        &scenarios,
        true,
    )
    .expect("baseline evaluates");

    let base = result
        .partitions
        .get("base")
        .expect("base scenario")
        .clone();
    let stress = result
        .partitions
        .get("stress")
        .expect("stress scenario")
        .clone();
    for metrics in base.values().chain(stress.values()) {
        // Decimals cannot represent NaN or infinity, but converting catches a
        // nonsensical magnitude and every risk metric must be non-negative.
        for value in [
            metrics.net_expectancy,
            metrics.net_expectancy_r,
            metrics.max_drawdown,
            metrics.max_drawdown_pct,
        ] {
            assert!(
                value.to_f64().is_some_and(f64::is_finite),
                "metric is not finite: {value}"
            );
        }
        assert!(metrics.max_drawdown >= Decimal::ZERO);
        assert!(metrics.max_drawdown_pct >= Decimal::ZERO);
        assert!(metrics.win_rate >= Decimal::ZERO);
        assert!(metrics.time_in_market_pct >= Decimal::ZERO);
    }
    let validation = base
        .get("validation")
        .expect("validation partition")
        .clone();
    let per_symbol_validation = vec![(dataset.symbol.clone(), validation.clone())];
    let pooled_validation = pool(std::slice::from_ref(&validation));
    let folds = ["fold1_val", "fold2_val", "fold3_val"]
        .iter()
        .map(|name| {
            let metrics = base.get(*name).expect("fold partition");
            pool(std::slice::from_ref(metrics))
        })
        .collect();
    let outcome = CandidateOutcome {
        label: baseline.label.clone(),
        origin: baseline.origin.clone(),
        candidate: baseline.clone(),
        pooled_validation: pooled_validation.clone(),
        per_symbol_validation,
        folds,
        base,
        stress,
    };
    let selection = rank(std::slice::from_ref(&outcome), &Thresholds::default());

    let mut outcomes = BTreeMap::new();
    outcomes.insert("1day".to_string(), vec![outcome]);
    let mut selections = BTreeMap::new();
    selections.insert("1day".to_string(), selection);
    let mut candidate_plan = BTreeMap::new();
    candidate_plan.insert(
        "1day".to_string(),
        CandidatePlan {
            seed: sampling::SEED_DEFAULT,
            candidates: vec![baseline.clone()],
        },
    );

    let provenance = &dataset.provenance;
    let dataset_summary = DatasetSummary {
        symbol: dataset.symbol.clone(),
        timeframe: "1day".to_string(),
        requested_first_ms: provenance.effective_range.first_open_time_ms,
        requested_last_ms: provenance.effective_range.last_open_time_ms,
        first_open_time_ms: provenance.effective_range.first_open_time_ms,
        last_open_time_ms: provenance.effective_range.last_open_time_ms,
        candles: dataset.candles.len(),
        requested_candles: dataset.candles.len(),
        expected_candles: dataset.candles.len() + provenance.gaps,
        missing_candles: provenance.gaps,
        duplicates_dropped: 0,
        pages: 1,
        content_sha256: provenance.content_sha256.clone(),
    };

    let results = SweepResults {
        generated_at: "2026-10-22T00:00:00Z".to_string(),
        as_of_ms: i64::MAX / 2,
        months: 24,
        seed: sampling::SEED_DEFAULT,
        symbols: vec![dataset.symbol.clone()],
        timeframes: vec!["1day".to_string()],
        fee_bps: 10,
        slip_bps: 5,
        stress_fee_bps: 20,
        stress_slip_bps: 10,
        init_fund: 1000,
        datasets: vec![dataset_summary],
        splits: vec![SplitSummary::from_candles(
            &dataset.symbol,
            "1day",
            &dataset.candles,
        )],
        baseline_config: BaselineConfig::from_candidate(&baseline),
        baseline: baseline.clone(),
        baseline_source: "pinned study baseline; matches process/analyzer-rs/.env values"
            .to_string(),
        outcomes,
        selections,
        errors: Vec::new(),
        thresholds: Thresholds::default(),
        candidate_plan,
    };

    let dir = temp_dir();
    let leaderboard = dir.join("leaderboard.csv");
    let results_json = dir.join("results.json");
    let report = dir.join("report.md");
    write_leaderboard_csv(&leaderboard, &results).expect("leaderboard writes");
    write_results_json(&results_json, &results).expect("results json writes");
    write_markdown(&report, &results).expect("report writes");

    assert!(leaderboard.is_file());
    assert!(results_json.is_file());
    assert!(report.is_file());

    let json = std::fs::read_to_string(&results_json).expect("json readable");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("json parses");
    assert!(parsed.get("outcomes").is_some());
    assert!(parsed.get("selections").is_some());
    assert!(parsed.get("baseline_config").is_some());
    assert!(parsed.get("baseline_source").is_some());
    let splits = parsed
        .get("splits")
        .and_then(serde_json::Value::as_array)
        .expect("splits array");
    assert!(!splits.is_empty());
    for split in splits {
        let holdout_start = split["holdout"]["start_index"]
            .as_u64()
            .expect("holdout start index");
        for fold in split["folds"].as_array().expect("folds array") {
            let validation_end = fold["validation"]["end_index"]
                .as_u64()
                .expect("fold validation end index");
            assert!(
                validation_end <= holdout_start,
                "fold validation must end at or before the holdout start"
            );
        }
    }

    let markdown = std::fs::read_to_string(&report).expect("markdown readable");
    assert!(markdown.contains("validation"));
    assert!(markdown.contains("holdout"));
    assert!(markdown.contains("Baseline configuration"));
    assert!(markdown.contains("Chronological splits"));
    assert!(markdown.contains("Limitations"));
    assert!(markdown.contains("Val stderr"));

    let leaderboard_text = std::fs::read_to_string(&leaderboard).expect("csv readable");
    assert!(leaderboard_text.contains("validation"));
    assert!(leaderboard_text.contains("holdout"));

    std::fs::remove_dir_all(&dir).ok();
}
