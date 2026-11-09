use share::ProcessError;
use tokio::signal;

mod app_config;
mod app_state;
mod grpc_handler;
mod kucoin_margin;
mod ledger;
mod position_exit;
mod reconciler;
mod settlement;
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

    let db = app_state.db_pool;
    let kcc = app_state.kucoin.clone();

    // gPRC server: `EngineService`.
    let engine_server = grpc_handler::EngineServer::new(db.clone(), kcc.clone());
    let grpc_addr = config.grpc_addr.clone();
    tokio::spawn(grpc_handler::serve(engine_server, grpc_addr));

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

    // Accrue borrow interest, repay residual debt, and settle cash-outs.
    let interest_db = db.clone();
    let interest_kc = kcc.clone();
    tokio::spawn(settlement::interest::run(interest_db, interest_kc));

    let repay_db = db.clone();
    let repay_kc = kcc.clone();
    tokio::spawn(settlement::repay::run(repay_db, repay_kc));

    let settle_db = db.clone();
    let settle_kc = kcc.clone();
    tokio::spawn(settlement::contract::run(settle_db, settle_kc));

    // Wait for Ctrl+C before shutting down
    signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    log::info!("shutting down...");

    Ok(())
}
