//! `/api/stats` tests. Skipped when `DATABASE_URL` is unset.

use actix_web::{App, http::StatusCode, test, web};
use sqlx::PgPool;

use crate::routes::api_routes;

/// A lazy pool for `DATABASE_URL`, or `None` when it is not configured.
fn real_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect_lazy(&url).ok()
}

/// The endpoint returns `200` with the four aggregate fields.
#[actix_web::test]
async fn test_stats_ok() {
    let Some(pool) = real_pool() else { return };
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .configure(api_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/stats").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["accounts"].is_number());
    assert!(body["active_contracts"].is_number());
    assert!(body["trades_executed"].is_number());
    assert!(body["funds_deployed"].is_string());
    assert!(
        body["funds_deployed"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .is_ok(),
        "funds_deployed must be a decimal string: {body:?}"
    );
}
