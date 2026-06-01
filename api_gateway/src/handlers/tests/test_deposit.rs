#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use actix_web::{App, http::StatusCode, test, web};
    use moka::future::Cache;
    use sqlx::PgPool;

    use crate::handlers::transaction::Transaction;

    static POOL: LazyLock<PgPool> = LazyLock::new(|| {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/test".into());
        PgPool::connect_lazy(&url).expect("Failed to create test pool")
    });

    fn mock_cache() -> Cache<String, String> {
        Cache::builder().max_capacity(10).build()
    }

    #[actix_web::test]
    async fn test_deposit_rejects_empty_tx_hash() {
        let cache = mock_cache();
        let token = uuid::Uuid::new_v4().to_string();
        cache.insert(token.clone(), "0xwallet".into()).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(cache))
                .app_data(web::Data::new(POOL.clone()))
                .route(
                    "/api/transactions/deposit",
                    web::post().to(Transaction::deposit),
                ),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/transactions/deposit")
            .cookie(actix_web::cookie::Cookie::new("session_token", token))
            .set_json(serde_json::json!({ "tx_hash":""}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
