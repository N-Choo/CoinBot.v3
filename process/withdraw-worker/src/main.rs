//! Withdrawal worker entry point.
//!
//! A DB-polling loop that executes signed withdrawals against KuCoin. The HTTP
//! intake and signature verification live in `api_gateway`; this process only
//! re-verifies the stored intent before moving money.

use std::str::FromStr;

use kucoin::client::rest::{Credentials, KuCoinClient};
use rust_decimal::Decimal;
use share::{ProcessConfig, ProcessError};

mod exchange;
mod worker;

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenvy::dotenv().ok();
    share::logger::init_logger();

    let cfg = ProcessConfig::new("withdraw");

    let database_url = cfg.get("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;

    // Require withdrawal-specific keys: never fall back to the deposit/trade
    // keys, which may carry broader exchange permissions.
    let kc_key = cfg.get_prefixed("KC_KEY")?;
    let kc_secret = cfg.get_prefixed("KC_SECRET")?;
    let kc_passphrase = cfg.get_prefixed("KC_PASSPHRASE")?;
    for other in ["DEPOSIT_KC_KEY", "TRADE_ENGINE_KC_KEY"] {
        if std::env::var(other).is_ok_and(|value| value == kc_key) {
            log::warn!(
                "WITHDRAW_KC_KEY is shared with {other}; use a dedicated withdrawal-only KuCoin key"
            );
        }
    }
    let kcc = KuCoinClient::new(Credentials::new(&kc_key, &kc_secret, &kc_passphrase));

    let main_buffer = cfg
        .get("WITHDRAW_MAIN_BUFFER_USDT")
        .ok()
        .and_then(|value| Decimal::from_str(&value).ok())
        .unwrap_or(Decimal::ZERO);

    let worker_config = worker::WorkerConfig {
        poll_secs: cfg.get_or("WITHDRAW_POLL_SECS", 15u64),
        claim_timeout_secs: cfg.get_or("WITHDRAW_CLAIM_TIMEOUT", 120i64),
        reconcile_window_secs: cfg.get_or("WITHDRAW_RECONCILE_WINDOW", 60i64),
        main_buffer,
    };

    log::info!("Withdrawal worker started");
    worker::run(pool, kcc, worker_config).await;
    Ok(())
}
