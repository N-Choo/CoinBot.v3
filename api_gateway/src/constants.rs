use std::sync::LazyLock;

use ethers::types::Address;

pub const TOKENS: &[(&str, &str)] = &[
    ("0xdac17f958d2ee523a2206206994597c13d831ec7", "USDT"),
    ("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", "USDC"),
];

pub const PLATFORM_ADD: &str = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c";
pub static PLATFORM_WALLET: LazyLock<Address> = LazyLock::new(|| PLATFORM_ADD.parse().unwrap());

use actix_web::{HttpResponse, Responder};

pub async fn get_config() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "platform_wallet": PLATFORM_ADD,
        "tokens": TOKENS,
    }))
}
