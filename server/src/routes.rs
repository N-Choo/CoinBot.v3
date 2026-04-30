use crate::handlers::user::auth::AuthController;
use actix_web::web::{self};

pub fn api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // Parent scope: /api
        web::scope("/api").service(
            web::scope("/user").service(
                web::resource("/auth")
                    .route(web::get().to(AuthController::request_challenge))
                    .route(web::post().to(AuthController::login)),
            ),
        ),
    );
}
