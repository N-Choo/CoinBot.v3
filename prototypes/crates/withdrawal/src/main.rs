use common::deposit_service_client::DepositServiceClient;
use common::BalanceRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DepositServiceClient::connect("http://[::1]:50051").await?;

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
