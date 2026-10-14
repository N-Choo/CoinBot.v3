use share::ProcessError;
use share::models::signals::TradeSignal;
use tokio::signal;

mod app_config;
mod app_state;
mod position_exit;
mod reconciler;
mod signals;
mod symbol;
mod trader;

use app_config::AppConfig;
use app_state::AppState;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let config = AppConfig::from_env()?;
    let app_state = AppState::new(&config).await?;
    let app_state_copy = app_state.clone();

    let db = app_state.db_pool;
    let cache = app_state.redis_cache.clone();
    let kcc = app_state.kucoin.clone();

    // Sync contract status
    let sweeper_db = db.clone();
    let kc_client = kcc.clone();
    tokio::spawn(async move {
        loop {
            reconciler::run(sweeper_db.clone(), kc_client.clone()).await;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    // Exit open positions once TP/SL is reached.
    let order_db = db.clone();
    let order_kc = kcc.clone();
    tokio::spawn(async move {
        position_exit::run(order_db, order_kc).await;
    });

    // loop signal generator
    tokio::spawn(async move {
        if let Err(e) = signals::signal_generator(&app_state_copy).await {
            log::error!("ignite analyze err: {e}");
        }
    });

    // Open listen channel to recieves `TradeSignal`
    let signal_cache = cache.clone();
    tokio::spawn(async move {
        if let Err(e) = signal_cache
            .subscribe("signals:result", move |payload| {
                let db = db.clone();
                let kcc = kcc.clone();
                async move {
                    match serde_json::from_str::<Vec<TradeSignal>>(&payload) {
                        Ok(signal_list) => {
                            for signal in &signal_list {
                                if let Err(e) =
                                    signals::signal_handler(signal, db.clone(), kcc.clone()).await
                                {
                                    log::error!("signal handler failed: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("failed to parse signal: {e}");
                        }
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
