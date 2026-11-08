use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use share::ProcessError;
use signal_gateway::agent_client::OpenCodeClient;
use signal_gateway::config::Config;
use signal_gateway::price::KuCoinPrice;
use signal_gateway::publisher::GrpcPublisher;
use signal_gateway::runner::{Gateway, RetryPolicy};
use signal_gateway::state::State;
use signal_gateway::symbols;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let config = Config::from_env()?;
    let state = State::load(&config.state_path, config.reset_state)?;

    let agent = Arc::new(OpenCodeClient::new(
        config.opencode_base_url.as_str(),
        config.opencode_username.as_str(),
        config.opencode_password.clone(),
        config.opencode_agent.as_str(),
        config.opencode_model.clone(),
        config.delete_sessions,
        config.opencode_timeout,
    )?);
    let publisher = Arc::new(GrpcPublisher::new(
        config.grpc_endpoint.as_str(),
        config.grpc_timeout,
    )?);
    let price = Arc::new(KuCoinPrice::new(
        config.kucoin_base_url.as_str(),
        config.grpc_timeout,
    )?);

    let mut gateway = Gateway::new(
        config.clone(),
        state,
        agent,
        publisher,
        price,
        RetryPolicy::default(),
    );

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            match signal(SignalKind::terminate()) {
                Ok(mut term) => {
                    tokio::select! {
                        result = tokio::signal::ctrl_c() => {
                            if let Err(e) = result {
                                log::error!("ctrl_c handler: {e}");
                            }
                        }
                        _ = term.recv() => {}
                    }
                }
                Err(e) => {
                    log::error!("SIGTERM handler: {e}");
                    if let Err(e) = tokio::signal::ctrl_c().await {
                        log::error!("ctrl_c handler: {e}");
                    }
                }
            }
        }
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                log::error!("ctrl_c handler: {e}");
            }
        }
        let _ = shutdown_tx.send(true);
    });

    if config.run_on_start {
        let candle = signal_gateway::scheduler::most_recent_closed_candle_open(Utc::now());
        run_once(&mut gateway, &config, candle).await;
    }

    loop {
        if *shutdown_rx.borrow() {
            log::info!("shutting down");
            return Ok(());
        }

        let now = Utc::now();
        let next = signal_gateway::scheduler::next_close_after(now, config.cycle_delay);
        let wait = (next - now)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(1));
        log::info!("next cycle at {}", next.to_rfc3339());

        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = shutdown_rx.changed() => {
                log::info!("shutting down");
                return Ok(());
            }
        }

        if *shutdown_rx.borrow() {
            log::info!("shutting down");
            return Ok(());
        }

        let candle = signal_gateway::scheduler::most_recent_closed_candle_open(Utc::now());
        run_once(&mut gateway, &config, candle).await;
    }
}

async fn run_once(gateway: &mut Gateway, config: &Config, candle: chrono::DateTime<Utc>) {
    match symbols::fetch_active_symbols_with_retry(
        config.grpc_endpoint.as_str(),
        config.grpc_timeout,
        3,
    )
    .await
    {
        Ok(symbols) => {
            if let Err(e) = gateway.run_cycle(&symbols, candle).await {
                log::error!("cycle failed: {e}");
            }
        }
        Err(e) => log::error!("GetActiveSymbols failed: {e}; skipping cycle"),
    }
}
