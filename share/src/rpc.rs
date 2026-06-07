use std::sync::OnceLock;

use ethers::types::U64;
use ethers::{providers::Middleware, types::Transaction as EthTx};

type Provider = ethers::providers::Provider<ethers::providers::Http>;

fn provider() -> &'static Provider {
    static PROVIDER: OnceLock<Provider> = OnceLock::new();
    PROVIDER.get_or_init(|| {
        Provider::try_from("https://ethereum-rpc.publicnode.com")
            .expect("Failed to create Ethereum RPC provider")
    })
}

pub struct Rpc;

impl Rpc {
    pub async fn get_transaction(tx_hash: &str) -> Result<EthTx, String> {
        if tx_hash.trim().is_empty() {
            return Err("Transaction hash is empty".to_string());
        }

        let hash: ethers::types::TxHash = tx_hash
            .parse()
            .map_err(|e| format!("Invalid tx hash format: {}", e))?;

        provider()
            .get_transaction(hash)
            .await
            .map_err(|e| format!("RPC error: {}", e))?
            .ok_or_else(|| "Transaction not found".to_string())
    }

    pub async fn confirm_onchain_success(tx_hash: &str) -> Result<(), String> {
        let hash: ethers::types::TxHash = tx_hash
            .parse()
            .map_err(|e| format!("Invalid tx hash format: {}", e))?;

        let receipt = provider()
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| format!("RPC error fetching receipt: {}", e))?
            .ok_or_else(|| "Transaction receipt not found — tx may be pending".to_string())?;

        match receipt.status {
            Some(status) if status == U64::from(1) => Ok(()),
            Some(status) if status == U64::from(0) => {
                Err("Transaction reverted on-chain (status=0)".to_string())
            }
            Some(_) => Err("Transaction status unknown".to_string()),
            None => Err("Transaction status unknown (pre-Byzantium)".to_string()),
        }
    }
}
