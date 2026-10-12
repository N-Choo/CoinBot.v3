//! Incoming signal handling and the periodic ticker/signal generator.

use kucoin::client::rest::KuCoinClient;
use share::ProcessError;
use share::auth::recover_wallet;
use share::db::contracts::{ContractFilter, Status};
use share::db::user::User;
use share::models::signals::TradeSignal;
use sqlx::{Pool, Postgres};

use crate::app_state::AppState;
use crate::trader;

/// Analyse a signal and act on every active contract authorising its ticker.
///
/// Each contract is verified before use: the stored signature must recover to
/// the wallet that owns the contract, otherwise the contract is skipped.
pub async fn signal_handler(
    signal: &TradeSignal,
    db: Pool<Postgres>,
    kcc: KuCoinClient,
) -> Result<(), ProcessError> {
    let contracts = ContractFilter::new()
        .with_ticker(&signal.ticker)
        .with_status(Status::Active)
        .execute(&db)
        .await?;

    for c in contracts {
        let wallet = match User::wallet_for_uid(&db, c.user_uid).await {
            Ok(w) => w,
            Err(e) => {
                log::error!("no wallet for contract {} (uid {}): {e}", c.id, c.user_uid);
                continue;
            }
        };

        let signer = match recover_wallet(&c.signature, &c.message) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("invalid signature on contract {}: {e}", c.id);
                continue;
            }
        };

        if signer != wallet {
            log::warn!(
                "signature mismatch on contract {}: signer={signer}, owner={wallet}",
                c.id
            );
            continue;
        }

        // Only trade while at least a quarter of the initial fund remains.
        if c.available_fund >= c.init_fund / rust_decimal::Decimal::from(4)
            && let Err(e) = trader::execute(&kcc, &db, signal, &c).await
        {
            log::error!("trade execution failed on contract {}: {e}", c.id);
        }

        log::info!(
            "contract {} verified for {} ({} {})",
            c.id,
            wallet,
            signal.action,
            signal.ticker
        );
    }

    Ok(())
}

pub async fn signal_generator(app_state: &AppState) -> Result<(), ProcessError> {
    let pool = &app_state.db_pool;
    let cache = &app_state.redis_cache;

    loop {
        let tickers = ContractFilter::new()
            .with_status(Status::Active)
            .execute_tickers(pool)
            .await?;

        let msg = serde_json::to_string(&tickers)?;
        cache.publish("tickers:analyze", &msg).await;
        tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
    }
}
