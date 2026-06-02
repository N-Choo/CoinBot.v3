use ethers::abi::{decode, ParamType};
use ethers::{providers::Middleware, types::Transaction as EthTx};

pub struct Rpc;

impl Rpc {
    pub async fn get_transaction(tx_hash: &str) -> Result<EthTx, String> {
        if tx_hash.trim().is_empty() {
            return Err("Transaction hash is empty".to_string());
        }

        let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(
            "https://ethereum-rpc.publicnode.com",
        )
        .map_err(|e| format!("Failed to create provider: {}", e))?;

        let hash: ethers::types::TxHash = tx_hash
            .parse()
            .map_err(|e| format!("Invalid tx hash format: {}", e))?;

        provider
            .get_transaction(hash)
            .await
            .map_err(|e| format!("RPC error: {}", e))?
            .ok_or_else(|| "Transaction not found".to_string())
    }
}

pub fn erc20_transfer_amount(input: &[u8]) -> String {
    const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

    if input.len() < 4 || input[..4] != TRANSFER_SELECTOR {
        return "0".into();
    }

    match decode(&[ParamType::Address, ParamType::Uint(256)], &input[4..]) {
        Ok(tokens) => tokens[1]
            .clone()
            .into_uint()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0".into()),
        Err(_) => "0".into(),
    }
}
