use std::sync::LazyLock;

use ethers::types::Address;

/// Mainnet USDT (ERC-20) contract address.
pub const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
/// Parsed [`USDT_ADDRESS`], compared against a transaction's `to` field.
pub static USDT_CONTRACT: LazyLock<Address> = LazyLock::new(|| USDT_ADDRESS.parse().unwrap());

/// Platform-controlled wallet that receives user deposits.
pub const PLATFORM_ADD: &str = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c";
/// Parsed [`PLATFORM_ADD`], compared against an ERC-20 transfer recipient.
pub static PLATFORM_WALLET: LazyLock<Address> = LazyLock::new(|| PLATFORM_ADD.parse().unwrap());

use actix_web::{HttpResponse, Responder};

/// `GET /api/config` — expose public client configuration.
///
/// Returns the platform deposit wallet address so the frontend can construct
/// USDT transfers.
pub async fn get_config() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "platform_wallet": PLATFORM_ADD,
    }))
}
