use analyzer::live::config::{strategy_from_env, LiveConfig};
use analyzer::live::grpc::{config_version, run_publisher, signal_ttl_ms, EngineClient};
use analyzer::live::sink::Sink;
use analyzer::live::supervisor::run_supervisor;
use analyzer::live::symbol_task::run_symbol_once;
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const PUBLISHER_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(unix)]
async fn shutdown_signal() -> std::io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    analyzer::live::config::load_dotenv();
    share::logger::init_logger();
    let cfg = LiveConfig::from_env()?;
    let strategy = strategy_from_env()?;
    cfg.validate_strategy(&strategy)?;
    strategy.validate()?;
    let mut client = EngineClient::connect(&cfg.endpoint)?;

    if std::env::var("ANALYZER_ONCE").as_deref() == Ok("1") {
        let symbols = match client.active_symbols().await {
            Ok(symbols) if !symbols.is_empty() => symbols,
            Ok(_) => cfg.symbols.clone(),
            Err(error) => {
                log::warn!("activation poll failed: {error}; using LIVE_SYMBOLS");
                cfg.symbols.clone()
            }
        };
        let symbols: BTreeSet<String> = symbols
            .into_iter()
            .map(|symbol| symbol.trim().replace('/', "-"))
            .filter(|symbol| !symbol.is_empty())
            .collect();
        let sink = Sink::Log;
        for symbol in symbols {
            match run_symbol_once(&cfg, &strategy, &symbol, &sink).await {
                Ok((window, emitted)) => {
                    log::info!("once symbol={symbol} window={window} emitted={emitted}")
                }
                Err(error) => log::error!("once failed symbol={symbol}: {error}"),
            }
        }
        return Ok(());
    }

    let (tx, rx) = mpsc::channel(256);
    let sink = Sink::Channel(tx);
    let mut publisher = tokio::spawn(run_publisher(
        client.clone(),
        rx,
        signal_ttl_ms(cfg.timeframe),
        config_version(&strategy),
        cfg.timeframe,
    ));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let supervisor = tokio::spawn(run_supervisor(cfg, strategy, sink, client, shutdown_rx));

    shutdown_signal().await?;
    let _ = shutdown_tx.send(true);
    let _ = supervisor.await;

    // The supervisor owns the original sink and every worker clone, so the channel is closed
    // once it returns. Let the publisher drain the queued signals before aborting it.
    match tokio::time::timeout(PUBLISHER_DRAIN_TIMEOUT, &mut publisher).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::error!("publisher join failed: {error}"),
        Err(_) => {
            log::warn!("publisher did not drain within {PUBLISHER_DRAIN_TIMEOUT:?}; aborting");
            publisher.abort();
            let _ = publisher.await;
        }
    }
    Ok(())
}
