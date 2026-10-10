use kucoin::client::rest::KuCoinClient;
use share::ProcessError;
use share::auth::recover_wallet;
use share::db::contracts::{ContractFilter, Contracts, Status};
use share::db::trade::{TradeOrderFilter, TradeOrders};
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
    let kcc = app_state.kucoin.clone();

    // Sync contract status
    let sweeper_db = db.clone();
    let kc_client = kcc.clone();
    tokio::spawn(async move {
        loop {
            sync_contract_status(sweeper_db.clone(), kc_client.clone()).await;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

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
                let kcc = kcc.clone();
                async move {
                    match serde_json::from_str::<Vec<TradeSignal>>(&payload) {
                        Ok(signals) => {
                            for signal in &signals {
                                if let Err(e) =
                                    signal_handler(signal, db.clone(), kcc.clone()).await
                                {
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

/// Sync open trade orders against the exchange and settle their contracts.
async fn sync_contract_status(db: Pool<Postgres>, kcc: KuCoinClient) {
    if let Ok(active_contracts) = ContractFilter::new()
        .with_status(Status::Active)
        .execute(&db)
        .await
    {
        for c in active_contracts {
            if let Ok(trade_orders) = TradeOrderFilter::new()
                .with_contract(c.id)
                .with_ticker(&c.ticker)
                .with_status(share::db::trade::Status::Open)
                .execute(&db)
                .await
            {
                for o in trade_orders {
                    let Some(order_id) = o.exchange_order_id.as_deref() else {
                        continue;
                    };
                    let symbol = o.ticker.replace('/', "-");

                    match o.status.as_str() {
                        // Working on the exchange — reconcile it below.
                        "open" => match kcc.spot().get_order(order_id, &symbol).await {
                            Ok(order) => {
                                let Some(data) = order.data else { continue };

                                // Still working — nothing to settle yet.
                                if data.active {
                                    continue;
                                }

                                let filled: f64 = data.deal_size.parse().unwrap_or(0.0);
                                if filled <= 0.0 {
                                    // Done with no fill => cancelled.
                                    if let Ok(mut tx) = db.begin().await {
                                        let _ = TradeOrders::mark_cancelled(
                                            &mut tx,
                                            o.id,
                                            "no fill (cancelled)",
                                        )
                                        .await;
                                        // Return the reserved funds to the contract.
                                        let _ =
                                            Contracts::release_fund(&mut tx, c.id, &o.funds).await;
                                        let _ = tx.commit().await;
                                    }
                                    continue;
                                }

                                let funds: f64 = data.deal_funds.parse().unwrap_or(0.0);
                                let avg = funds / filled; // avg fill = dealFunds / dealSize

                                if let Ok(mut tx) = db.begin().await {
                                    let _ = TradeOrders::mark_filled(
                                        &mut tx,
                                        o.id,
                                        &data.deal_size,
                                        &avg.to_string(),
                                        &data.fee,
                                    )
                                    .await;
                                    // Credit the filled proceeds back to the contract.
                                    let _ =
                                        Contracts::add_fund(&mut tx, c.id, &data.deal_funds).await;
                                    let _ = tx.commit().await;
                                }

                                log::info!(
                                    "[SWEEP] {} order {} filled {} @ {avg} (fee {})",
                                    symbol,
                                    o.id,
                                    data.deal_size,
                                    data.fee
                                );
                            }
                            Err(e) => trader::fail_order(&db, o.id, &e.to_string()).await,
                        },

                        // Not accepted yet, or already terminal: nothing to sync.
                        "pending" | "filled" | "closed" | "cancelled" | "failed" => {
                            log::debug!("[SWEEP] order {} skipped (status={})", o.id, o.status);
                            continue;
                        }

                        other => {
                            log::warn!("[SWEEP] order {} has unknown status '{}'", o.id, other);
                            continue;
                        }
                    }
                }
            }
        }
    }
}

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

        let Ok(init_fund) = c.init_fund.parse::<f64>() else {
            log::warn!("contract {} has invalid init_fund '{}'", c.id, c.init_fund);
            continue;
        };
        let Ok(available) = c.available_fund.parse::<f64>() else {
            log::warn!(
                "contract {} has invalid available_fund '{}'",
                c.id,
                c.available_fund
            );
            continue;
        };

        if available >= init_fund / 4.0
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
