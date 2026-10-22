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
    let mut stress = metrics(40, 6, 10);
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
    assert!(text.contains("## 9. Commands executed"));
    assert!(text.contains("## 10. Recommendation"));
    assert!(text.contains("validation"));
    assert!(text.contains("holdout"));
    assert!(text.contains("no candidate"));
    assert!(text.contains("SMA_PERIOD"));

    std::fs::remove_dir_all(path.parent().expect("parent")).ok();
}
