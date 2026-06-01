#[derive(serde::Deserialize)]
pub struct DepositPayloadRequest {
    pub tx_hash: String,
}
