//! Polling loop that expires, verifies, executes, and reconciles withdrawals.

use std::time::Duration;

use chrono::Utc;
use kucoin::client::rest::KuCoinClient;
use rust_decimal::Decimal;
use share::db::withdrawals::{Withdrawal, Withdrawals};
use share::withdraw as intent;
use sqlx::PgPool;

use crate::exchange;

/// Runtime knobs for the worker loop.
#[derive(Debug, Clone, Copy)]
pub struct WorkerConfig {
    pub poll_secs: u64,
    pub claim_timeout_secs: i64,
    pub reconcile_window_secs: i64,
    pub main_buffer: Decimal,
}

/// How a single claimed row ended.
enum Outcome {
    /// The row reached a new state or is being reconciled; keep draining.
    Continue,
    /// The row was returned to pending (for example after a transfer); stop
    /// this pass so it is not immediately re-claimed.
    Yield,
}

/// Run until the process is stopped.
pub async fn run(pool: PgPool, kcc: KuCoinClient, cfg: WorkerConfig) {
    let mut pass_number: u64 = 0;

    loop {
        if let Err(e) = pass(&pool, &kcc, cfg).await {
            log::error!("[WITHDRAW] pass failed: {e}");
        }

        // Solvency is informational and comparatively expensive, so sample it.
        if pass_number.is_multiple_of(20) {
            check_solvency(&pool, &kcc).await;
        }

        pass_number = pass_number.wrapping_add(1);
        tokio::time::sleep(Duration::from_secs(cfg.poll_secs)).await;
    }
}

/// One full pass: expire, execute pending rows, reconcile the rest.
async fn pass(pool: &PgPool, kcc: &KuCoinClient, cfg: WorkerConfig) -> Result<(), String> {
    for withdrawal in Withdrawals::expire_safe(pool, cfg.claim_timeout_secs)
        .await
        .map_err(|e| e.to_string())?
    {
        log::info!("[WITHDRAW] {} expired", withdrawal.id);
    }

    while let Some(withdrawal) = Withdrawals::claim_next(pool)
        .await
        .map_err(|e| e.to_string())?
    {
        match process(pool, kcc, cfg, &withdrawal).await {
            Ok(Outcome::Continue) => {}
            Ok(Outcome::Yield) => break,
            Err(e) => {
                log::warn!("[WITHDRAW] {} processing failed: {e}", withdrawal.id);
                // A transient failure before the exchange call returns the row
                // to pending; the SQL guard keeps attempted rows untouched.
                let _ = Withdrawals::release_unattempted(pool, withdrawal.id).await;
                break;
            }
        }
    }

    // Run both independently so a transient history failure cannot stop
    // already-submitted withdrawals from settling.
    let mut errors = Vec::new();
    if let Err(e) = reconcile_processing(pool, kcc, cfg).await {
        errors.push(format!("processing: {e}"));
    }
    if let Err(e) = reconcile_submitted(pool, kcc).await {
        errors.push(format!("submitted: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

async fn process(
    pool: &PgPool,
    kcc: &KuCoinClient,
    cfg: WorkerConfig,
    withdrawal: &Withdrawal,
) -> Result<Outcome, String> {
    if withdrawal.expires_at <= Utc::now() {
        Withdrawals::mark_expired(pool, withdrawal.id)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Outcome::Continue);
    }

    let quote = exchange::quote(kcc).await?;
    if let Err(reason) = verify(withdrawal, quote.min, quote.precision) {
        Withdrawals::mark_failed(pool, withdrawal.id, &reason)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Outcome::Continue);
    }
    if quote.fee != withdrawal.fee {
        Withdrawals::mark_failed(pool, withdrawal.id, "fee changed")
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Outcome::Continue);
    }

    let required = withdrawal.amount + quote.fee + cfg.main_buffer;
    let main = exchange::main_available(kcc).await?;
    if main < required {
        let shortfall = required - main;
        let margin = exchange::margin_available(kcc).await?;
        if margin < shortfall {
            Withdrawals::mark_failed(pool, withdrawal.id, "platform liquidity unavailable")
                .await
                .map_err(|e| e.to_string())?;
            return Ok(Outcome::Continue);
        }

        exchange::transfer_to_main(kcc, shortfall, &withdrawal.id.to_string()).await?;
        // No exchange call was made; put the row back so a later pass retries
        // after the transfer settles.
        Withdrawals::release_unattempted(pool, withdrawal.id)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Outcome::Yield);
    }

    let attempted = Withdrawals::mark_attempt(pool, withdrawal.id, withdrawal.amount, quote.fee)
        .await
        .map_err(|e| e.to_string())?;
    if !attempted {
        // The row left `processing` between claim and attempt. Do not call the
        // exchange: its funds may already have been refunded.
        return Err("row is no longer processing before the exchange call".into());
    }

    match exchange::create(
        kcc,
        withdrawal.amount,
        &withdrawal.destination,
        &withdrawal.id.to_string(),
    )
    .await
    {
        Ok(exchange_id) => {
            let stored = Withdrawals::mark_submitted(pool, withdrawal.id, &exchange_id)
                .await
                .map_err(|e| e.to_string())?;
            if !stored {
                log::error!(
                    "[WITHDRAW] {} paid as {exchange_id} but could not be marked submitted; manual reconciliation required",
                    withdrawal.id
                );
            }
            Ok(Outcome::Continue)
        }
        Err(exchange::CreateError::Rejected(reason)) => {
            Withdrawals::mark_failed(pool, withdrawal.id, &reason)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Outcome::Continue)
        }
        Err(exchange::CreateError::Retry(reason)) => {
            log::warn!(
                "[WITHDRAW] {} retryable create error: {reason}",
                withdrawal.id
            );
            Withdrawals::release_attempt(pool, withdrawal.id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Outcome::Yield)
        }
        Err(exchange::CreateError::Ambiguous(reason)) => {
            // The exchange may have accepted the order. The next pass
            // reconciles it through history using the unique `remark`.
            log::warn!("[WITHDRAW] {} create ambiguous: {reason}", withdrawal.id);
            Ok(Outcome::Continue)
        }
    }
}

fn verify(withdrawal: &Withdrawal, min: Decimal, precision: u32) -> Result<(), String> {
    let parsed = intent::parse(&withdrawal.message)?;

    if withdrawal.currency != "USDT" || withdrawal.chain != "eth" {
        return Err("unsupported currency or chain".into());
    }
    if parsed.nonce != withdrawal.nonce {
        return Err("stored nonce mismatch".into());
    }
    if !parsed
        .destination
        .eq_ignore_ascii_case(&withdrawal.destination)
    {
        return Err("stored destination mismatch".into());
    }
    if parsed.amount_decimal()? != withdrawal.amount
        || parsed.fee_decimal()? != withdrawal.fee
        || parsed.total_decimal()? != withdrawal.total
    {
        return Err("stored amounts do not match the signed message".into());
    }

    intent::verify(&withdrawal.signature, &parsed, &withdrawal.destination)?;

    intent::validate(
        &parsed,
        &withdrawal.destination,
        Utc::now().timestamp(),
        min,
        precision,
    )?;
    Ok(())
}

/// Resolve `processing` rows whose create call had an unknown outcome.
async fn reconcile_processing(
    pool: &PgPool,
    kcc: &KuCoinClient,
    cfg: WorkerConfig,
) -> Result<(), String> {
    let rows = Withdrawals::processing_attempted(pool)
        .await
        .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Ok(());
    }

    let start_ms = rows
        .iter()
        .filter_map(|row| row.attempted_at)
        .min()
        .map(|at| at.timestamp_millis() - 300_000)
        .unwrap_or_else(|| Utc::now().timestamp_millis() - 3_600_000);
    let history = exchange::history_since(kcc, start_ms).await?;

    for row in rows {
        let id = row.id.to_string();
        let found = history
            .iter()
            .find(|item| item.remark.as_deref() == Some(id.as_str()));

        if let Some(item) = found {
            if let Some(expected) = row.attempted_amount {
                if let Ok(reported) = item.amount.parse::<Decimal>() {
                    if reported != expected {
                        log::warn!(
                            "[WITHDRAW] {} reconciled amount mismatch: attempted={expected} exchange={reported}",
                            row.id
                        );
                    }
                }
            }
            let stored = Withdrawals::mark_submitted(pool, row.id, &item.id)
                .await
                .map_err(|e| e.to_string())?;
            if !stored {
                log::error!(
                    "[WITHDRAW] {} found as {} but is no longer processing",
                    row.id,
                    item.id
                );
            }
            continue;
        }

        // Never refund an ambiguous create: the exchange may simply be late or
        // the item may be beyond the pages we read. Keep the funds locked and
        // surface it for manual reconciliation, at most once every 5 minutes.
        if let Some(attempted_at) = row.attempted_at {
            let age = Utc::now().signed_duration_since(attempted_at).num_seconds();
            if age >= cfg.reconcile_window_secs && age % 300 < cfg.poll_secs as i64 {
                log::error!(
                    "[WITHDRAW] {} create unconfirmed after {age}s; manual reconciliation required",
                    row.id
                );
            }
        }
    }

    Ok(())
}

/// Log when user liabilities exceed the platform's free USDT.
async fn check_solvency(pool: &PgPool, kcc: &KuCoinClient) {
    let liabilities: Decimal =
        match sqlx::query_scalar("SELECT COALESCE(SUM(balance + locked_balance), 0) FROM users")
            .fetch_one(pool)
            .await
        {
            Ok(value) => value,
            Err(e) => {
                log::warn!("[WITHDRAW] solvency query failed: {e}");
                return;
            }
        };

    let main = exchange::main_available(kcc).await.unwrap_or_default();
    let margin = exchange::margin_available(kcc).await.unwrap_or_default();
    let available = main + margin;

    if liabilities > available {
        log::error!(
            "[WITHDRAW] liabilities {liabilities} exceed free USDT {available} (main={main}, margin={margin})"
        );
    }
}

/// Poll `submitted` rows until the exchange reports a terminal state.
async fn reconcile_submitted(pool: &PgPool, kcc: &KuCoinClient) -> Result<(), String> {
    for row in Withdrawals::submitted(pool)
        .await
        .map_err(|e| e.to_string())?
    {
        let Some(exchange_id) = row.kucoin_withdrawal_id.as_deref() else {
            continue;
        };

        let item = match exchange::history_by_id(kcc, exchange_id).await {
            Ok(item) => item,
            Err(e) => {
                log::warn!("[WITHDRAW] {} history lookup failed: {e}", row.id);
                continue;
            }
        };

        match item.status.as_str() {
            "SUCCESS" => {
                if let Ok(reported) = item.amount.parse::<Decimal>() {
                    if reported != row.amount {
                        log::error!(
                            "[WITHDRAW] {} amount mismatch: local={} exchange={reported}; not settling",
                            row.id,
                            row.amount
                        );
                        continue;
                    }
                }

                let actual_fee = item
                    .fee
                    .as_deref()
                    .and_then(|value| value.parse::<Decimal>().ok())
                    .unwrap_or(row.fee);
                if actual_fee < Decimal::ZERO {
                    log::error!(
                        "[WITHDRAW] {} reported a negative fee ({actual_fee}); not settling",
                        row.id
                    );
                    continue;
                }

                Withdrawals::mark_completed(pool, row.id, actual_fee, item.wallet_tx_id.as_deref())
                    .await
                    .map_err(|e| e.to_string())?;
            }
            "FAILURE" => {
                Withdrawals::mark_failed(pool, row.id, "exchange rejected withdrawal")
                    .await
                    .map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }

    Ok(())
}
