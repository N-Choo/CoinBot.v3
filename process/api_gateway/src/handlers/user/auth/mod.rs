//! Wallet-based authentication: challenge/response login, sessions, and logout.
pub mod cache;

pub use cache::{NonceCache, SessionCache};

use std::str::FromStr;

use actix_web::{HttpResponse, Responder, cookie::Cookie, web};
use ethers::types::Signature;
use ethers::utils::hash_message;
use log::{error, info, warn};
use uuid::Uuid;

use crate::models::auth::{ChallengeQuery, ChallengeResponse, VerifySignaturRequest};
use crate::models::err::AppError;

/// Wallet authentication handlers.
pub struct AuthController;

impl AuthController {
    /// `GET /api/user/auth?wallet_address=...` — issue a login nonce.
    ///
    /// Generates a UUID nonce, stores it in the nonce cache keyed by the
    /// lowercased wallet address, and returns it as [`ChallengeResponse`].
    ///
    /// Responses:
    /// - `200 OK` — JSON `{ nonce }`.
    /// - `500 Internal Server Error` — nonce could not be stored.
    pub async fn request_challenge(
        query: web::Query<ChallengeQuery>,
        nonce_cache: web::Data<NonceCache>,
    ) -> impl Responder {
        let wallet = query.wallet_address.to_lowercase();
        let nonce = Uuid::new_v4().to_string();

        nonce_cache.insert(wallet.clone(), nonce.clone()).await;

        if nonce_cache.get(&wallet).await.is_none() {
            log::error!("Failed to store nonce in cache for wallet={}", wallet);
            return HttpResponse::InternalServerError().finish();
        }

        HttpResponse::Ok().json(ChallengeResponse { nonce })
    }

    /// `POST /api/user/auth` — verify a signed nonce and start a session.
    ///
    /// Recovers the wallet from the signature, requires it to match the nonce
    /// stored for that wallet, invalidates the nonce, and issues a session token
    /// as an HttpOnly `session_token` cookie.
    ///
    /// Responses:
    /// - `200 OK` — session cookie set.
    /// - `400 Bad Request` — nonce missing or mismatched.
    /// - `401 Unauthorized` — signature could not be recovered.
    /// - `500 Internal Server Error` — session could not be stored.
    pub async fn login(
        payload: web::Json<VerifySignaturRequest>,
        nonce_cache: web::Data<NonceCache>,
        session_cache: web::Data<SessionCache>,
    ) -> impl Responder {
        let wallet = match Self::get_wallet(&payload.signature, &payload.msg) {
            Ok(w) => w,
            Err(_) => return HttpResponse::Unauthorized().body("Invalid signature"),
        };

        if let Some(nonce) = nonce_cache.get(&wallet).await {
            if nonce != payload.msg {
                nonce_cache.invalidate(&wallet).await;
                return HttpResponse::BadRequest().finish();
            }
        } else {
            return HttpResponse::BadRequest().finish();
        }

        nonce_cache.invalidate(&wallet).await;

        let token = uuid::Uuid::new_v4().to_string();
        session_cache.insert(token.clone(), wallet.clone()).await;

        if session_cache.get(&token).await.is_none() {
            error!("Failed to store session token in cache");
            return HttpResponse::InternalServerError().finish();
        }

        let session_cookies = Cookie::build("session_token", token.clone())
            .path("/")
            .http_only(true)
            .same_site(actix_web::cookie::SameSite::Strict)
            .max_age(actix_web::cookie::time::Duration::hours(2))
            .finish();

        HttpResponse::Ok().cookie(session_cookies).finish()
    }

    /// `POST /api/user/logout` — invalidate the current session.
    ///
    /// Responses:
    /// - `200 OK` — session invalidated.
    /// - `400 Bad Request` — no session cookie present.
    pub async fn logout(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
    ) -> impl Responder {
        let session_cookie = header.cookie("session_token");
        if let Some(cookie) = session_cookie {
            let token = cookie.value().to_string();
            session_cache.invalidate(&token).await;
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::BadRequest().body("No session cookie found")
        }
    }

    /// `POST /api/user/verify` — check whether the session cookie is valid.
    ///
    /// Responses:
    /// - `200 OK` — session is valid.
    /// - `400 Bad Request` — no session cookie present.
    /// - `401 Unauthorized` — session token unknown or expired.
    pub async fn verify_session(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
    ) -> impl Responder {
        let session_cookie = header.cookie("session_token");
        if let Some(cookie) = session_cookie {
            let token = cookie.value().to_string();
            if session_cache.get(&token).await.is_some() {
                HttpResponse::Ok().finish()
            } else {
                HttpResponse::Unauthorized().body("Invalid session")
            }
        } else {
            HttpResponse::BadRequest().body("No session cookie found")
        }
    }

    /// Recover the signer address from a signature over `msg`.
    ///
    /// Returns the lowercased `0x`-prefixed wallet address, or an
    /// [`AppError::Input`] when the signature is malformed or cannot be
    /// recovered.
    pub fn get_wallet(s: &str, msg: &str) -> Result<String, AppError> {
        let signature = match Signature::from_str(s) {
            Ok(sig) => sig,
            Err(_) => return Err(AppError::Input("Invalid signature format".to_string())),
        };

        let msg_hash = hash_message(msg);
        let wallet_addr = match signature.recover(msg_hash) {
            Ok(addr) => addr,
            Err(e) => {
                warn!("Signature recovery failed: {:?}", e);
                return Err(AppError::Input(" Invalid signature ".to_string()));
            }
        };

        info!("Recovered wallet: {:?}", wallet_addr);
        let full_address = format!("0x{:x}", wallet_addr);
        Ok(full_address.to_lowercase())
    }
}
