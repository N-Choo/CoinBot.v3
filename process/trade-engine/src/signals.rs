//! Incoming signal handling and the periodic ticker/signal generator.

use kucoin::client::rest::KuCoinClient;
use share::ProcessError;
use share::auth::recover_wallet;
use share::db::contracts::{ContractFilter, Status};
use share::db::user::User;
use share::models::signals::TradeSignal;
use sqlx::{Pool, Postgres};

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

        // Only trade when the available fund covers the per-order budget
        // (half of the initial fund).
        if c.available_fund >= trader::order_budget(c.init_fund)
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

