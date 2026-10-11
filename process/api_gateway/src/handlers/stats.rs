use actix_web::{HttpResponse, Responder, web};
use sqlx::PgPool;

/// Public platform metrics handler.
pub struct Stats;

impl Stats {
    /// `GET /api/stats` — aggregate counts and deployed funds.
    ///
    /// Responses:
    /// - `200 OK` — JSON [`share::db::stats::SystemStats`].
    /// - `500 Internal Server Error` — query failed.
    pub async fn get(pool: web::Data<PgPool>) -> impl Responder {
        match share::db::stats::system_stats(&pool).await {
            Ok(stats) => HttpResponse::Ok().json(stats),
            Err(e) => {
                log::error!("stats: query failed: {e}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
