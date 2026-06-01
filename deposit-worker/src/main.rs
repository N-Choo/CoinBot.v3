use tonic::{transport::Server, Request, Response, Status};

use common::deposit_service_server::{DepositService, DepositServiceServer};
use common::{TicketRequest, TicketResponse};

#[derive(Default)]
pub struct DepositServer;

#[tonic::async_trait]
impl DepositService for DepositServer {
    async fn create_ticket0(
        &self,
        _req: Request<TicketRequest>,
    ) -> Result<Response<TicketResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    println!("Deposit service on {}", addr);

    Server::builder()
        .add_service(DepositServiceServer::new(DepositServer::default()))
        .serve(addr)
        .await?;

    Ok(())
}
