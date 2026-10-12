use actix_web::{HttpResponse, Responder, web};
use share::auth::recover_wallet;
use uuid::Uuid;

use crate::auth::{NonceCache, SessionCache, authenticate};
use crate::models::contracts::{BotSettings, ContractView, MessagePayload, SignRequest};

/// HTTP handlers for the `/api/contracts` surface.
///
/// Issues one-time signing nonces and accepts signed bot-contract requests.
pub struct Contracts;

impl Contracts {
    /// Issue a one-time nonce for the authenticated wallet.
    ///
    /// `GET /api/contracts/nonce`
    ///
    /// Stores a fresh UUID nonce in the nonce cache, bound to the session
    /// wallet, and returns it as `{ "nonce": ... }`.
    ///
    /// Responses:
    /// - `200 OK` — JSON `{ nonce }`.
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `500 Internal Server Error` — nonce could not be stored.
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

    /// Verify and persist a signed bot contract.
    ///
    /// `POST /api/contracts/sign`
    ///
    /// Validates, in order: the session, the signed message format, that the
    /// message nonce matches the request nonce, the signature (the recovered
    /// wallet must equal the session wallet), the bot settings, that the user
    /// exists, and that the user has sufficient funds. On success the nonce is
    /// invalidated and the contract is stored.
    ///
    /// Responses:
    /// - `200 OK` — JSON `{ "message": "Contract signed" }`.
    /// - `400 Bad Request` — malformed message, nonce mismatch, invalid
    ///   settings, unknown user, or insufficient funds.
    /// - `401 Unauthorized` — missing session, wallet mismatch, or bad signature.
    /// - `409 Conflict` — nonce already used.
    /// - `500 Internal Server Error` — unexpected persistence failure.
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

        // The nonce must be one this wallet was issued (replay protection),
        // not just equal to the one echoed inside the signed message.
        match nonce_cache.get(&payload.nonce).await {
            Some(bound) if bound == wallet => {}
            _ => return HttpResponse::BadRequest().body("Invalid or expired nonce"),
        }

        let settings = match BotSettings::from_message(msg_payload.settings) {
            Ok(s) => s,
            Err(msg) => return HttpResponse::BadRequest().body(msg),
        };

        if let Err(msg) = settings.validate() {
            return HttpResponse::BadRequest().body(msg);
        }

        let user = match share::db::user::User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(u)) => u,
            Ok(None) => return HttpResponse::BadRequest().body("User not found"),
            Err(e) => {
                log::error!("Failed to load user for wallet {}: {}", wallet, e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to begin transaction: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        if let Err(e) = user.lock_balance(&mut tx, settings.init_fund).await {
            return match e {
                sqlx::Error::RowNotFound => HttpResponse::BadRequest().body("Insufficient funds"),
                other => {
                    log::error!("lock_balance failed for {}: {}", wallet, other);
                    HttpResponse::InternalServerError().finish()
                }
            };
        }

        nonce_cache.invalidate(&payload.nonce).await;

        if let Err(e) = share::db::contracts::Contracts::create(
            &mut tx,
            user.uid,
            &payload.signature,
            &payload.message,
            &payload.nonce,
            &settings.ticker,
            settings.init_fund,
            settings.init_fund,
            rust_decimal::Decimal::ZERO,
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

        if let Err(e) = tx.commit().await {
            log::error!("Failed to commit contract for {}: {}", wallet, e);
            return HttpResponse::InternalServerError().finish();
        }

        log::info!("Contract signed for wallet: {}", wallet);
        HttpResponse::Ok().json(serde_json::json!({ "message": "Contract signed" }))
    }

    /// List the authenticated caller's contracts.
    ///
    /// `GET /api/contracts/me`
    ///
    /// Requires a valid session. Resolves the session wallet to a user and
    /// returns every contract owned by that user as [`ContractView`] (signing
    /// material omitted). An unknown wallet is not an error: it yields an empty
    /// JSON array.
    ///
    /// Responses:
    /// - `200 OK` — JSON array of contracts (possibly empty).
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `500 Internal Server Error` — user lookup or contract query failed.
    pub async fn get_my_contracts(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<sqlx::PgPool>,
    ) -> impl Responder {
        let wallet = match Self::authenticated_wallet(&header, &session_cache).await {
            Ok(w) => w,
            Err(resp) => return resp,
        };

        let user = match share::db::user::User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(u)) => u,
            Ok(None) => return HttpResponse::Ok().json(Vec::<ContractView>::new()),
            Err(e) => {
                log::error!("get_my_contracts: find_by_wallet failed: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        match share::db::contracts::ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
        {
            Ok(contracts) => {
                let views: Vec<ContractView> = contracts.iter().map(ContractView::from).collect();
                HttpResponse::Ok().json(views)
            }
            Err(e) => {
                log::error!("get_my_contracts: query failed for uid={}: {}", user.uid, e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }

    /// List the authenticated caller's trade orders for one contract.
    ///
    /// `GET /api/contracts/{id}/orders`
    ///
    /// Requires a valid session. Returns the caller's orders for that contract,
    /// newest first. Only the caller's own orders are ever returned.
    ///
    /// Responses:
    /// - `200 OK` — JSON array of trade orders (possibly empty).
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `500 Internal Server Error` — user lookup or query failed.
    pub async fn orders(
        header: actix_web::HttpRequest,
        path: web::Path<Uuid>,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<sqlx::PgPool>,
    ) -> impl Responder {
        let wallet = match Self::authenticated_wallet(&header, &session_cache).await {
            Ok(w) => w,
            Err(resp) => return resp,
        };

        let user = match share::db::user::User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(u)) => u,
            Ok(None) => return HttpResponse::Ok().json(Vec::<share::db::trade::TradeOrder>::new()),
            Err(e) => {
                log::error!("orders: find_by_wallet failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        match share::db::trade::TradeOrderFilter::new()
            .with_user(user.uid)
            .with_contract(path.into_inner())
            .execute(&pool)
            .await
        {
            Ok(orders) => HttpResponse::Ok().json(orders),
            Err(e) => {
                log::error!("orders: query failed for uid={}: {e}", user.uid);
                HttpResponse::InternalServerError().finish()
            }
        }
    }

    /// Authenticate the request, mapping failure to a `401` response.
    async fn authenticated_wallet(
        header: &actix_web::HttpRequest,
        cache: &web::Data<SessionCache>,
    ) -> Result<String, HttpResponse> {
        match authenticate(header, cache).await {
            Some(w) => Ok(w),
            None => Err(HttpResponse::Unauthorized().finish()),
        }
    }

    /// Recover the signer from `signature`/`message` and require it to equal
    /// `expected_wallet`.
    ///
    /// Returns `401` when the signature is invalid or recovers a different
    /// wallet.
    fn verify_signature(
        signature: &str,
        message: &str,
        expected_wallet: &str,
    ) -> Result<(), HttpResponse> {
        match recover_wallet(signature, message) {
            Ok(recovered) if recovered == *expected_wallet => Ok(()),
            Ok(_) => Err(HttpResponse::Unauthorized().body("Wallet mismatch")),
            Err(_) => Err(HttpResponse::Unauthorized().body("Invalid signature")),
        }
    }
}
