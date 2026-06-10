use actix_web::{HttpResponse, Responder, web};
use sqlx::PgPool;

use common::{TicketRequest, deposit_service_client::DepositServiceClient};
use ethers::types::Address;
use share::{
    db::{self, deposit::DepositFilter},
    erc20::Erc20,
    rpc::Rpc,
};
use tonic::transport::Channel;

use crate::{
    constants::{PLATFORM_WALLET, USDT_CONTRACT},
    handlers::user::auth::CacheType,
    models::transaction::DepositPayloadRequest,
};

pub struct Transaction {}

impl Transaction {
    pub async fn list(
        header: actix_web::HttpRequest,
        session_cache: web::Data<CacheType>,
        pool: web::Data<PgPool>,
    ) -> impl Responder {
        let wallet = match Self::authenticate(&header, &session_cache).await {
            Some(w) => w,
            None => return HttpResponse::Unauthorized().finish(),
        };

        let user = match share::db::user::User::find_by_wallet(&pool, &wallet).await {
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

    pub async fn deposit(
        header: actix_web::HttpRequest,
        payload: web::Json<DepositPayloadRequest>,
        session_cache: web::Data<CacheType>,
        pool: web::Data<PgPool>,
        grpc_deposit: web::Data<Channel>,
    ) -> impl Responder {
        let wallet = match Self::authenticate(&header, &session_cache).await {
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

        let user_uid = match db::get_uid(&pool, &wallet).await {
            Ok(uid) => uid,
            Err(e) => {
                log::warn!("get_uid failed for wallet='{}': {}", wallet, e);
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

    async fn authenticate(header: &actix_web::HttpRequest, cache: &CacheType) -> Option<String> {
        let cookie = header.cookie("session_token")?;
        cache.get(cookie.value()).await
    }
}
