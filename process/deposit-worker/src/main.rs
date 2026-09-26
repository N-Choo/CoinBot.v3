use std::sync::Arc;

use kucoin::client::rest::{Credentials, KuCoinClient};
use share::{deposit_service_server::DepositServiceServer, ProcessConfig, ProcessError};
use tokio::spawn;
use tokio::sync::{mpsc, Semaphore};
use tonic::transport::Server;
use tonic_health::server::health_reporter;

use deposit::deposit_sweeper::run_deposit_sweeper;
use deposit::grpc_handler::DepositServer;
use deposit::task::{run_dispatcher, DepositTask};

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let cfg = ProcessConfig::new("deposit");

    let database_url = cfg.get("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;

    let api_key = cfg.get("KC_KEY")?;
    let api_secret = cfg.get("KC_SECRET")?;
    let api_passphrase = cfg.get("KC_PASSPHRASE")?;
    let kcc = KuCoinClient::new(Credentials::new(&api_key, &api_secret, &api_passphrase));

    let (tx, rx) = mpsc::channel::<DepositTask>(256);
    let max_concurrent = cfg.get_or("N_WORKERS", 2usize);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    run_dispatcher(pool.clone(), rx, semaphore.clone());
    spawn(run_deposit_sweeper(pool.clone(), kcc));

    let addr = cfg.get("GRPC_ADDR")?;
    let (mut health_reporter, health_service) = health_reporter();
    health_reporter
        .set_serving::<DepositServiceServer<DepositServer>>()
        .await;

    let service = DepositServiceServer::new(DepositServer { tx });

    log::info!("Deposit service on {}", addr);
    Server::builder()
        .add_service(health_service)
        .add_service(service)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
