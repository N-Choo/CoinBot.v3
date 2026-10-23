#![cfg(feature = "live")]

use analyzer::driver::CandleDriver;
use analyzer::live::config::{LiveConfig, LiveConfigError};
use analyzer::live::sink::Sink;
use analyzer::live::symbol_task::{process_batch, run_symbol_once};
use analyzer::strategy::config::Config;
use analyzer::strategy::types::{Candle, Signal};
use rust_decimal::Decimal;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;

fn synthetic_candles() -> Vec<Candle> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/synthetic_1day.csv");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split(',').collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|value| value.trim() == name)
            .unwrap_or_else(|| panic!("missing column {name}"))
    };
    let open_time = column("open_time_ms");
    let open = column("open");
    let high = column("high");
    let low = column("low");
    let close = column("close");
    let volume = column("volume");

    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            let decimal = |index: usize| Decimal::from_str(fields[index]).unwrap();
            Candle {
                open_time_ms: fields[open_time].parse().unwrap(),
                open: decimal(open),
                high: decimal(high),
                low: decimal(low),
                close: decimal(close),
                volume: decimal(volume),
            }
        })
        .collect()
}

fn new_config(
    symbols: Vec<&str>,
    window: usize,
    settle_ms: i64,
    poll_ms: u64,
    activation_poll_ms: u64,
) -> Result<LiveConfig, LiveConfigError> {
    LiveConfig::new(
        symbols.into_iter().map(str::to_string).collect(),
        "15min",
        window,
        settle_ms,
        poll_ms,
        activation_poll_ms,
        "http://127.0.0.1:50051".into(),
        "https://api.kucoin.com".into(),
    )
}

fn collected(sink: &Sink) -> Vec<Signal> {
    match sink {
        Sink::Collect(buffer) => buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
        _ => panic!("expected a collect sink"),
    }
}

#[test]
fn live_config_reads_symbols_and_defaults() {
    let cfg = new_config(vec!["BTC-USDT", "ETH-USDT"], 100, 5_000, 1_000, 30_000).unwrap();
    assert_eq!(cfg.symbols.len(), 2);
    assert_eq!(cfg.timeframe.as_kucoin(), "15min");
    assert_eq!(cfg.window, 100);
    assert_eq!(cfg.settle_ms, 5_000);
    assert_eq!(cfg.poll_ms, 1_000);
    assert_eq!(cfg.activation_poll_ms, 30_000);
    assert_eq!(cfg.endpoint, "http://127.0.0.1:50051");
    assert_eq!(cfg.base_url, "https://api.kucoin.com");
}

#[test]
fn live_config_normalizes_and_dedupes_symbols() {
    let cfg = new_config(
        vec![" BTC/USDT ", "BTC-USDT", "ETH/USDT", "BTC/USDT", ""],
        100,
        5_000,
        1_000,
        30_000,
    )
    .unwrap();
    assert_eq!(
        cfg.symbols,
        vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()]
    );
}

#[test]
fn live_config_rejects_zero_poll_interval() {
    let error = new_config(vec!["BTC/USDT"], 100, 5_000, 0, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::PollIntervalZero));
}

#[test]
fn live_config_rejects_zero_activation_poll_interval() {
    let error = new_config(vec!["BTC/USDT"], 100, 5_000, 1_000, 0).unwrap_err();
    assert!(matches!(error, LiveConfigError::ActivationPollIntervalZero));
}

#[test]
fn live_config_rejects_negative_settle() {
    let error = new_config(vec!["BTC/USDT"], 100, -1, 1_000, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::NegativeSettle));
}

#[test]
fn live_config_rejects_a_window_below_eight() {
    let error = new_config(vec!["BTC/USDT"], 7, 5_000, 1_000, 30_000).unwrap_err();
    assert!(matches!(error, LiveConfigError::WindowTooSmall));
}

#[test]
fn live_config_rejects_a_window_below_the_driver_window() {
    let strategy = Config::default();
    let window = strategy.window - 1;
    let cfg = new_config(vec!["BTC/USDT"], window, 5_000, 1_000, 30_000).unwrap();

    assert!(matches!(
        cfg.validate_strategy(&strategy),
        Err(LiveConfigError::WindowBelowStrategy { window: got, need })
            if got == window && need == strategy.window
    ));
}

#[test]
fn live_config_accepts_a_window_at_the_driver_window() {
    let strategy = Config::default();
    let cfg = new_config(vec!["BTC/USDT"], strategy.window, 5_000, 1_000, 30_000).unwrap();
    assert!(cfg.validate_strategy(&strategy).is_ok());
}

#[test]
fn process_batch_emits_signals_through_the_sink() {
    let candles = synthetic_candles();
    let strategy = Config::default();
    let mut driver = CandleDriver::new("SYNTH-USDT", strategy.clone());
    let sink = Sink::Collect(Default::default());

    let emitted = process_batch(&mut driver, &candles, &sink).unwrap();
    assert_eq!(emitted, collected(&sink).len());
    assert!(emitted > 0);
}

/// Closed 15-minute candles ending well in the past.
fn closed_candles(count: usize) -> Vec<Candle> {
    let interval = 900_000i64;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let last_open = (now_ms / interval - 2) * interval;
    (0..count)
        .map(|i| Candle {
            open_time_ms: last_open - (count as i64 - 1 - i as i64) * interval,
            open: Decimal::from(100),
            high: Decimal::from(101),
            low: Decimal::from(99),
            close: Decimal::from(100),
            volume: Decimal::from(1),
        })
        .collect()
}

/// Serve one kline response from a throwaway local HTTP server.
fn kline_server(candles: &[Candle]) -> (String, thread::JoinHandle<()>) {
    let rows: Vec<String> = candles
        .iter()
        .map(|c| {
            format!(
                r#"["{}","{}","{}","{}","{}","{}"]"#,
                c.open_time_ms / 1_000,
                c.open,
                c.close,
                c.high,
                c.low,
                c.volume
            )
        })
        .collect();
    let body = format!(r#"{{"code":"200000","data":[{}]}}"#, rows.join(","));

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while let Ok(read) = stream.read(&mut chunk) {
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn run_symbol_once_uses_the_strategy_window_when_the_live_window_is_larger() {
    let strategy = Config::default();
    let mut cfg = new_config(vec!["BTC/USDT"], strategy.window + 10, 0, 1_000, 30_000).unwrap();
    let (base_url, server) = kline_server(&closed_candles(strategy.window + 1));
    cfg.base_url = base_url;
    let sink = Sink::Collect(Default::default());

    let (window, _emitted) = run_symbol_once(&cfg, &strategy, "BTC-USDT", &sink)
        .await
        .unwrap();
    assert_eq!(window, strategy.window);
    server.join().unwrap();
}
