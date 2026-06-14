use actix_web::{HttpResponse, Responder, web};
use share::db::get_uid;
use uuid::Uuid;

use crate::handlers::authenticate;
use crate::handlers::user::auth::{AuthController, NonceCache, SessionCache};
use crate::models::contracts::{BotSettings, MessagePayload, SignRequest};

pub struct Contracts;

impl Contracts {
    pub async fn get_nonce(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        nonce_cache: web::Data<NonceCache>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(w) => w,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let nonce = Uuid::new_v4().to_string();
        nonce_cache.insert(nonce.clone(), wallet).await;

        if nonce_cache.get(&nonce).await.is_none() {
            log::error!("Failed to store nonce in cache");
            return HttpResponse::InternalServerError().finish();
        }

        HttpResponse::Ok().json(serde_json::json!({ "nonce": nonce }))
    }

    pub async fn sign(
        header: actix_web::HttpRequest,
        payload: web::Json<SignRequest>,
        session_cache: web::Data<SessionCache>,
        nonce_cache: web::Data<NonceCache>,
        pool: web::Data<sqlx::PgPool>,
    ) -> impl Responder {
        let wallet = match Self::authenticated_wallet(&header, &session_cache).await {
            Ok(w) => w,
            Err(resp) => return resp,
        };

        let msg_payload: MessagePayload = match serde_json::from_str(&payload.message) {
            Ok(m) => m,
            Err(_) => return HttpResponse::BadRequest().body("Invalid message format"),
        };

        if msg_payload.nonce != payload.nonce {
            return HttpResponse::BadRequest().body("Nonce mismatch in signed message");
        }

        if let Err(resp) = Self::verify_signature(&payload.signature, &payload.message, &wallet) {
            return resp;
        }

        let settings = match BotSettings::from_message(msg_payload.settings) {
            Ok(s) => s,
            Err(msg) => return HttpResponse::BadRequest().body(msg),
        };

        if let Err(msg) = settings.validate() {
            return HttpResponse::BadRequest().body(msg);
        }

        let uid = match get_uid(&pool, &wallet).await {
            Ok(id) => id,
            Err(sqlx::Error::RowNotFound) => {
                return HttpResponse::BadRequest().body("User not found");
            }
            Err(e) => {
                log::error!("Failed to get UID for wallet {}: {}", wallet, e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        if let Err(e) = share::db::user::User::check_funds(&pool, uid, &settings.amount).await {
            log::warn!("check_funds failed for {}: {}", wallet, e);
            return HttpResponse::BadRequest().body(e);
        }

        nonce_cache.invalidate(&payload.nonce).await;

        if let Err(e) = share::db::contracts::Contracts::create(
            &pool,
            uid,
            payload.signature.clone(),
            payload.message.clone(),
            payload.nonce.clone(),
            settings.ticker,
            settings.amount,
            settings.sl_pct,
            settings.tp_pct,
        )
        .await
        {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.constraint() == Some("idx_contracts_nonce")
            {
                return HttpResponse::Conflict().body("Nonce already used");
            }
            log::error!("Failed to create contract: {}", e);
            return HttpResponse::InternalServerError().finish();
        }

        log::info!("Contract signed for wallet: {}", wallet);
        HttpResponse::Ok().json(serde_json::json!({ "message": "Contract signed" }))
    }

    async fn authenticated_wallet(
        header: &actix_web::HttpRequest,
        cache: &web::Data<SessionCache>,
    ) -> Result<String, HttpResponse> {
        match authenticate(header, cache).await {
            Some(w) => Ok(w),
            None => Err(HttpResponse::Unauthorized().finish()),
        }
    }

    fn verify_signature(
        signature: &str,
        message: &str,
        expected_wallet: &str,
    ) -> Result<(), HttpResponse> {
        match AuthController::get_wallet(signature, message) {
            Ok(recovered) if recovered == *expected_wallet => Ok(()),
            Ok(_) => Err(HttpResponse::Unauthorized().body("Wallet mismatch")),
            Err(_) => Err(HttpResponse::Unauthorized().body("Invalid signature")),
        }
    }
}
