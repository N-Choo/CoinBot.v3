use analyzer_rs::config_env::{load_dotenv, parse_optional_usize_str, strategy_from_env};
use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::{fetch_dataset, load_csv, Dataset};
use analyzer_rs::harness::replay::build_replay;
use analyzer_rs::harness::report::{build_payload, write_report, PayloadInput};
use analyzer_rs::harness::sim::{simulate, SimSignal};
use analyzer_rs::harness::stats::summarize;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Symbol to replay; repeat for multiple symbols.
    #[arg(long = "symbol")]
    symbols: Vec<String>,
    #[arg(long)]
    timeframe: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
    #[arg(long)]
    init_fund: Option<i64>,
    #[arg(long, default_value = "process/analyzer-rs/out/replay.html")]
    out: PathBuf,
    /// Offline CSV: SYMBOL=path (repeatable).
    #[arg(long = "data")]
    data: Vec<String>,
    /// Reference time in ms; defaults to now so only closed candles replay.
    #[arg(long)]
    as_of: Option<i64>,
    #[arg(long)]
    generated_at: Option<String>,
    #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
    max_gap_candles: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    let cli = Cli::parse();
    let strategy = strategy_from_env()?;
    let timeframe_name = cli
        .timeframe
        .clone()
        .or_else(|| std::env::var("KUCOIN_TIMEFRAME").ok())
        .unwrap_or_else(|| "15min".into());
    let timeframe = Timeframe::from_kucoin(&timeframe_name)
        .ok_or_else(|| format!("unsupported timeframe {timeframe_name:?}"))?;
    let init_fund = cli
        .init_fund
        .or_else(|| {
            std::env::var("BENCHMARK_INIT_FUND")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(1000);
    let as_of = cli
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    if cli.max_gap_candles < 0 {
        return Err(format!(
            "--max-gap-candles must be >= 0 (got {})",
            cli.max_gap_candles
        )
        .into());
    }
    let base_url =
        std::env::var("KUCOIN_BASE_URL").unwrap_or_else(|_| "https://api.kucoin.com".into());
    let max_age_candles = match std::env::var("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES") {
        Ok(value) => parse_optional_usize_str("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES", &value)?,
        Err(_) => None,
    };

    let offline: std::collections::BTreeMap<String, PathBuf> = cli
        .data
        .iter()
        .filter_map(|entry| entry.split_once('='))
        .map(|(symbol, path)| (symbol.to_string(), PathBuf::from(path)))
        .collect();

    // Explicit --symbol wins; otherwise replay the offline datasets, then the benchmark list.
    let symbols = if !cli.symbols.is_empty() {
        cli.symbols.clone()
    } else if !offline.is_empty() {
        offline.keys().cloned().collect()
    } else {
        std::env::var("BENCHMARK_TICKERS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };

    let mut inputs = Vec::new();
    let mut failures = Vec::new();
    for symbol in symbols {
        let dataset: Result<Dataset, String> = if let Some(path) = offline.get(&symbol) {
            load_csv(
                &symbol,
                path,
                timeframe,
                as_of,
                5_000,
                cli.max_gap_candles,
                strategy.window,
                cli.limit,
            )
            .map_err(|error| error.to_string())
        } else {
            fetch_dataset(
                &symbol,
                timeframe,
                cli.limit,
                strategy.window,
                as_of,
                5_000,
                cli.max_gap_candles,
                &base_url,
            )
            .map_err(|error| error.to_string())
        };
        let dataset = match dataset {
            Ok(dataset) => dataset,
            Err(error) => {
                failures.push(serde_json::json!({"symbol": symbol, "error": error}));
                continue;
            }
        };
        let replay = build_replay(&dataset, &strategy, max_age_candles)?;
        let sim_signals: Vec<SimSignal> = replay
            .signals
            .iter()
            .map(|s| SimSignal {
                candle_index: s.candle_index,
                signal: s.signal.clone(),
            })
            .collect();
        let outcome = simulate(&dataset.candles, &sim_signals, init_fund, strategy.window);
        let last_close = dataset.candles.last().map(|c| c.close).unwrap_or_default();
        let summary = summarize(
            rust_decimal::Decimal::from(init_fund),
            &outcome,
            last_close,
            dataset.candles.len().saturating_sub(1),
            dataset.candles.len().saturating_sub(strategy.window),
        );
        inputs.push((symbol, replay, outcome, summary, dataset.provenance));
    }

    let generated_at = cli
        .generated_at
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let payload_inputs: Vec<PayloadInput<'_>> = inputs
        .iter()
        .map(|(symbol, replay, sim, summary, provenance)| PayloadInput {
            symbol,
            replay,
            sim,
            summary,
            provenance,
        })
        .collect();
    let mut payload = build_payload(&payload_inputs, &strategy, generated_at, failures);
    payload["config"]["timeframe"] = serde_json::json!(timeframe.as_kucoin());
    payload["config"]["limit"] = serde_json::json!(cli.limit);
    payload["config"]["init_fund"] = serde_json::json!(init_fund.to_string());
    payload["config"]["as_of_ms"] = serde_json::json!(as_of);
    write_report(&payload, &cli.out)?;
    println!("wrote {}", cli.out.display());
    Ok(())
}
