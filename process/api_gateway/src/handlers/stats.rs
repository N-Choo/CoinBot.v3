use actix_web::{HttpResponse, Responder, web};
use share::db::stats::{self, system_stats, user_stats};
use sqlx::PgPool;

use crate::auth::{SessionCache, authenticate};

/// Public platform metrics handler.
pub struct Stats;

impl Stats {
    /// `GET /api/stats` — aggregate counts and deployed funds.
    ///
    /// Responses:
    /// - `200 OK` — JSON [`share::db::stats::SystemStats`].
    /// - `500 Internal Server Error` — query failed.
    pub async fn get(pool: web::Data<PgPool>) -> impl Responder {
        match system_stats(&pool).await {
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

        match user_stats(&pool, &wallet).await {
            Ok(Some(stats)) => HttpResponse::Ok().json(stats),
            Ok(None) => HttpResponse::Ok().json(stats::UserStats {
                total: rust_decimal::Decimal::ZERO,
                free: rust_decimal::Decimal::ZERO,
                locked: rust_decimal::Decimal::ZERO,
                realized_pnl: rust_decimal::Decimal::ZERO,
            }),
            Err(e) => {
                log::error!("user stats: query failed: {e}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
