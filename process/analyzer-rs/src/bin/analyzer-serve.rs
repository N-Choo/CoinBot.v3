use analyzer_rs::live::config::LiveConfig;
use analyzer_rs::live::grpc::{config_version, run_publisher, signal_ttl_ms, EngineClient};
use analyzer_rs::live::sink::Sink;
use analyzer_rs::live::supervisor::run_supervisor;
use analyzer_rs::live::symbol_task::run_symbol_once;
use analyzer_rs::strategy::config::Config;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cfg = LiveConfig::from_env()?;
    let strategy = Config::default();
    let mut client = EngineClient::connect(&cfg.endpoint)?;

    if std::env::var("ANALYZER_ONCE").as_deref() == Ok("1") {
        let symbols = match client.active_symbols().await {
            Ok(symbols) if !symbols.is_empty() => symbols,
            _ => cfg.symbols.clone(),
        };
        let sink = Sink::Log;
        for symbol in symbols {
            let symbol = symbol.replace('/', "-");
            match run_symbol_once(&cfg, &strategy, &symbol, &sink) {
                Ok((window, emitted)) => {
                    println!("once symbol={symbol} window={window} emitted={emitted}")
                }
                Err(error) => eprintln!("once failed symbol={symbol}: {error}"),
            }
        }
        return Ok(());
    }

    let (tx, rx) = mpsc::channel(256);
    let sink = Sink::Channel(tx);
    let publisher = tokio::spawn(run_publisher(
        client.clone(),
        rx,
        signal_ttl_ms(cfg.timeframe),
        config_version(&strategy),
        cfg.timeframe,
    ));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let supervisor = tokio::spawn(run_supervisor(cfg, strategy, sink, client, shutdown_rx));

    tokio::signal::ctrl_c().await?;
    let _ = shutdown_tx.send(true);
    let _ = supervisor.await;
    publisher.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(())
}
