use tonic::{transport::Server, Request, Response, Status};
use tonic_health::server::health_reporter;

use common::deposit_service_server::{DepositService, DepositServiceServer};
use common::{TicketRequest, TicketResponse};
use share::db::deposit::Deposit;
use share::rpc::Rpc;

pub struct DepositServer {
    pool: sqlx::PgPool,
}

#[tonic::async_trait]
impl DepositService for DepositServer {
    async fn create_ticket0(
        &self,
        req: Request<TicketRequest>,
    ) -> Result<Response<TicketResponse>, Status> {
        let req = req.into_inner();
        log::info!("create_ticket0: tx={} ticker={}", req.tx_hash, req.ticker);

        let tx = Rpc::get_transaction(&req.tx_hash)
            .await
            .map_err(|e| {
                log::warn!("get_transaction failed: {e}");
                Status::invalid_argument(format!("invalid tx: {e}"))
            })?;

        let user_uid = req
            .uid
            .parse()
            .map_err(|_| Status::invalid_argument("invalid uid"))?;

        let deposit = Deposit::create_pending(
            &self.pool,
            user_uid,
            &req.tx_hash,
            &tx,
            &req.ticker,
        )
        .await
        .map_err(|e| {
            log::error!("create_pending failed: {e}");
            Status::internal(e.to_string())
        })?;

        log::info!("ticket created: {}", deposit.id);
        Ok(Response::new(TicketResponse {
            ticket_id: deposit.id.to_string(),
            timestamp: deposit.created_at.to_rfc3339(),
            accepted: true,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let addr = std::env::var("GRPC_ADDR").unwrap_or_else(|_| "[::1]:50051".into());
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;

    let (mut health_reporter, health_service) = health_reporter();
    health_reporter
        .set_serving::<DepositServiceServer<DepositServer>>()
        .await;

    let service = DepositServiceServer::new(DepositServer { pool });

    log::info!("Deposit service on {}", addr);
    Server::builder()
        .add_service(health_service)
        .add_service(service)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
