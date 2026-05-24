use tonic::{transport::Server, Request, Response, Status};

use common::deposit_service_server::{DepositService, DepositServiceServer};
use common::{BalanceRequest, BalanceResponse, HistoryRequest, HistoryResponse, TxEntry};

#[derive(Default)]
pub struct DepositServer;

#[tonic::async_trait]
impl DepositService for DepositServer {
    async fn get_balance(
        &self,
        req: Request<BalanceRequest>,
    ) -> Result<Response<BalanceResponse>, Status> {
        let _user = req.into_inner().user_id;
        // TODO: query deposit DB
        Ok(Response::new(BalanceResponse {
            balance: "1500.00".into(),
            currency: "USDT".into(),
        }))
    }

    async fn get_history(
        &self,
        req: Request<HistoryRequest>,
    ) -> Result<Response<HistoryResponse>, Status> {
        let _req = req.into_inner();
        Ok(Response::new(HistoryResponse { entries: vec![] }))
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
