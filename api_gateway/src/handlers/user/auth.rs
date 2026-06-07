use std::str::FromStr;

use actix_web::{HttpResponse, Responder, cookie::Cookie, web};
use ethers::types::Signature;
use log::{error, info, warn};
use moka::future::Cache;
use uuid::Uuid;

use crate::models::{
    auth::{ChallengeQuery, ChallengeResponse, VerifySignaturRequest},
    err::AppError,
};

pub type CacheType = Cache<String, String>;
pub struct AuthController;

/// ETH signature-based, Prove ownership of wallet by signing a given nonce.
impl AuthController {
    /// Handles GET /api/user/auth
    /// Generate and return a nonce for the given wallet address.
    ///
    /// # Arguments
    /// - `query` - Query parameters containing the wallet address.
    /// - `nonce_cache` - Shared cache for storing nonces.
    ///
    /// # Returns
    /// - HTTP response with the generated nonce.
    pub async fn request_challenge(
        query: web::Query<ChallengeQuery>,
        nonce_cache: web::Data<CacheType>,
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

    /// Handles POST /api/user/authentication
    /// Verifies the signature of the nonce and issues a session cookie.
    ///
    /// # Arguments
    /// - `payload` - JSON body containing the signed nonce and signature.
    /// - `nonce_cache` - Shared cache for retrieving and invalidating nonces.
    /// - `session_cache` - Shared cache for storing active sessions.
    ///
    /// # Returns
    /// - HTTP response with a session token if authentication is successful.
    /// - HTTP 401 if signature verification fails.
    /// - HTTP 400 if nonce is missing or does not match (possible replay attack).
    pub async fn login(
        payload: web::Json<VerifySignaturRequest>,
        nonce_cache: web::Data<CacheType>,
        session_cache: web::Data<CacheType>,
    ) -> impl Responder {
        let wallet = match Self::get_wallet(payload.signature.clone(), payload.msg.clone()) {
            Ok(w) => w,
            Err(_) => return HttpResponse::Unauthorized().body("Invalid signature"),
        };

        if let Some(nonce) = nonce_cache.get(&wallet).await {
            if nonce != payload.msg {
                // Nonce mismatch - possible replay attack or user error
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

    pub async fn logout(
        header: actix_web::HttpRequest,
        session_cache: web::Data<CacheType>,
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

    pub async fn verify_session(
        header: actix_web::HttpRequest,
        session_cache: web::Data<CacheType>,
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

    /// Helper function to recover wallet address from signature and message.
    pub fn get_wallet(s: String, msg: String) -> Result<String, AppError> {
        // Validate Signature Format
        let signature = match Signature::from_str(&s) {
            Ok(sig) => sig,
            Err(_) => return Err(AppError::Input("Invalid signature format".to_string())),
        };

        // Recover Wallet Address
        let wallet_addr = match signature.recover(msg) {
            Ok(addr) => addr,
            Err(e) => {
                warn!("Signature recovery failed: {:?}", e);
                return Err(AppError::Input(" Invalid signature ".to_string()));
            }
        };

        info!("Recovered wallet: {:?}", wallet_addr);
        let full_address = format!("0x{:x}", wallet_addr); // Standard hex .
        Ok(full_address.to_lowercase())
    }
}
