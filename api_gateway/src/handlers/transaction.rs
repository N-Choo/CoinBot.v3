use actix_web::{HttpResponse, Responder, web};
pub struct Transaction {}

#[derive(serde::Deserialize)]
pub struct DepositPayloadRequest {
    pub tx_hash: String,
}

impl Transaction {
    pub async fn deposit(
        _header: actix_web::HttpRequest,
        payload: web::Json<DepositPayloadRequest>,
    ) -> impl Responder {
        log::info!("tx_hash: {}", payload.tx_hash);
        HttpResponse::Ok().finish()
    }
}
