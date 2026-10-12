use actix_web::{HttpResponse, Responder};

use crate::constants::PLATFORM_ADD;

/// `GET /api/config` — expose public client configuration.
///
/// Returns the platform deposit wallet address so the frontend can construct
/// USDT transfers.
pub async fn get_config() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "platform_wallet": PLATFORM_ADD,
    }))
}
