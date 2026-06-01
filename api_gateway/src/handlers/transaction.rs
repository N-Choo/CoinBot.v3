use actix_web::{HttpResponse, Responder, web};
use ethers::types::Transaction as EthTx;
use sqlx::PgPool;

use common::{
    db::{self, deposit::Deposit},
    rpc::Rpc,
};

use crate::{
    constants::{PLATFORM_WALLET, TOKENS},
    handlers::user::auth::CacheType,
    models::transaction::DepositPayloadRequest,
};

struct DepositInfo {
    ticker: String,
}

pub struct Transaction {}
impl Transaction {
    pub async fn deposit(
        header: actix_web::HttpRequest,
        payload: web::Json<DepositPayloadRequest>,
        session_cache: web::Data<CacheType>,
        pool: web::Data<PgPool>,
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

        let info = match Self::validate(&tx) {
            Some(i) => i,
            None => {
                log::warn!(
                    "validate failed: tx.to={:?}, platform={:?}",
                    tx.to,
                    *PLATFORM_WALLET
                );
                return HttpResponse::BadRequest().finish();
            }
        };

        let user_uid = match db::get_uid(&pool, &wallet).await {
            Ok(uid) => uid,
            Err(e) => {
                log::warn!("get_uid failed for wallet='{}': {}", wallet, e);
                return HttpResponse::InternalServerError().finish();
            }
        };

        if let Err(e) =
            Deposit::create_pending(&pool, user_uid, &payload.tx_hash, &tx, &info.ticker).await
        {
            log::error!("Failed to create deposit: {}", e);
            return HttpResponse::InternalServerError().finish();
        }

        log::info!("Pending deposit: {} from {}", info.ticker, wallet);
        HttpResponse::Ok().finish()
    }

    async fn authenticate(header: &actix_web::HttpRequest, cache: &CacheType) -> Option<String> {
        let cookie = header.cookie("session_token")?;
        cache.get(cookie.value()).await
    }

    fn validate(tx: &EthTx) -> Option<DepositInfo> {
        let host = *PLATFORM_WALLET;

        if tx.to == Some(host) {
            return Some(DepositInfo {
                ticker: "ETH".into(),
            });
        }

        if let Some(to) = tx.to {
            let addr = format!("0x{:x}", to);
            if let Some(&(_, ticker)) = TOKENS.iter().find(|(a, _)| *a == addr) {
                return Some(DepositInfo {
                    ticker: ticker.into(),
                });
            }
        }
        None
    }
}
