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

    if config.run_on_start {
        let candle = signal_gateway::scheduler::most_recent_closed_candle_open(Utc::now());
        run_once(&mut gateway, &config, candle).await;
    }

    loop {
        let now = Utc::now();
        let next = signal_gateway::scheduler::next_close_after(now, config.cycle_delay);
        let wait = (next - now)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(1));
        log::info!("next cycle at {}", next.to_rfc3339());

        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = tokio::signal::ctrl_c() => {
                log::info!("shutting down");
                return Ok(());
            }
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
