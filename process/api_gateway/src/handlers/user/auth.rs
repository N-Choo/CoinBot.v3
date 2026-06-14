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

#[derive(Clone)]
pub struct SessionCache(Cache<String, String>);

#[derive(Clone)]
pub struct NonceCache(Cache<String, String>);

impl SessionCache {
    pub fn new(cache: Cache<String, String>) -> Self {
        Self(cache)
    }
    pub async fn get(&self, k: &str) -> Option<String> {
        self.0.get(k).await
    }
    pub async fn insert(&self, k: String, v: String) {
        self.0.insert(k, v).await
    }
    pub async fn invalidate(&self, k: &str) {
        self.0.invalidate(k).await
    }
}

impl NonceCache {
    pub fn new(cache: Cache<String, String>) -> Self {
        Self(cache)
    }
    pub async fn get(&self, k: &str) -> Option<String> {
        self.0.get(k).await
    }
    pub async fn insert(&self, k: String, v: String) {
        self.0.insert(k, v).await
    }
    pub async fn invalidate(&self, k: &str) {
        self.0.invalidate(k).await
    }
}

pub struct AuthController;

impl AuthController {
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

    pub fn get_wallet(s: &str, msg: &str) -> Result<String, AppError> {
        let signature = match Signature::from_str(s) {
            Ok(sig) => sig,
            Err(_) => return Err(AppError::Input("Invalid signature format".to_string())),
        };

        let wallet_addr = match signature.recover(msg) {
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
