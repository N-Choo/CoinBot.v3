use analyzer::config_env::{env_string, load_dotenv, parse_optional_usize_str, strategy_from_env};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::{fetch_dataset, load_csv, Dataset};
use analyzer::harness::replay::build_replay;
use analyzer::harness::report::{build_payload, write_report, PayloadInput};
use analyzer::harness::sim::{simulate, SimSignal};
use analyzer::harness::stats::summarize;
use analyzer::identity;
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Symbol to replay; repeat for multiple symbols. Defaults to --data keys, then BENCHMARK_TICKERS.
    #[arg(long = "symbol")]
    symbols: Vec<String>,
    /// Candle interval (15min, 4hour, 1day); defaults to KUCOIN_TIMEFRAME, then 15min.
    #[arg(long)]
    timeframe: Option<String>,
    /// Fetch request size; offline --data still replays every closed row.
    #[arg(long, default_value_t = 500)]
    limit: usize,
    /// Initial account fund; defaults to BENCHMARK_INIT_FUND, then 1000. Must be > 0.
    #[arg(long)]
    init_fund: Option<i64>,
    /// HTML report output path.
    #[arg(long, default_value = "process/analyzer/out/replay.html")]
    out: PathBuf,
    /// Offline CSV: SYMBOL=path (repeatable).
    #[arg(long = "data")]
    data: Vec<String>,
    /// Reference time in ms; defaults to now. Sets the fetch cutoff and closed-candle filter.
    #[arg(long)]
    as_of: Option<i64>,
    /// Override the report generation timestamp (RFC 3339).
    #[arg(long)]
    generated_at: Option<String>,
    /// Maximum missing candles allowed between rows; must be >= 0.
    #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
    max_gap_candles: i64,
}

fn parse_data_entries(entries: &[String]) -> Result<Vec<(String, PathBuf)>, String> {
    let mut parsed: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let (symbol, path) = entry
            .split_once('=')
            .ok_or_else(|| format!("invalid --data entry {entry:?}: expected SYMBOL=path"))?;
        let symbol = symbol.trim();
        let path = path.trim();
        if symbol.is_empty() {
            return Err(format!("invalid --data entry {entry:?}: empty symbol"));
        }
        if path.is_empty() {
            return Err(format!("invalid --data entry {entry:?}: empty path"));
        }
        if parsed
            .iter()
            .any(|(existing, _)| existing.as_str() == symbol)
        {
            continue;
        }
        parsed.push((symbol.to_string(), PathBuf::from(path)));
    }
    Ok(parsed)
}

fn resolve_symbols(
    explicit: &[String],
    data_symbols: &[String],
    env_value: Option<String>,
) -> Result<Vec<String>, String> {
    fn normalize(values: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() || out.iter().any(|existing| existing.as_str() == value) {
                continue;
            }
            out.push(value.to_string());
        }
        out
    }

    let explicit = normalize(explicit.iter().cloned());
    if !explicit.is_empty() {
        return Ok(explicit);
    }

    let data = normalize(data_symbols.iter().cloned());
    if !data.is_empty() {
        return Ok(data);
    }

    let env_symbols = normalize(env_value.unwrap_or_default().split(',').map(str::to_string));
    if !env_symbols.is_empty() {
        return Ok(env_symbols);
    }

    Err("no symbols resolved: pass --symbol, --data SYMBOL=path, or set BENCHMARK_TICKERS".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    share::logger::init_logger();
    let cli = Cli::parse();
    let strategy = strategy_from_env()?;
    let timeframe_name = cli
        .timeframe
        .clone()
        .or_else(|| env_string("KUCOIN_TIMEFRAME"))
        .unwrap_or_else(|| "15min".into());
    let timeframe = Timeframe::from_kucoin(&timeframe_name)
        .ok_or_else(|| format!("unsupported timeframe {timeframe_name:?}"))?;
    let init_fund = match cli.init_fund {
        Some(value) => value,
        None => match env_string("BENCHMARK_INIT_FUND") {
            Some(value) => value.parse::<i64>().map_err(|_| {
                format!("invalid BENCHMARK_INIT_FUND={value:?}: expected an integer")
            })?,
            None => 1000,
        },
    };
    if init_fund <= 0 {
        return Err(
            format!("--init-fund/BENCHMARK_INIT_FUND must be > 0 (got {init_fund})").into(),
        );
    }
    let as_of = cli.as_of.unwrap_or_else(identity::now_ms);
    if cli.max_gap_candles < 0 {
        return Err(format!(
            "--max-gap-candles must be >= 0 (got {})",
            cli.max_gap_candles
        )
        .into());
    }
    let base_url = env_string("KUCOIN_BASE_URL").unwrap_or_else(|| "https://api.kucoin.com".into());
    let max_age_candles = match env_string("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES") {
        Some(value) => parse_optional_usize_str("ANALYZER_MAX_DIVERGENCE_AGE_CANDLES", &value)?,
        None => None,
    };
    let settle_ms = match env_string("ANALYZER_CANDLE_SETTLE_MS") {
        Some(value) => value.parse::<i64>().map_err(|_| {
            format!("invalid ANALYZER_CANDLE_SETTLE_MS={value:?}: expected milliseconds")
        })?,
        None => 5_000,
    };

    let parsed_data = parse_data_entries(&cli.data)?;
    let data_symbols: Vec<String> = parsed_data
        .iter()
        .map(|(symbol, _)| symbol.clone())
        .collect();
    let symbols = resolve_symbols(&cli.symbols, &data_symbols, env_string("BENCHMARK_TICKERS"))?;
    let offline: BTreeMap<String, PathBuf> = parsed_data.into_iter().collect();

    let mut inputs = Vec::new();
    let mut failures = Vec::new();
    for symbol in symbols {
        let dataset: Result<Dataset, String> = if let Some(path) = offline.get(&symbol) {
            load_csv(
                &symbol,
                path,
                timeframe,
                as_of,
                settle_ms,
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
                settle_ms,
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
                signal_id: s.signal_id.clone(),
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

    if inputs.is_empty() {
        for failure in &failures {
            let symbol = failure
                .get("symbol")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let error = failure
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown error");
            eprintln!("failed symbol {symbol}: {error}");
        }
        return Err("no symbols replayed successfully".into());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_entry_without_equals_is_rejected() {
        let error = parse_data_entries(&["BTC-USDT".into()]).unwrap_err();
        assert!(error.contains("expected SYMBOL=path"));
    }

    #[test]
    fn data_entry_requires_symbol_and_path() {
        let error = parse_data_entries(&["=path.csv".into()]).unwrap_err();
        assert!(error.contains("empty symbol"));

        let error = parse_data_entries(&["BTC-USDT=".into()]).unwrap_err();
        assert!(error.contains("empty path"));

        let error = parse_data_entries(&["  = path.csv ".into()]).unwrap_err();
        assert!(error.contains("empty symbol"));
    }

    #[test]
    fn data_entries_are_trimmed_and_deduplicated() {
        let parsed = parse_data_entries(&[
            " BTC-USDT = first.csv ".into(),
            "ETH-USDT=eth.csv".into(),
            "BTC-USDT=second.csv".into(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            vec![
                ("BTC-USDT".to_string(), PathBuf::from("first.csv")),
                ("ETH-USDT".to_string(), PathBuf::from("eth.csv")),
            ]
        );
    }

    #[test]
    fn explicit_symbols_win_and_preserve_order() {
        let symbols = resolve_symbols(
            &[
                " BTC-USDT ".into(),
                "BTC-USDT".into(),
                "ETH-USDT".into(),
                " ".into(),
            ],
            &["XRP-USDT".into()],
            Some("SOL-USDT".into()),
        )
        .unwrap();

        assert_eq!(
            symbols,
            vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()]
        );
    }

    #[test]
    fn data_symbols_are_used_when_explicit_is_empty() {
        let symbols = resolve_symbols(
            &["".into()],
            &["ETH-USDT".into(), " ETH-USDT ".into(), "XRP-USDT".into()],
            Some("SOL-USDT".into()),
        )
        .unwrap();

        assert_eq!(
            symbols,
            vec!["ETH-USDT".to_string(), "XRP-USDT".to_string()]
        );
    }

    #[test]
    fn benchmark_env_is_the_last_fallback() {
        let symbols =
            resolve_symbols(&[], &[], Some(" BTC-USDT, ,ETH-USDT,BTC-USDT ".into())).unwrap();

        assert_eq!(
            symbols,
            vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()]
        );
    }

    #[test]
    fn empty_resolution_is_an_error() {
        assert!(resolve_symbols(&[], &[], None).is_err());
        assert!(resolve_symbols(&[" ".into()], &["".into()], Some(" , ".into())).is_err());
    }
}
