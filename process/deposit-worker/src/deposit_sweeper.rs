use std::time::Duration;

use kucoin::{
    client::rest::KuCoinClient,
    types::{
        deposit::{DepositHistoryRequest, DepositStatus},
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

        //Finalize newly-successful deposits (credit the user, then sweep).
        match DepositFilter::new()
            .with_status(&DStatus::Pending.to_string())
            .execute(&pool)
            .await
        {
            Ok(pending) => {
                for deposit in pending {
                    let tx_hash = deposit.tx_hash.clone();
                    match check_and_finalize(&pool, &kcc, &deposit).await {
                        Ok(true) => info!("{:<24} | TX {:<36}", "DEPOSIT FINALIZED", tx_hash),
                        Ok(false) => {
                            warn!("{:<24} | TX {:<36}", "DEPOSIT STILL PENDING", tx_hash)
                        }
                        Err(e) => {
                            error!("{:<24} | TX {:<36} | ERR {}", "FINALIZE FAILED", tx_hash, e)
                        }
                    }
                }
            }
            Err(e) => error!("{:<24} | ERR {}", "FETCH PENDING DEPOSITS FAILED", e),
        }

        // Retry sweeps that were credited but not yet moved Main -> Trade.
        match Deposit::unswept(&pool).await {
            Ok(unswept) => {
                for d in unswept {
                    let amount_value = d.amount.to_string().parse::<f64>().unwrap_or(0.0);
                    sweep_transfer(&pool, &kcc, &d, amount_value).await;
                }
            }
            Err(e) => error!("{:<24} | ERR {}", "FETCH UNSWEPT DEPOSITS FAILED", e),
        }

        info!("{:<24} | SLEEP {:>4}s", "BATCH COMPLETE", SLEEP_DURATION);
        sleep(Duration::from_secs(SLEEP_DURATION)).await;
    }
}

/// Update pending deposits if it is completed.
async fn check_and_finalize(
    pool: &PgPool,
    kcc: &KuCoinClient,
    deposit: &Deposit,
) -> Result<bool, String> {
    // Find the matching USDT deposit on KuCoin. Filter by currency + status and
    // use a large page so the record is found regardless of history size.
    let filter = DepositHistoryRequest::new(Some("USDT".to_string()))
        .set_status(DepositStatus::Success)
        .set_page_size(500);
    let deposit_log = kcc
        .deposit()
        .history(filter)
        .await
        .map_err(|e| format!("KCC API Error: {}", e))?;

    let Some(record) = deposit_log.data.and_then(|d| {
        d.items
            .into_iter()
            .find(|i| i.wallet_tx_id.as_deref() == Some(deposit.tx_hash.as_str()))
    }) else {
        return Ok(false);
    };

    if record.status != Some(DepositStatus::Success) {
        return Ok(false);
    }

    // Use KuCoin's credited amount (human units): it is what actually sits in
    // the Main account, so it is what we sweep and credit.
    let raw_amount = record
        .amount
        .as_deref()
        .ok_or_else(|| "missing amount".to_string())?;
    let amount: rust_decimal::Decimal = raw_amount
        .parse()
        .map_err(|_| format!("invalid amount: {raw_amount}"))?;
    let amount_value: f64 = raw_amount
        .parse()
        .map_err(|_| format!("invalid amount: {raw_amount}"))?;

    // Finalize the ledger first (idempotent): credit the user and confirm the
    // deposit. Doing this before the sweep means a sweep failure can never
    // double-move funds, and the user is credited exactly once.
    {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        // Create the user on their first finalized deposit, or fetch the existing one.
        let user = User::upsert(&mut tx, &deposit.from_address)
            .await
            .map_err(|e| format!("Failed to upsert user: {}", e))?;

        if let Err(e) = user.add_balance(&mut tx, amount).await {
            return Err(e.to_string());
        }

        if let Err(e) = Deposit::confirm(&mut tx, deposit.id, amount, None).await {
            return Err(e.to_string());
        }

        tx.commit().await.map_err(|e| e.to_string())?;
    }

    // Sweep Main -> Trade for platform liquidity. Best-effort and idempotent;
    // on failure the deposit stays confirmed-but-unswept and the retry pass
    // below re-attempts it. The user is already credited either way.
    sweep_transfer(pool, kcc, deposit, amount_value).await;

    Ok(true)
}

/// Move `deposit` from the Main to the Trade account and mark it swept.
///
/// Idempotent via a deterministic `clientOid` (the deposit id), so a retry after
/// a partial failure cannot move the funds twice.
async fn sweep_transfer(pool: &PgPool, kcc: &KuCoinClient, deposit: &Deposit, amount_value: f64) {
    // Claim the sweep so two workers cannot move the same deposit; released on
    // failure so a later pass retries.
    match Deposit::claim_transfer(pool, deposit.id).await {
        Ok(Some(_)) => {}
        Ok(None) => return,
        Err(e) => {
            error!("{:<24} | ID {:<20} | ERR {}", "CLAIM FAILED", deposit.id, e);
            return;
        }
    }

    let mut request = TransferRequest::new(
        "USDT",
        amount_value,
        AccountType::Main,
        AccountType::Trade,
        TransferType::Internal,
    );
    request.client_oid = deposit.id.simple().to_string();

    match kcc.transfer().execute(request).await {
        // KuCoin returns HTTP 200 with `code != 200000` / `data: null` for
        // business failures, so success must be checked at the envelope level.
        Ok(res) if res.code == "200000" && res.data.is_some() => {
            info!("{:<24} | USER {:<20}", "TRANSFER SUCCESS", deposit.id);
        }
        Ok(res) => {
            error!(
                "{:<24} | USER {:<20} | code={} msg={}",
                "TRANSFER FAILED",
                deposit.id,
                res.code,
                res.msg.as_deref().unwrap_or("<none>")
            );
            if let Err(e) = Deposit::release_transfer(pool, deposit.id).await {
                error!(
                    "{:<24} | ID {:<20} | ERR {}",
                    "RELEASE FAILED", deposit.id, e
                );
            }
        }
        Err(e) => {
            error!(
                "{:<24} | USER {:<20} | ERR {}",
                "TRANSFER FAILED", deposit.id, e
            );
            if let Err(e) = Deposit::release_transfer(pool, deposit.id).await {
                error!(
                    "{:<24} | ID {:<20} | ERR {}",
                    "RELEASE FAILED", deposit.id, e
                );
            }
        }
    }
}
