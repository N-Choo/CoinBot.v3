use crate::handlers::contracts::Contracts;
use crate::handlers::transaction::Transaction;
use crate::handlers::user::auth::AuthController;
use actix_web::web::{self};

pub fn api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/user")
                    .service(
                        web::resource("/auth")
                            .route(web::get().to(AuthController::request_challenge))
                            .route(web::post().to(AuthController::login)),
                    )
                    .route("/logout", web::post().to(AuthController::logout))
                    .route("/verify", web::post().to(AuthController::verify_session)),
            )
            .service(
                web::scope("/transactions")
                    .route("/deposit", web::post().to(Transaction::deposit))
                    .route("", web::get().to(Transaction::list)),
            )
            .service(
                web::scope("/contracts")
                    .route("/nonce", web::get().to(Contracts::get_nonce))
                    .route("/sign", web::post().to(Contracts::sign)),
            )
            .route("/config", web::get().to(crate::constants::get_config)),
    );
}
