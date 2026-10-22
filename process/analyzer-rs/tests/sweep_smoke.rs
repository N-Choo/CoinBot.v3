#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::sweep::costs::CostScenario;
use analyzer_rs::harness::sweep::evaluate::{evaluate_config, pool};
use analyzer_rs::harness::sweep::partition::chronological_splits;
use analyzer_rs::harness::sweep::ranking::{rank, CandidateOutcome, Thresholds};
use analyzer_rs::harness::sweep::report::{
    write_leaderboard_csv, write_markdown, write_results_json, CandidatePlan, DatasetSummary,
    SweepResults,
};
use analyzer_rs::harness::sweep::sampling;
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

    let markdown = std::fs::read_to_string(&report).expect("markdown readable");
    assert!(markdown.contains("validation"));
    assert!(markdown.contains("holdout"));

    let leaderboard_text = std::fs::read_to_string(&leaderboard).expect("csv readable");
    assert!(leaderboard_text.contains("validation"));
    assert!(leaderboard_text.contains("holdout"));

    std::fs::remove_dir_all(&dir).ok();
}
