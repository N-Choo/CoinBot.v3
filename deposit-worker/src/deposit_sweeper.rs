use std::time::Duration;

use kucoin::{
    client::rest::KuCoinClient,
    types::{
        deposit::DepositStatus,
        transfer::{AccountType, TransferRequest, TransferType},
    },
};
use log::{error, info, warn};
use share::db::{
    deposit::{Deposit, DepositFilter, Status as DStatus},
    user::User,
};
use sqlx::PgPool;
use tokio::time::sleep;

/// Loop every 60sec to sync the deposit record.
pub async fn run_deposit_sweeper(pool: PgPool, kcc: KuCoinClient) {
    const SLEEP_DURATION: u64 = 60;
    loop {
        info!("{:<24}", "RUNNING DEPOSIT SWEEPER");
        let pending = match DepositFilter::new()
            .with_status(&DStatus::Pending.to_string())
            .execute(&pool)
            .await
        {
            Ok(list) => list,
            Err(e) => {
                error!("{:<24} | ERR {}", "FETCH PENDING DEPOSITS FAILED", e);
                sleep(Duration::from_secs(SLEEP_DURATION)).await;
                continue;
            }
        };

        if pending.is_empty() {
            info!(
                "{:<24} | SLEEP {:>4}s",
                "NO PENDING DEPOSITS", SLEEP_DURATION
            );
            sleep(Duration::from_secs(SLEEP_DURATION)).await;
            continue;
        }

        for deposit in pending {
            let tx_hash = &deposit.tx_hash;

            match check_and_finalize(&pool, &kcc, &deposit).await {
                Ok(true) => info!("{:<24} | TX {:<36}", "DEPOSIT FINALIZED", tx_hash),

                Ok(false) => warn!("{:<24} | TX {:<36}", "DEPOSIT STILL PENDING", tx_hash),

                Err(e) => error!("{:<24} | TX {:<36} | ERR {}", "FINALIZE FAILED", tx_hash, e),
            }
        }

        // Small sleep between batches
        info!("{:<24} | SLEEP {:>4}s", "BATCH COMPLETE", 40);
        sleep(Duration::from_secs(40)).await;
    }
}

/// Update pending deposits if it is completed.
async fn check_and_finalize(
    pool: &PgPool,
    kcc: &KuCoinClient,
    deposit: &Deposit,
) -> Result<bool, String> {
    // Fetch a deposit by_tx_hash"
    let deposit_log = kcc
        .deposit()
        .by_tx_hash(&deposit.tx_hash)
        .await
        .map_err(|e| format!("KCC API Error: {}", e))?;

    let Some(record) = deposit_log else {
        return Ok(false);
    };

    if record.status != Some(DepositStatus::Success) {
        return Ok(false);
    }

    // Transfer to Spot
    {
        let currency = record
            .currency
            .as_deref()
            .ok_or_else(|| "missing currency".to_string())?;
        let raw_amount = record
            .amount
            .as_deref()
            .ok_or_else(|| "missing amount".to_string())?;
        let amount: f64 = raw_amount
            .parse()
            .map_err(|_| format!("invalid amount: {}", raw_amount))?;

        let request = TransferRequest::new(
            currency,
            amount,
            AccountType::Main,
            AccountType::Trade,
            TransferType::Internal,
        );

        if let Err(e) = kcc.transfer().execute(request).await {
            error!(
                "{:<24} | USER {:<20} | ERR {}",
                "TRANSFER FAILED", deposit.id, e
            );
            return Err(e.to_string());
        }

        info!("{:<24} | USER {:<20}", "TRANSFER SUCCESS", deposit.id);
    }

    // Add user balance.
    {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        // Add Money to Member Wallet
        let user = User::find_by_wallet(pool, &deposit.from_address)
            .await
            .map_err(|e| format!("Failed to identify user: {}", e))?
            .ok_or_else(|| format!("user not found: {}", &deposit.from_address))?;

        if let Err(e) = user.add_balance(&mut tx, &deposit.amount).await {
            return Err(e.to_string());
        }

        if let Err(e) = Deposit::confirm(&mut tx, deposit.id, &deposit.amount, None).await {
            return Err(e.to_string());
        }

        tx.commit().await.map_err(|e| e.to_string())?;
    }

    Ok(true)
}
