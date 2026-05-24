use tonic::{transport::Server, Request, Response, Status};

pub mod wallet {
    tonic::include_proto!("wallet");
}

use wallet::wallet_server::{Wallet, WalletServer};
use wallet::{BalanceRequest, BalanceResponse, TransferRequest, TransferResponse};

// ── Server: Deposit/Balance Service ──
// This process listens for gRPC requests from other services.

#[derive(Default)]
pub struct DepositService;

#[tonic::async_trait]
impl Wallet for DepositService {
    async fn get_balance(
        &self,
        _req: Request<BalanceRequest>,
    ) -> Result<Response<BalanceResponse>, Status> {
        // Reads from deposit DB (isolated from withdrawal data)
        Ok(Response::new(BalanceResponse {
            balance: "1500.00".into(),
            currency: "USDT".into(),
        }))
    }

    async fn transfer(
        &self,
        req: Request<TransferRequest>,
    ) -> Result<Response<TransferResponse>, Status> {
        let _r = req.into_inner();
        Ok(Response::new(TransferResponse {
            success: true,
            tx_id: "dep-tx-001".into(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    println!("Deposit service listening on {}", addr);

    Server::builder()
        .add_service(WalletServer::new(DepositService::default()))
        .serve(addr)
        .await?;

    Ok(())
}
