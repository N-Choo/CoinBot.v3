use share::ProcessError;
use share::db::contracts::{ContractFilter, Status};
use share::models::signals::TradeSignal;

mod app_config;
mod app_state;
mod trader;

use app_config::AppConfig;
use app_state::AppState;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let config = AppConfig::from_env()?;
    let app_state = AppState::new(&config).await?;
    let app_state_copy = app_state.clone();

    let _db = app_state.db_pool;
    let cache = app_state.redis_cache.clone();

    // loop signal generator
    tokio::spawn(async move {
        if let Err(e) = signal_generator(&app_state_copy).await {
            log::error!("ignite analyze err: {e}");
        }
    });

    // Open listen channel to recieves `TradeSignal`
    let signal_cache = cache.clone();
    tokio::spawn(async move {
        if let Err(e) = signal_cache
            .subscribe("signals:result", |payload| async move {
                match serde_json::from_str::<Vec<TradeSignal>>(&payload) {
                    Ok(signals) => {
                        for _signal in &signals {
                            todo!();
                            // consider_action(signal).await;
                        }
                    }
                    Err(e) => {
                        log::error!("failed to parse signal: {e}");
                    }
                }
            })
            .await
        {
            log::error!("signal subscriber error: {e}");
        }
    });

    // Wait for Ctrl+C before shutting down
    signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    log::info!("shutting down...");

    Ok(())
}

async fn signal_generator(app_state: &AppState) -> Result<(), ProcessError> {
    let pool = &app_state.db_pool;
    let cache = &app_state.redis_cache;

    loop {
        let tickers = ContractFilter::new()
            .with_status(Status::Active)
            .execute_tickers(pool)
            .await?;

        let msg = serde_json::to_string(&tickers)?;
        cache.publish("tickers:analyze", &msg).await;
        tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
    }
}
