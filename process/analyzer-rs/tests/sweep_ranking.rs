#![cfg(feature = "harness")]

use analyzer_rs::harness::sweep::evaluate::PartitionMetrics;
use analyzer_rs::harness::sweep::ranking::{rank, CandidateOutcome, Selection, Thresholds};
use analyzer_rs::harness::sweep::report::{
    write_leaderboard_csv, write_markdown, write_results_json, CandidatePlan, DatasetSummary,
    SweepResults,
};
use analyzer_rs::harness::sweep::sampling::{self, Candidate};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn metrics(trades: usize, expectancy: i64, dd_pct: i64) -> PartitionMetrics {
    PartitionMetrics {
        trades,
        net_expectancy: d(expectancy),
        max_drawdown_pct: d(dd_pct),
        ..Default::default()
    }
}

fn candidate(label: &str) -> Candidate {
    Candidate {
        label: label.to_string(),
        ..sampling::baseline()
    }
}

fn outcome(
    label: &str,
    trades: usize,
    expectancy: i64,
    dd_pct: i64,
    fold_signs: [i64; 3],
    symbol_signs: [i64; 3],
) -> CandidateOutcome {
    CandidateOutcome {
        label: label.to_string(),
        origin: "joint".to_string(),
        candidate: candidate(label),
        pooled_validation: metrics(trades, expectancy, dd_pct),
        per_symbol_validation: symbol_signs
            .iter()
            .enumerate()
            .map(|(index, sign)| (format!("SYM{index}"), metrics(trades / 3, *sign, dd_pct)))
            .collect(),
        folds: fold_signs
            .iter()
            .map(|sign| metrics(trades / 3, *sign, dd_pct))
            .collect(),
        base: BTreeMap::new(),
        stress: BTreeMap::new(),
    }
}

fn outcome_with_symbols(
    label: &str,
    trades: usize,
    expectancy: i64,
    dd_pct: i64,
    fold_signs: [i64; 3],
    symbols: Vec<(&str, PartitionMetrics)>,
) -> CandidateOutcome {
    CandidateOutcome {
        label: label.to_string(),
        origin: "joint".to_string(),
        candidate: candidate(label),
        pooled_validation: metrics(trades, expectancy, dd_pct),
        per_symbol_validation: symbols
            .into_iter()
            .map(|(symbol, metrics)| (symbol.to_string(), metrics))
            .collect(),
        folds: fold_signs
            .iter()
            .map(|sign| metrics(trades / 3, *sign, dd_pct))
            .collect(),
        base: BTreeMap::new(),
        stress: BTreeMap::new(),
    }
}

#[test]
fn rejects_candidates_below_trade_floor() {
    let thresholds = Thresholds::default();
    let weak = outcome("weak", 5, 10, 5, [1, 1, 1], [1, 1, 1]);
    let strong = outcome("strong", 40, 8, 10, [1, 1, -1], [1, 1, -1]);

    let selection = rank(&[weak, strong], &thresholds);

    assert_eq!(selection.winners, vec!["strong".to_string()]);
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("weak") && note.contains("trades")),
        "expected a trade-floor note naming weak, got {:?}",
        selection.notes
    );
}

#[test]
fn rejects_negative_validation_expectancy() {
    let bad = outcome("negative", 40, -5, 10, [1, 1, 1], [1, 1, 1]);
    let selection = rank(&[bad], &Thresholds::default());

    assert!(selection.winners.is_empty());
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("negative") && note.contains("expectancy")),
        "expected an expectancy note, got {:?}",
        selection.notes
    );
}

#[test]
fn rejects_candidates_above_drawdown_limit() {
    let bad = outcome("high-dd", 40, 8, 25, [1, 1, 1], [1, 1, 1]);
    let selection = rank(&[bad], &Thresholds::default());

    assert!(selection.winners.is_empty());
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("high-dd") && note.contains("drawdown")),
        "expected a drawdown note, got {:?}",
        selection.notes
    );
}

#[test]
fn rejects_on_per_symbol_drawdown_even_when_pooled_drawdown_is_within_limit() {
    let bad = outcome_with_symbols(
        "symbol-dd",
        40,
        8,
        10,
        [1, 1, -1],
        vec![
            ("AAA", metrics(14, 5, 10)),
            ("BBB", metrics(14, 5, 25)),
            ("CCC", metrics(12, 5, 10)),
        ],
    );
    let selection = rank(&[bad], &Thresholds::default());

    assert!(
        selection.winners.is_empty(),
        "per-symbol drawdown above the limit must reject the candidate"
    );
    assert!(
        selection.notes.iter().any(|note| note.contains("symbol-dd")
            && note.contains("BBB")
            && note.contains("drawdown")),
        "expected a per-symbol drawdown note, got {:?}",
        selection.notes
    );
}

#[test]
fn rejects_concentrated_trades_and_accepts_the_exact_share_boundary() {
    let thresholds = Thresholds::default();
    let concentrated = outcome_with_symbols(
        "concentrated",
        40,
        8,
        10,
        [1, 1, -1],
        vec![
            ("AAA", metrics(32, 5, 10)),
            ("BBB", metrics(4, 5, 10)),
            ("CCC", metrics(4, 5, 10)),
        ],
    );
    let boundary = outcome_with_symbols(
        "boundary",
        40,
        8,
        10,
        [1, 1, -1],
        vec![
            ("AAA", metrics(28, 5, 10)),
            ("BBB", metrics(6, 5, 10)),
            ("CCC", metrics(6, 5, 10)),
        ],
    );

    let rejected = rank(&[concentrated], &thresholds);
    assert!(rejected.winners.is_empty());
    assert!(
        rejected
            .notes
            .iter()
            .any(|note| note.contains("concentrated") && note.contains("single symbol")),
        "expected a concentration note, got {:?}",
        rejected.notes
    );

    // Exactly 28/40 = 70% must pass: the limit is inclusive.
    let accepted = rank(&[boundary], &thresholds);
    assert_eq!(accepted.winners, vec!["boundary".to_string()]);
}

#[test]
fn requires_enough_positive_symbols() {
    let one_positive = outcome("one-positive", 40, 8, 10, [1, 1, -1], [1, -1, -1]);
    let two_positive = outcome("two-positive", 40, 8, 10, [1, 1, -1], [1, 1, -1]);

    let rejected = rank(&[one_positive], &Thresholds::default());
    assert!(rejected.winners.is_empty());
    assert!(
        rejected
            .notes
            .iter()
            .any(|note| note.contains("one-positive")
                && note.contains("symbol")
                && note.contains("positive")),
        "expected a positive-symbol note, got {:?}",
        rejected.notes
    );

    let accepted = rank(&[two_positive], &Thresholds::default());
    assert_eq!(accepted.winners, vec!["two-positive".to_string()]);
}

#[test]
fn reports_expectancy_first_when_multiple_criteria_fail() {
    let bad = outcome("multi-fail", 40, -5, 25, [-1, -1, -1], [-1, -1, -1]);
    let selection = rank(&[bad], &Thresholds::default());
    let first = selection.notes.first().expect("a rejection note");

    assert!(first.contains("multi-fail"));
    assert!(
        first.contains("net expectancy"),
        "expectancy must be the first criterion checked, got {first:?}"
    );
    assert!(!first.contains("drawdown"));
    assert!(!first.contains("fold"));
}

#[test]
fn ranks_qualifiers_by_drawdown_folds_trades_then_label() {
    let candidates = vec![
        outcome("high_dd", 40, 10, 15, [1, 1, 1], [1, 1, 1]),
        outcome("low_dd_low_folds", 40, 10, 10, [1, 1, -1], [1, 1, 1]),
        outcome("few_trades", 35, 10, 10, [1, 1, 1], [1, 1, 1]),
        outcome("more_trades", 40, 10, 10, [1, 1, 1], [1, 1, 1]),
        outcome("beta", 40, 10, 10, [1, 1, 1], [1, 1, 1]),
        outcome("alpha", 40, 10, 10, [1, 1, 1], [1, 1, 1]),
    ];

    let selection = rank(&candidates, &Thresholds::default());

    assert_eq!(
        selection.winners,
        vec![
            "alpha".to_string(),
            "beta".to_string(),
            "more_trades".to_string(),
            "few_trades".to_string(),
            "low_dd_low_folds".to_string(),
            "high_dd".to_string(),
        ]
    );
}

#[test]
fn accepts_exact_threshold_boundaries() {
    let boundary = outcome_with_symbols(
        "boundary",
        30,
        5,
        20,
        [1, 1, -1],
        vec![("AAA", metrics(15, 5, 20)), ("BBB", metrics(15, 5, 20))],
    );

    let selection = rank(&[boundary], &Thresholds::default());

    assert_eq!(selection.winners, vec!["boundary".to_string()]);
}

#[test]
fn rejects_candidates_with_too_few_positive_folds() {
    let bad = outcome("one-fold", 40, 8, 10, [1, -1, -1], [1, 1, 1]);
    let selection = rank(&[bad], &Thresholds::default());

    assert!(selection.winners.is_empty());
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("one-fold") && note.contains("fold")),
        "expected a fold note, got {:?}",
        selection.notes
    );
}

#[test]
fn reports_no_candidate_when_none_qualify() {
    let bad = outcome("bad", 40, -5, 10, [-1, -1, -1], [-1, -1, -1]);
    let selection = rank(&[bad], &Thresholds::default());

    assert!(selection.winners.is_empty());
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("no candidate")),
        "expected a no-candidate note, got {:?}",
        selection.notes
    );
}

#[test]
fn flags_negative_stress_without_dropping_the_winner() {
    let mut strong = outcome("strong", 40, 8, 10, [1, 1, -1], [1, 1, -1]);
    let mut stress = metrics(40, -1, 10);
    stress.net_pnl = d(-5);
    strong.stress.insert("validation".to_string(), stress);

    let selection = rank(&[strong], &Thresholds::default());

    assert_eq!(selection.winners, vec!["strong".to_string()]);
    assert!(
        selection
            .notes
            .iter()
            .any(|note| note.contains("strong") && note.contains("stress")),
        "expected a stress flag note, got {:?}",
        selection.notes
    );
}

#[test]
fn stress_flag_uses_expectancy_not_net_pnl() {
    let mut strong = outcome("strong", 40, 8, 10, [1, 1, -1], [1, 1, -1]);
    let mut stress = metrics(40, 3, 10);
    // Negative net PnL but positive expectancy mirrors a few large open losses; it must not flag.
    stress.net_pnl = d(-5);
    strong.stress.insert("validation".to_string(), stress);

    let selection = rank(&[strong], &Thresholds::default());

    assert_eq!(selection.winners, vec!["strong".to_string()]);
    assert!(
        selection.notes.iter().all(|note| !note.contains("stress")),
        "negative stress net PnL with positive expectancy must not flag, got {:?}",
        selection.notes
    );
}

fn temp_file(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "analyzer-sweep-ranking-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("temp dir");
    directory.join(name)
}

fn sample_results() -> SweepResults {
    let mut baseline = outcome("baseline", 40, 8, 10, [1, 1, -1], [1, 1, -1]);
    baseline.origin = "baseline".to_string();
    baseline.base.insert("train".to_string(), metrics(40, 5, 8));
    baseline
        .base
        .insert("validation".to_string(), metrics(40, 8, 10));
    baseline
        .base
        .insert("holdout".to_string(), metrics(40, 6, 12));
    for (index, fold) in baseline.folds.iter().enumerate() {
        baseline
            .base
            .insert(format!("fold{}_val", index + 1), fold.clone());
    }
    let mut stress = metrics(40, 4, 11);
    stress.net_pnl = d(-2);
    baseline.stress.insert("validation".to_string(), stress);

    let mut outcomes = BTreeMap::new();
    outcomes.insert("1hour".to_string(), vec![baseline]);

    let mut selections = BTreeMap::new();
    selections.insert(
        "1hour".to_string(),
        Selection {
            winners: Vec::new(),
            notes: vec!["no candidate meets the criteria".to_string()],
        },
    );

    let mut candidate_plan = BTreeMap::new();
    candidate_plan.insert(
        "1hour".to_string(),
        CandidatePlan {
            seed: 20261022,
            candidates: vec![sampling::baseline()],
        },
    );

    SweepResults {
        generated_at: "2026-10-22T00:00:00Z".to_string(),
        as_of_ms: 1_700_000_000_000,
        months: 24,
        seed: 20261022,
        symbols: vec!["BTC-USDT".to_string()],
        timeframes: vec!["1hour".to_string()],
        fee_bps: 10,
        slip_bps: 5,
        stress_fee_bps: 20,
        stress_slip_bps: 10,
        init_fund: 1000,
        datasets: vec![DatasetSummary {
            symbol: "BTC-USDT".to_string(),
            timeframe: "1hour".to_string(),
            requested_first_ms: 1_600_000_000_000,
            requested_last_ms: 1_700_000_000_000,
            first_open_time_ms: 1_600_000_000_000,
            last_open_time_ms: 1_700_000_000_000,
            candles: 1000,
            requested_candles: 1000,
            expected_candles: 1000,
            missing_candles: 0,
            duplicates_dropped: 0,
            pages: 1,
            content_sha256: "0".repeat(64),
        }],
        outcomes,
        selections,
        errors: Vec::new(),
        thresholds: Thresholds::default(),
        candidate_plan,
    }
}

#[test]
fn writes_leaderboard_csv_with_expected_columns() {
    let path = temp_file("leaderboard.csv");
    write_leaderboard_csv(&path, &sample_results()).expect("csv writes");

    let text = std::fs::read_to_string(&path).expect("csv readable");
    let header = text.lines().next().expect("header");
    assert!(header.starts_with("timeframe,symbol_or_pooled,label,origin"));
    assert!(header.contains("stress_validation_net_pnl"));
    assert!(header.contains("stress_validation_expectancy"));
    assert!(text.contains("validation"));
    assert!(text.contains("holdout"));
    assert!(text.contains("pooled"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn writes_results_json_with_thresholds_and_candidate_plan() {
    let path = temp_file("results.json");
    write_results_json(&path, &sample_results()).expect("json writes");

    let text = std::fs::read_to_string(&path).expect("json readable");
    assert!(text.contains("\"thresholds\""));
    assert!(text.contains("\"candidate_plan\""));
    assert!(text.contains("\"outcomes\""));
    assert!(text.contains("\"candidates\""));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn writes_markdown_with_required_sections() {
    let path = temp_file("report.md");
    write_markdown(&path, &sample_results()).expect("markdown writes");

    let text = std::fs::read_to_string(&path).expect("markdown readable");
    assert!(text.contains("## 1. Run metadata"));
    assert!(text.contains("## 2. Data quality"));
    assert!(text.contains("## 3. Baseline vs candidates"));
    assert!(text.contains("## 4. Exact configurations tested and reasons"));
    assert!(text.contains("## 5. Selection"));
    assert!(text.contains("## 6. Walk-forward"));
    assert!(text.contains("## 7. Parameter sensitivity"));
    assert!(text.contains("## 8. Cost assumptions"));
    assert!(text.contains("## 9. Command (synthesized from run configuration)"));
    assert!(text.contains("## 10. Recommendation"));
    assert!(text.contains("### Errors"));
    assert!(text.contains("validation"));
    assert!(text.contains("holdout"));
    assert!(text.contains("no candidate"));
    assert!(text.contains("SMA_PERIOD"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

fn results_with_selection(winners: &[&str]) -> SweepResults {
    let mut results = sample_results();
    results.selections.insert(
        "1hour".to_string(),
        Selection {
            winners: winners.iter().map(|winner| winner.to_string()).collect(),
            notes: Vec::new(),
        },
    );
    results
}

fn markdown_for(results: &SweepResults, name: &str) -> (PathBuf, String) {
    let path = temp_file(name);
    write_markdown(&path, results).expect("markdown writes");
    let text = std::fs::read_to_string(&path).expect("markdown readable");
    (path, text)
}

#[test]
fn recommends_retain_baseline_when_only_baseline_wins() {
    let (path, text) = markdown_for(&results_with_selection(&["baseline"]), "retain.md");

    assert!(
        text.contains("**retain baseline**"),
        "expected the exact retain-baseline label, got {text}"
    );

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn recommends_larger_experiment_when_a_non_baseline_wins() {
    let (path, text) = markdown_for(
        &results_with_selection(&["baseline", "candidate"]),
        "larger.md",
    );

    assert!(
        text.contains("**larger experiment**"),
        "expected the exact larger-experiment label, got {text}"
    );

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn recommends_insufficient_evidence_when_no_candidate_wins() {
    let (path, text) = markdown_for(&sample_results(), "insufficient.md");

    assert!(
        text.contains("**insufficient evidence**"),
        "expected the exact insufficient-evidence label, got {text}"
    );

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn recommends_insufficient_evidence_when_errors_are_recorded() {
    let mut results = results_with_selection(&["baseline"]);
    results.errors.push("fetch failed for BTC-USDT".to_string());
    let (path, text) = markdown_for(&results, "errors.md");

    assert!(text.contains("**insufficient evidence**"));
    assert!(text.contains("### Errors"));
    assert!(text.contains("fetch failed for BTC-USDT"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn recommends_insufficient_evidence_when_a_requested_timeframe_has_no_selection() {
    let mut results = results_with_selection(&["baseline"]);
    results.timeframes.push("4hour".to_string());
    let (path, text) = markdown_for(&results, "missing-selection.md");

    assert!(text.contains("**insufficient evidence**"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn report_lists_data_quality_warnings() {
    let mut results = sample_results();
    results.datasets[0].missing_candles = 2;
    results.datasets[0].duplicates_dropped = 1;
    let (path, text) = markdown_for(&results, "warnings.md");

    assert!(text.contains("### Warnings"), "got {text}");
    assert!(text.contains("Warning: BTC-USDT 1hour has 2 missing candle(s)"));
    assert!(text.contains("Warning: BTC-USDT 1hour dropped 1 duplicate candle(s)"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn commands_join_symbols_without_spaces() {
    let mut results = sample_results();
    results.symbols = vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()];
    let (path, text) = markdown_for(&results, "commands.md");

    assert!(text.contains("--symbols BTC-USDT,ETH-USDT"), "got {text}");
    assert!(!text.contains("--symbols BTC-USDT, ETH-USDT"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn reports_errors_when_every_dataset_fetch_failed() {
    let mut results = sample_results();
    results.datasets = Vec::new();
    results.errors = vec!["fetch failed".to_string()];
    let (path, text) = markdown_for(&results, "all-fetches-failed.md");

    assert!(text.contains("No datasets recorded."), "got {text}");
    assert!(text.contains("### Errors"), "got {text}");
    assert!(text.contains("fetch failed"), "got {text}");

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

fn with_full_base(mut outcome: CandidateOutcome) -> CandidateOutcome {
    outcome.base.insert("train".to_string(), metrics(40, 5, 8));
    outcome
        .base
        .insert("validation".to_string(), metrics(40, 8, 10));
    outcome
        .base
        .insert("holdout".to_string(), metrics(40, 6, 12));
    for (index, fold) in outcome.folds.clone().iter().enumerate() {
        outcome
            .base
            .insert(format!("fold{}_val", index + 1), fold.clone());
    }
    outcome
}

#[test]
fn artifacts_are_byte_identical_across_writes() {
    // Deliberately unsorted input must not leak into any artifact.
    let mut results = sample_results();
    results.outcomes.insert(
        "1hour".to_string(),
        vec![
            with_full_base(outcome("zzz", 40, 8, 10, [1, 1, -1], [1, 1, -1])),
            with_full_base(outcome("aaa", 40, 8, 10, [1, 1, -1], [1, 1, -1])),
        ],
    );
    type ArtifactWriter = fn(&std::path::Path, &SweepResults) -> Result<(), String>;
    let writers: [ArtifactWriter; 3] = [write_results_json, write_markdown, write_leaderboard_csv];
    let extensions = ["json", "md", "csv"];

    for (index, writer) in writers.iter().enumerate() {
        let extension = extensions[index];
        let first = temp_file(&format!("deterministic-1.{extension}"));
        let second = temp_file(&format!("deterministic-2.{extension}"));

        writer(&first, &results).expect("first write");
        writer(&second, &results).expect("second write");
        let first_bytes = std::fs::read(&first).expect("first readable");
        let second_bytes = std::fs::read(&second).expect("second readable");
        assert_eq!(
            first_bytes, second_bytes,
            "{extension} artifact ordering is not deterministic"
        );
        let text = String::from_utf8(first_bytes).expect("artifact is utf-8");
        let aaa = text.find("aaa").expect("aaa outcome is present");
        let zzz = text.find("zzz").expect("zzz outcome is present");
        assert!(aaa < zzz, "{extension} outcomes are not sorted by label");

        std::fs::remove_dir_all(first.parent().expect("parent")).ok();
        std::fs::remove_dir_all(second.parent().expect("parent")).ok();
    }
}

#[test]
fn leaderboard_csv_rejects_outcomes_missing_base_partitions() {
    let mut results = sample_results();
    let mut incomplete = outcome("incomplete", 40, 8, 10, [1, 1, 1], [1, 1, 1]);
    incomplete
        .base
        .insert("train".to_string(), metrics(40, 5, 8));
    incomplete
        .base
        .insert("validation".to_string(), metrics(40, 8, 10));
    // "holdout" and the fold partitions are deliberately absent.
    results
        .outcomes
        .insert("1hour".to_string(), vec![incomplete]);
    let path = temp_file("missing-partition.csv");

    let error = write_leaderboard_csv(&path, &results).expect_err("missing partitions must fail");

    assert!(
        error.contains("missing base partitions"),
        "expected a missing-base-partitions error, got {error}"
    );

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn leaderboard_csv_pooled_validation_uses_base_partition_metrics() {
    let mut results = sample_results();
    let mut diverging = with_full_base(outcome("diverging", 40, 8, 10, [1, 1, -1], [1, 1, -1]));
    // `pooled_validation` stays at 8; the recorded validation partition is clearly different.
    diverging
        .base
        .insert("validation".to_string(), metrics(41, 99, 33));
    results
        .outcomes
        .insert("1hour".to_string(), vec![diverging]);
    let path = temp_file("base-validation.csv");
    write_leaderboard_csv(&path, &results).expect("csv writes");

    let mut reader = csv::Reader::from_path(&path).expect("csv readable");
    let headers = reader.headers().expect("headers").clone();
    let net_expectancy = headers
        .iter()
        .position(|column| column == "net_expectancy")
        .expect("net_expectancy column");
    let validation_row = reader
        .records()
        .map(|record| record.expect("record"))
        .find(|record| {
            record.get(1) == Some("pooled")
                && record.get(2) == Some("diverging")
                && record.get(11) == Some("validation")
        })
        .expect("pooled validation row");

    assert_eq!(
        validation_row.get(net_expectancy),
        Some("99"),
        "pooled validation must come from base[\"validation\"], not pooled_validation"
    );

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}

#[test]
fn drawdown_failure_takes_priority_over_folds_and_trades() {
    // Positive expectancy, excessive drawdown, too few positive folds, and too few trades.
    let bad = outcome("dd-first", 5, 8, 25, [-1, -1, -1], [-1, -1, -1]);
    let selection = rank(&[bad], &Thresholds::default());

    let reasons: Vec<&String> = selection
        .notes
        .iter()
        .filter(|note| note.contains("dd-first"))
        .collect();
    assert_eq!(
        reasons.len(),
        1,
        "expected exactly one rejection note, got {reasons:?}"
    );
    assert!(reasons[0].contains("drawdown"), "got {reasons:?}");
    assert!(!reasons[0].contains("fold"), "got {reasons:?}");
    assert!(!reasons[0].contains("trades"), "got {reasons:?}");
}

#[test]
fn fold_failure_takes_priority_over_trades() {
    // Positive expectancy, acceptable drawdown, too few positive folds, and too few trades.
    let bad = outcome("fold-first", 5, 8, 10, [-1, -1, -1], [1, -1, -1]);
    let selection = rank(&[bad], &Thresholds::default());

    let reasons: Vec<&String> = selection
        .notes
        .iter()
        .filter(|note| note.contains("fold-first"))
        .collect();
    assert_eq!(
        reasons.len(),
        1,
        "expected exactly one rejection note, got {reasons:?}"
    );
    assert!(reasons[0].contains("fold"), "got {reasons:?}");
    assert!(!reasons[0].contains("trades"), "got {reasons:?}");
}
