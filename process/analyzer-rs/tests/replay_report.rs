#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{load_csv, Provenance};
use analyzer_rs::harness::replay::{build_replay, Replay};
use analyzer_rs::harness::report::{build_payload, write_report, PayloadInput};
use analyzer_rs::harness::sim::{simulate, SimOutcome, SimSignal};
use analyzer_rs::harness::stats::{summarize, SimSummary};
use analyzer_rs::strategy::config::Config;
use rust_decimal::Decimal;
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

struct Run {
    symbol: String,
    replay: Replay,
    sim: SimOutcome,
    summary: SimSummary,
    provenance: Provenance,
}

fn run(symbol: &str, fixture_name: &str, cfg: &Config) -> Run {
    let dataset = load_csv(
        symbol,
        &fixture(fixture_name),
        Timeframe::D1,
        i64::MAX,
        0,
        0,
        cfg.window,
        0,
    )
    .unwrap();
    let replay = build_replay(&dataset, cfg, None).unwrap();
    let signals: Vec<SimSignal> = replay
        .signals
        .iter()
        .map(|signal| SimSignal {
            candle_index: signal.candle_index,
            signal: signal.signal.clone(),
        })
        .collect();
    let sim = simulate(&dataset.candles, &signals, 1000, cfg.window);
    let last_close = dataset
        .candles
        .last()
        .map(|candle| candle.close)
        .unwrap_or_default();
    let summary = summarize(
        Decimal::from(1000),
        &sim,
        last_close,
        dataset.candles.len().saturating_sub(1),
        dataset.candles.len().saturating_sub(cfg.window),
    );
    Run {
        symbol: symbol.to_string(),
        replay,
        sim,
        summary,
        provenance: dataset.provenance,
    }
}

fn inputs(runs: &[Run]) -> Vec<PayloadInput<'_>> {
    runs.iter()
        .map(|run| PayloadInput {
            symbol: &run.symbol,
            replay: &run.replay,
            sim: &run.sim,
            summary: &run.summary,
            provenance: &run.provenance,
        })
        .collect()
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "analyzer-replay-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[test]
fn payload_has_two_symbols_and_report_fields() {
    let cfg = Config::default();
    let runs = vec![
        run("SYNTH-USDT", "synthetic_1day.csv", &cfg),
        run("BTC-USDT", "btc_usdt_1day.csv", &cfg),
    ];
    let payload = build_payload(
        &inputs(&runs),
        &cfg,
        "2026-10-21T00:00:00Z".into(),
        Vec::new(),
    );

    let symbols = payload["symbols"].as_array().unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0]["symbol"], "SYNTH-USDT");
    assert_eq!(symbols[1]["symbol"], "BTC-USDT");

    for (symbol, run) in symbols.iter().zip(&runs) {
        let candles = run.replay.candles.len();
        assert_eq!(symbol["candles"].as_array().unwrap().len(), candles);
        assert_eq!(symbol["rsi"].as_array().unwrap().len(), candles);
        assert_eq!(symbol["sma"].as_array().unwrap().len(), candles);
        assert_eq!(
            symbol["events"].as_array().unwrap().len(),
            run.replay.events.len()
        );
        assert_eq!(
            symbol["signals"].as_array().unwrap().len(),
            run.replay.signals.len()
        );
        assert_eq!(
            symbol["trades"].as_array().unwrap().len(),
            run.sim.trades.len()
        );
        assert_eq!(
            symbol["open_positions"].as_array().unwrap().len(),
            run.sim.open_positions.len()
        );
        for signal in symbol["signals"].as_array().unwrap() {
            assert!(matches!(signal["action"].as_str(), Some("Long" | "Short")));
            assert!(signal["candle_index"].is_u64());
            assert!(signal["confidence"].is_string());
            assert!(signal["divergence_kind"].is_string());
            assert!(signal["signal_candle_open_time_ms"].is_i64());
        }
        for trade in symbol["trades"].as_array().unwrap() {
            assert!(matches!(trade["side"].as_str(), Some("Long" | "Short")));
        }
        for position in symbol["open_positions"].as_array().unwrap() {
            assert!(matches!(position["side"].as_str(), Some("Long" | "Short")));
            assert!(position["unrealized"].is_string());
        }

        let sha = symbol["provenance"]["content_sha256"].as_str().unwrap();
        assert_eq!(sha.len(), 64, "content hash must be a sha256 hex digest");
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));

        let label = symbol["summary"]["returns_label"].as_str().unwrap();
        assert!(label.contains("gross simulated"));
        assert_eq!(
            symbol["summary"]["gaps"].as_u64().unwrap(),
            run.provenance.gaps as u64
        );
        assert!(symbol["summary"]["profit_factor"].is_string());
    }

    assert_eq!(payload["generated_at"], "2026-10-21T00:00:00Z");
    assert_eq!(payload["config"]["timeframe"], "");
    assert_eq!(payload["config"]["limit"], 0);
    assert_eq!(payload["config"]["init_fund"], "1000");
    assert_eq!(payload["config"]["as_of_ms"], 0);
    assert_eq!(payload["config"]["window"], cfg.window);
    assert_eq!(payload["config"]["fees"], false);
    assert_eq!(payload["config"]["rsi_period"], cfg.rsi_period);
    assert!(payload["failures"].as_array().unwrap().is_empty());
}

#[test]
fn report_is_self_contained_and_deterministic() {
    let cfg = Config::default();
    let runs = vec![run("SYNTH-USDT", "synthetic_1day.csv", &cfg)];
    let payload = build_payload(
        &inputs(&runs),
        &cfg,
        "2026-10-21T00:00:00Z".into(),
        Vec::new(),
    );

    let dir = temp_dir("deterministic");
    let first = dir.join("replay.html");
    let second = dir.join("nested/replay.html");
    write_report(&payload, &first).unwrap();
    write_report(&payload, &second).unwrap();

    let html = std::fs::read_to_string(&first).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("const REPLAY ="));
    assert!(html.contains("SYNTH-USDT"));
    assert!(html.contains("signals live-exact"));
    assert!(!html.contains("http://"));
    assert!(!html.contains("https://"));
    assert!(!html.contains("shared.css"));
    assert!(!html.contains("shared.js"));
    assert_eq!(html, std::fs::read_to_string(&second).unwrap());

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn report_escapes_script_end_and_lists_failures() {
    let cfg = Config::default();
    let runs = vec![run("SYNTH-USDT", "synthetic_1day.csv", &cfg)];
    let reason = "bad </script><script>alert(1)</script>";
    let failures = vec![json!({"symbol": "BAD-USDT", "error": reason})];
    let payload = build_payload(
        &inputs(&runs),
        &cfg,
        "2026-10-21T00:00:00Z".into(),
        failures,
    );

    assert_eq!(payload["failures"][0]["symbol"], "BAD-USDT");
    assert_eq!(payload["failures"][0]["error"], reason);

    let dir = temp_dir("escaping");
    let out = dir.join("replay.html");
    write_report(&payload, &out).unwrap();
    let html = std::fs::read_to_string(&out).unwrap();

    assert!(html.contains("BAD-USDT"));
    assert!(
        html.contains("\"symbol\":\"BAD-USDT\""),
        "the embedded JSON must carry the failure symbol"
    );
    assert!(
        html.contains("\"error\":\"bad "),
        "the embedded JSON must carry the failure error"
    );
    assert!(html.contains("failures"));
    assert!(
        html.contains("class=\"tab failed\""),
        "the template must render failures as failed tabs"
    );
    assert!(html.contains("disabled"));
    assert!(
        !html.contains(reason),
        "the raw closing script tag must not survive serialization"
    );
    assert!(
        html.contains("bad \\u003c/script>\\u003cscript>alert(1)\\u003c/script>"),
        "the embedded JSON must escape every < sequence"
    );

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn report_escapes_every_angle_bracket() {
    let cfg = Config::default();
    let runs = vec![run("SYNTH-USDT", "synthetic_1day.csv", &cfg)];
    let reason = "<!--<script  | <!--<script> | </SCRIPT>";
    let failures = vec![json!({"symbol": "BAD-USDT", "error": reason})];
    let payload = build_payload(
        &inputs(&runs),
        &cfg,
        "2026-10-21T00:00:00Z".into(),
        failures,
    );

    let dir = temp_dir("angle-brackets");
    let out = dir.join("replay.html");
    write_report(&payload, &out).unwrap();
    let html = std::fs::read_to_string(&out).unwrap();

    assert!(html.contains("\\u003c!--\\u003cscript "));
    assert!(html.contains("\\u003c!--\\u003cscript>"));
    assert!(html.contains("\\u003c/SCRIPT>"));
    assert!(
        !html.contains("<!--<script"),
        "a raw comment opening must not survive serialization"
    );
    assert!(
        !html.contains("</SCRIPT>"),
        "a raw case-insensitive closing script tag must not survive serialization"
    );

    std::fs::remove_dir_all(dir).unwrap();
}
