use share::ProcessError;
use share::auth::recover_wallet;
use share::db::contracts::{ContractFilter, Status};
use share::models::signals::TradeSignal;

mod app_config;
mod app_state;
mod trader;

use app_config::AppConfig;
use app_state::AppState;
use sqlx::{Pool, Postgres};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let config = AppConfig::from_env()?;
    let app_state = AppState::new(&config).await?;
    let app_state_copy = app_state.clone();

    let db = app_state.db_pool;
    let cache = app_state.redis_cache.clone();

    // loop signal generator
    tokio::spawn(async move {
        if let Err(e) = signal_generator(&app_state_copy).await {
            log::error!("ignite analyze err: {e}");
        }
    });

    // Open listen channel to recieves `TradeSignal`
    let signal_cache = cache.clone();
    tokio::spawn(async move {
        if let Err(e) = signal_cache
            .subscribe("signals:result", move |payload| {
                let db = db.clone();
                async move {
                    match serde_json::from_str::<Vec<TradeSignal>>(&payload) {
                        Ok(signals) => {
                            for signal in &signals {
                                if let Err(e) = signal_handler(signal, db.clone()).await {
                                    log::error!("signal handler failed: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("failed to parse signal: {e}");
                        }
                    }
                }
            })
            .await
        {
            log::error!("signal subscriber error: {e}");
        }
    });

    // Wait for Ctrl+C before shutting down
    signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    log::info!("shutting down...");

    Ok(())
}

/// Analyse a signal and act on every active contract authorising its ticker.
///
/// Each contract is verified before use: the stored signature must recover to
/// the wallet that owns the contract, otherwise the contract is skipped.
pub async fn signal_handler(signal: &TradeSignal, db: Pool<Postgres>) -> Result<(), ProcessError> {
    let contracts = ContractFilter::new()
        .with_ticker(&signal.ticker)
        .with_status(Status::Active)
        .execute(&db)
        .await?;

    for c in contracts {
        let wallet = match share::db::wallet_for_uid(&db, c.user_uid).await {
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

        // let mininum = c.init_fund / 4;
        // if mininum =< c.available_fund {
        //
        //     // gerenerate trade ticket to kcc.
        //     // comsume_fund
        //
        //
        // }

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

async fn signal_generator(app_state: &AppState) -> Result<(), ProcessError> {
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
