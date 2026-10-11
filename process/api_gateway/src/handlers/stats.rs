use actix_web::{HttpResponse, Responder, web};
use sqlx::PgPool;

use crate::handlers::authenticate;
use crate::handlers::user::auth::SessionCache;

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

    /// `GET /api/user/stats` — the authenticated user's balance stats.
    ///
    /// Responses:
    /// - `200 OK` — JSON `{ total, free, locked, realized_pnl }` (decimal strings).
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `500 Internal Server Error` — query failed.
    pub async fn user(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<PgPool>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(w) => w,
            None => return HttpResponse::Unauthorized().finish(),
        };

        match share::db::stats::user_stats(&pool, &wallet).await {
            Ok(Some(stats)) => HttpResponse::Ok().json(stats),
            Ok(None) => HttpResponse::Ok().json(share::db::stats::UserStats {
                total: "0".into(),
                free: "0".into(),
                locked: "0".into(),
                realized_pnl: "0".into(),
            }),
            Err(e) => {
                log::error!("user stats: query failed: {e}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
