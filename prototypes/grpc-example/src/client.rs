use tonic::transport::Endpoint;

pub mod wallet {
    tonic::include_proto!("wallet");
}

use wallet::wallet_client::WalletClient;
use wallet::BalanceRequest;

// ── WithdrawalService (client) calls DepositService (server) ──
// The withdrawal process needs to verify balance before issuing a withdrawal.
// It queries the deposit service via gRPC instead of sharing a DB.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = Endpoint::from_static("http://[::1]:50051");
    let mut client = WalletClient::connect(endpoint).await?;

    // Check if user has enough balance before processing withdrawal
    let resp = client
        .get_balance(BalanceRequest {
            user_id: "user_abc".into(),
        })
        .await?;

    let balance = resp.into_inner();
    println!(
        "User balance: {} {}",
        balance.balance, balance.currency
    );

    Ok(())
}
