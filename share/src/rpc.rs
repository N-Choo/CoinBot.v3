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

    match decode_erc20_transfer(&input[4..]) {
        Some((_, amount)) => amount.to_string(),
        None => "0".into(),
    }
}

pub fn erc20_transfer_recipient(input: &[u8]) -> Option<String> {
    const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

    if input.len() < 4 || input[..4] != TRANSFER_SELECTOR {
        return None;
    }

    decode_erc20_transfer(&input[4..])
        .map(|(recipient, _)| format!("0x{:x}", recipient))
}

fn decode_erc20_transfer(data: &[u8]) -> Option<(ethers::types::Address, ethers::types::U256)> {
    match decode(&[ParamType::Address, ParamType::Uint(256)], data) {
        Ok(tokens) => {
            let recipient = tokens[0].clone().into_address()?;
            let amount = tokens[1].clone().into_uint()?;
            Some((recipient, amount))
        }
        Err(_) => None,
    }
}
