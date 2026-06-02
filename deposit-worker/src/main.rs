use tokio::sync::mpsc;
use tonic::transport::Server;
use tonic_health::server::health_reporter;

use common::deposit_service_server::DepositServiceServer;
use deposit::grpc_handler::DepositServer;
use deposit::task::{run_dispatcher, DepositTask};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let addr = std::env::var("GRPC_ADDR").unwrap_or_else(|_| "[::1]:50051".into());
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;

    let (tx, rx) = mpsc::channel::<DepositTask>(256);
    let max_concurrent = std::env::var("DEPOSIT_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));

    run_dispatcher(pool.clone(), rx, semaphore);

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
