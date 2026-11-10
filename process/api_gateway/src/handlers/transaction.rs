use actix_web::{HttpResponse, Responder, web};
use share::db::user::User;
use share::db::withdrawals::Withdrawals;
use share::withdraw::{self as intent, CHAIN, CHAIN_ID};
use sqlx::PgPool;

use ethers::types::Address;
use share::{TicketRequest, deposit_service_client::DepositServiceClient};
use share::{db::deposit::DepositFilter, erc20::Erc20, rpc::Rpc};
use tonic::transport::Channel;

use crate::{
    auth::{NonceCache, SessionCache, authenticate},
    constants::{PLATFORM_WALLET, USDT_CONTRACT},
    kucoin_quote::WithdrawQuoteCache,
    models::transaction::DepositPayloadRequest,
    models::withdraw::{WithdrawPayloadRequest, WithdrawQuoteResponse, WithdrawView},
};

/// HTTP handlers for the `/api/transactions` surface.
///
/// Exposes the deposit intake path (on-chain USDT verification followed by a
/// gRPC ticket on the deposit worker) and a read path listing the caller's
/// deposits.
pub struct Transaction {}

impl Transaction {
    /// List the authenticated caller's deposits.
    ///
    /// `GET /api/transactions`
    ///
    /// Requires a valid session. Resolves the session wallet to a user and
    /// returns every deposit owned by that user. An unknown wallet is not an
    /// error: it yields an empty JSON array.
    ///
    /// Responses:
    /// - `200 OK` — JSON array of deposits (possibly empty).
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `500 Internal Server Error` — user lookup or deposit query failed.
    pub async fn list(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<PgPool>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(w) => w,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let user = match User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(u)) => u,
            Ok(None) => return HttpResponse::Ok().json(Vec::<serde_json::Value>::new()),
            Err(e) => {
                log::error!("list: find_by_wallet failed: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        match DepositFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
        {
            Ok(deposits) => HttpResponse::Ok().json(deposits),
            Err(e) => {
                log::error!("list: fetch failed for uid={}: {}", user.uid, e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }

    /// Submit an on-chain USDT deposit for verification.
    ///
    /// `POST /api/transactions/deposit`
    ///
    /// The body is a [`DepositPayloadRequest`] carrying a transaction hash. The
    /// hash is validated in this order before a ticket is created:
    ///
    /// 1. The session is authenticated.
    /// 2. The transaction is fetched on-chain and confirmed successful.
    /// 3. The transaction sender must equal the session wallet.
    /// 4. The transaction must target the USDT contract.
    /// 5. The ERC20 transfer recipient must be the platform wallet.
    /// 6. A deposit ticket is requested from the deposit worker over gRPC.
    ///
    /// Responses:
    /// - `200 OK` — ticket created; JSON `{ ticket_id, timestamp, accepted }`.
    /// - `400 Bad Request` — unknown/failed tx, non-USDT tx, malformed transfer,
    ///   or recipient is not the platform wallet.
    /// - `401 Unauthorized` — missing or invalid session.
    /// - `403 Forbidden` — tx sender does not match the session wallet.
    /// - `500 Internal Server Error` — user lookup or ticket creation failed.
    pub async fn deposit(
        header: actix_web::HttpRequest,
        payload: web::Json<DepositPayloadRequest>,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<PgPool>,
        grpc_deposit: web::Data<Channel>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(w) => w,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let tx = match Rpc::get_transaction(&payload.tx_hash).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("get_transaction failed for {}: {}", payload.tx_hash, e);
                return HttpResponse::BadRequest().finish();
            }
        };

        if let Err(e) = Rpc::confirm_onchain_success(&payload.tx_hash).await {
            log::warn!("receipt check failed for {}: {}", payload.tx_hash, e);
            return HttpResponse::BadRequest().finish();
        }

        let sender = format!("0x{:x}", tx.from);
        if sender != wallet {
            log::warn!(
                "tx.from mismatch: session wallet={}, tx sender={}",
                wallet,
                sender
            );
            return HttpResponse::Forbidden().finish();
        }

        if tx.to != Some(*USDT_CONTRACT) {
            log::warn!("tx.to is not USDT contract: {:?}", tx.to);
            return HttpResponse::BadRequest().body("Only USDT deposits accepted");
        }

        let recipient = match Erc20::decode_recipient(&tx.input) {
            Some(r) => r,
            None => return HttpResponse::BadRequest().body("Invalid USDT transfer"),
        };

        let recipient_addr: Address = match recipient.parse() {
            Ok(a) => a,
            Err(_) => return HttpResponse::BadRequest().body("Invalid recipient address"),
        };

        if recipient_addr != *PLATFORM_WALLET {
            log::warn!("ERC20 recipient is not platform wallet");
            return HttpResponse::BadRequest().body("Recipient is not platform wallet");
        }

        let user_uid = match User::uid_for_wallet(&pool, &wallet).await {
            Ok(uid) => uid,
            Err(e) => {
                log::warn!("uid_for_wallet failed for wallet='{}': {}", wallet, e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        let mut client = DepositServiceClient::new(grpc_deposit.get_ref().clone());
        let resp = match client
            .create_ticket0(TicketRequest {
                tx_hash: payload.tx_hash.clone(),
                uid: user_uid.to_string(),
                ticker: "USDT".into(),
            })
            .await
        {
            Ok(r) => r.into_inner(),
            Err(e) => {
                log::error!("Failed to create deposit: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        log::info!("Ticket {} created for USDT {}", resp.ticket_id, wallet);
        HttpResponse::Ok().json(serde_json::json!({
            "ticket_id": resp.ticket_id,
            "timestamp": resp.timestamp,
            "accepted": resp.accepted,
        }))
    }

    /// Return the live USDT-ERC20 quote for the authenticated wallet.
    ///
    /// `GET /api/transactions/withdraw/quote`
    pub async fn withdraw_quote(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<PgPool>,
        quote_cache: web::Data<WithdrawQuoteCache>,
        http: web::Data<reqwest::Client>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(wallet) => wallet,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let user = match User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(user)) => user,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(e) => {
                log::error!("withdraw_quote: user lookup failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        let quote = match quote_cache
            .get_or_fetch(&http, std::time::Duration::from_secs(60))
            .await
        {
            Ok(quote) => quote,
            Err(e) => {
                log::error!("withdraw_quote: quote failed: {e}");
                return HttpResponse::ServiceUnavailable().body(e);
            }
        };

        let max = (user.balance - quote.fee).max(rust_decimal::Decimal::ZERO);
        HttpResponse::Ok().json(WithdrawQuoteResponse {
            destination: wallet,
            currency: "USDT".into(),
            chain: CHAIN.into(),
            chain_id: CHAIN_ID,
            chain_name: quote.chain_name,
            fee: quote.fee.to_string(),
            min: quote.min.to_string(),
            precision: quote.precision,
            available_balance: user.balance.to_string(),
            max_withdrawable: max
                .round_dp_with_strategy(quote.precision, rust_decimal::RoundingStrategy::ToZero)
                .to_string(),
        })
    }

    /// Issue a one-time nonce for a withdrawal signature.
    ///
    /// `GET /api/transactions/withdraw/nonce`
    pub async fn withdraw_nonce(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        nonce_cache: web::Data<NonceCache>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(wallet) => wallet,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let nonce = uuid::Uuid::new_v4().to_string();
        // The signed intent lives for 5 minutes, so give the nonce a little
        // more headroom for the user to review and confirm.
        nonce_cache
            .insert_with_ttl(nonce.clone(), wallet, 600)
            .await;

        if nonce_cache.get(&nonce).await.is_none() {
            log::error!("withdraw_nonce: failed to store nonce");
            return HttpResponse::InternalServerError().finish();
        }

        HttpResponse::Ok().json(serde_json::json!({ "nonce": nonce }))
    }

    /// Verify a signed withdrawal, lock the funds, and queue it.
    ///
    /// `POST /api/transactions/withdraw`
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw(
        header: actix_web::HttpRequest,
        payload: web::Json<WithdrawPayloadRequest>,
        session_cache: web::Data<SessionCache>,
        nonce_cache: web::Data<NonceCache>,
        pool: web::Data<PgPool>,
        quote_cache: web::Data<WithdrawQuoteCache>,
        http: web::Data<reqwest::Client>,
        config: web::Data<crate::config::AppConfig>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(wallet) => wallet,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let parsed = match intent::parse(&payload.message) {
            Ok(parsed) => parsed,
            Err(e) => return HttpResponse::BadRequest().body(e),
        };
        if parsed.nonce != payload.nonce {
            return HttpResponse::BadRequest().body("nonce mismatch");
        }
        if let Err(e) = intent::verify(&payload.signature, &parsed, &wallet) {
            return HttpResponse::Unauthorized().body(e);
        }
        match nonce_cache.get(&payload.nonce).await {
            Some(bound) if bound == wallet => {}
            _ => {
                return HttpResponse::Conflict().json(serde_json::json!({
                    "code": "NONCE_CONFLICT",
                    "error": "invalid, expired, or already used nonce",
                }));
            }
        }

        let quote = match quote_cache
            .get_or_fetch(&http, std::time::Duration::from_secs(60))
            .await
        {
            Ok(quote) => quote,
            Err(e) => return HttpResponse::ServiceUnavailable().body(e),
        };

        let now = chrono::Utc::now().timestamp();
        let (amount, fee, total) =
            match intent::validate(&parsed, &wallet, now, quote.min, quote.precision) {
                Ok(values) => values,
                Err(e) => return HttpResponse::BadRequest().body(e),
            };

        if fee != quote.fee {
            return HttpResponse::Conflict().json(serde_json::json!({
                "code": "FEE_CHANGED",
                "error": "fee changed; re-quote",
            }));
        }

        let user = match User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(user)) => user,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(e) => {
                log::error!("withdraw: user lookup failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
        if user.balance < total {
            return HttpResponse::BadRequest().body("insufficient funds");
        }

        match Withdrawals::count_active_for_user(&pool, user.uid).await {
            Ok(active) if active >= config.withdraw_max_pending => {
                return HttpResponse::TooManyRequests().json(serde_json::json!({
                    "code": "PENDING_LIMIT",
                    "error": "too many pending withdrawals; wait for one to finish",
                }));
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("withdraw: pending count failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }

        match Withdrawals::count_since_for_user(&pool, user.uid, 24).await {
            Ok(count) if count >= config.withdraw_daily_cap => {
                return HttpResponse::TooManyRequests().json(serde_json::json!({
                    "code": "DAILY_LIMIT",
                    "error": "daily withdrawal limit reached",
                }));
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("withdraw: daily count failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }

        let Some(expires_at) = chrono::DateTime::from_timestamp(parsed.expires_at, 0) else {
            return HttpResponse::BadRequest().body("invalid expiry");
        };

        let result = Withdrawals::create_pending(
            &pool,
            user.uid,
            &wallet,
            amount,
            fee,
            total,
            &payload.nonce,
            &payload.message,
            &payload.signature,
            expires_at,
        )
        .await;

        match result {
            Ok(withdrawal) => {
                nonce_cache.invalidate(&payload.nonce).await;
                HttpResponse::Ok().json(serde_json::json!({
                    "id": withdrawal.id,
                    "status": withdrawal.status,
                    "amount": withdrawal.amount.to_string(),
                    "fee": withdrawal.fee.to_string(),
                    "total": withdrawal.total.to_string(),
                    "destination": withdrawal.destination,
                    "expires_at": withdrawal.expires_at,
                }))
            }
            Err(sqlx::Error::Database(db)) if db.constraint() == Some("withdrawals_nonce_key") => {
                HttpResponse::Conflict().json(serde_json::json!({
                    "code": "NONCE_CONFLICT",
                    "error": "nonce already used",
                }))
            }
            Err(sqlx::Error::RowNotFound) => HttpResponse::BadRequest().body("insufficient funds"),
            Err(e) => {
                log::error!("withdraw: create failed: {e}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }

    /// List the authenticated caller's withdrawals.
    ///
    /// `GET /api/transactions/withdrawals`
    pub async fn withdrawals(
        header: actix_web::HttpRequest,
        session_cache: web::Data<SessionCache>,
        pool: web::Data<PgPool>,
    ) -> impl Responder {
        let wallet = match authenticate(&header, &session_cache).await {
            Some(wallet) => wallet,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let user = match User::find_by_wallet(&pool, &wallet).await {
            Ok(Some(user)) => user,
            Ok(None) => return HttpResponse::Ok().json(Vec::<WithdrawView>::new()),
            Err(e) => {
                log::error!("withdrawals: user lookup failed: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        match Withdrawals::list_for_user(&pool, user.uid).await {
            Ok(rows) => {
                let views: Vec<WithdrawView> = rows.iter().map(WithdrawView::from).collect();
                HttpResponse::Ok().json(views)
            }
            Err(e) => {
                log::error!("withdrawals: query failed: {e}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
