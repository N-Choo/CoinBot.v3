#[cfg(test)]
mod tests {
    use crate::handlers::transaction::Transaction;
    use actix_web::{App, http::StatusCode, test, web};

    #[actix_web::test]
    async fn test_deposit_rejects_empty_tx_hash() {
        let app = test::init_service(App::new().route(
            "/api/transactions/deposit",
            web::post().to(Transaction::deposit),
        ))
        .await;

        let req = test::TestRequest::post()
            .uri("/api/transactions/deposit")
            .set_json(&serde_json::json!({ "tx_hash": "" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
