use share::ProcessError;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();
    let config = signal_gateway::config::Config::from_env()?;
    log::info!(
        "signal-gateway starting: agent={} engine={}",
        config.opencode_agent,
        config.grpc_endpoint
    );
    Ok(())
}
