use std::sync::LazyLock;

use ethers::types::Address;

pub const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
pub static USDT_CONTRACT: LazyLock<Address> = LazyLock::new(|| USDT_ADDRESS.parse().unwrap());

pub const PLATFORM_ADD: &str = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c";
pub static PLATFORM_WALLET: LazyLock<Address> = LazyLock::new(|| PLATFORM_ADD.parse().unwrap());

use actix_web::{HttpResponse, Responder};

pub async fn get_config() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "platform_wallet": PLATFORM_ADD,
    }))
}
