//! Withdrawal rows and their balance-impacting transitions.
//!
//! The withdrawal lifecycle is a small explicit state machine:
//! `pending -> processing -> submitted -> completed`, with `failed` and
//! `expired` as terminal branches. Every transition that touches money is a
//! single SQL transaction guarded by the current status, so a row can never be
//! released or settled twice.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::ledger::{self, LedgerKind};
use super::user::User;

/// A withdrawal request and its exchange status.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Withdrawal {
    pub id: Uuid,
    pub user_uid: Uuid,
    pub destination: String,
    pub currency: String,
    pub chain: String,
    pub amount: Decimal,
    pub fee: Decimal,
    pub total: Decimal,
    pub status: String,
    pub nonce: String,
    pub message: String,
    pub signature: String,
    pub expires_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub attempted_amount: Option<Decimal>,
    pub attempted_fee: Option<Decimal>,
    pub kucoin_withdrawal_id: Option<String>,
    pub tx_hash: Option<String>,
    pub actual_fee: Option<Decimal>,
    pub failure_reason: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Failure modes for a capped withdrawal creation.
#[derive(Debug)]
pub enum CreateWithdrawalError {
    Database(sqlx::Error),
    InsufficientFunds,
    PendingLimit,
    DailyLimit,
}

impl From<sqlx::Error> for CreateWithdrawalError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Persistence operations for withdrawals.
pub struct Withdrawals;

impl Withdrawals {
    /// Lock `total` from `wallet` and insert a `pending` withdrawal atomically.
    ///
    /// Uncapped variant kept for tests and internal callers; the API uses
    /// [`Self::create_pending_capped`].
    #[allow(clippy::too_many_arguments)]
    pub async fn create_pending(
        pool: &PgPool,
        user_uid: Uuid,
        wallet: &str,
        amount: Decimal,
        fee: Decimal,
        total: Decimal,
        nonce: &str,
        message: &str,
        signature: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Withdrawal, sqlx::Error> {
        match Self::create_pending_inner(
            pool, user_uid, wallet, amount, fee, total, nonce, message, signature, expires_at, None,
        )
        .await
        {
            Ok(withdrawal) => Ok(withdrawal),
            Err(CreateWithdrawalError::Database(error)) => Err(error),
            Err(CreateWithdrawalError::InsufficientFunds) => Err(sqlx::Error::RowNotFound),
            Err(CreateWithdrawalError::PendingLimit | CreateWithdrawalError::DailyLimit) => {
                Err(sqlx::Error::Protocol("withdrawal limit".into()))
            }
        }
    }

    /// Capped variant: checks pending/daily limits in the same transaction that
    /// locks the user row, so concurrent requests cannot bypass them.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_pending_capped(
        pool: &PgPool,
        user_uid: Uuid,
        wallet: &str,
        amount: Decimal,
        fee: Decimal,
        total: Decimal,
        nonce: &str,
        message: &str,
        signature: &str,
        expires_at: DateTime<Utc>,
        max_pending: i64,
        daily_cap: i64,
    ) -> Result<Withdrawal, CreateWithdrawalError> {
        Self::create_pending_inner(
            pool,
            user_uid,
            wallet,
            amount,
            fee,
            total,
            nonce,
            message,
            signature,
            expires_at,
            Some((max_pending, daily_cap)),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_pending_inner(
        pool: &PgPool,
        user_uid: Uuid,
        wallet: &str,
        amount: Decimal,
        fee: Decimal,
        total: Decimal,
        nonce: &str,
        message: &str,
        signature: &str,
        expires_at: DateTime<Utc>,
        caps: Option<(i64, i64)>,
    ) -> Result<Withdrawal, CreateWithdrawalError> {
        let mut tx = pool.begin().await?;

        let user = sqlx::query_as::<_, User>(
            r#"UPDATE users
               SET balance = balance - $1,
                   locked_balance = locked_balance + $1
               WHERE wallet_address = $2 AND balance >= $1
               RETURNING *"#,
        )
        .bind(total)
        .bind(wallet)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => CreateWithdrawalError::InsufficientFunds,
            other => CreateWithdrawalError::Database(other),
        })?;

        if let Some((max_pending, daily_cap)) = caps {
            let active: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM withdrawals
                   WHERE user_uid = $1
                     AND status IN ('pending', 'processing', 'submitted')"#,
            )
            .bind(user_uid)
            .fetch_one(&mut *tx)
            .await?;
            if active >= max_pending {
                return Err(CreateWithdrawalError::PendingLimit);
            }

            let recent: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM withdrawals
                   WHERE user_uid = $1
                     AND created_at > NOW() - INTERVAL '24 hours'"#,
            )
            .bind(user_uid)
            .fetch_one(&mut *tx)
            .await?;
            if recent >= daily_cap {
                return Err(CreateWithdrawalError::DailyLimit);
            }
        }

        // v1 withdraws only to the authenticated wallet, so `wallet` is the
        // canonical destination. This keeps refunds/settlement independent of
        // the casing the user happened to sign.
        let withdrawal = sqlx::query_as::<_, Withdrawal>(
            r#"INSERT INTO withdrawals
                   (user_uid, destination, amount, fee, total,
                    nonce, message, signature, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING *"#,
        )
        .bind(user_uid)
        .bind(wallet)
        .bind(amount)
        .bind(fee)
        .bind(total)
        .bind(nonce)
        .bind(message)
        .bind(signature)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        ledger::record(
            &mut tx,
            user_uid,
            LedgerKind::WithdrawalLock,
            -total,
            total,
            &user,
            "withdrawal",
            Some(withdrawal.id),
        )
        .await?;

        tx.commit().await?;
        Ok(withdrawal)
    }

    /// Caller's withdrawals, newest first.
    pub async fn list_for_user(
        pool: &PgPool,
        user_uid: Uuid,
    ) -> Result<Vec<Withdrawal>, sqlx::Error> {
        sqlx::query_as::<_, Withdrawal>(
            "SELECT * FROM withdrawals WHERE user_uid = $1 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(user_uid)
        .fetch_all(pool)
        .await
    }

    /// One row by id.
    pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<Withdrawal>, sqlx::Error> {
        sqlx::query_as::<_, Withdrawal>("SELECT * FROM withdrawals WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Claim the oldest eligible `pending` row for execution.
    pub async fn claim_next(pool: &PgPool) -> Result<Option<Withdrawal>, sqlx::Error> {
        sqlx::query_as::<_, Withdrawal>(
            r#"UPDATE withdrawals
               SET status = 'processing',
                   claimed_at = NOW(),
                   updated_at = NOW()
               WHERE id = (
                   SELECT id FROM withdrawals
                   WHERE status = 'pending' AND expires_at > NOW()
                   ORDER BY created_at
                   LIMIT 1
               ) AND status = 'pending'
               RETURNING *"#,
        )
        .fetch_optional(pool)
        .await
    }

    /// Expire rows that were never attempted and release their locks.
    ///
    /// A `processing` row is only eligible after `claim_timeout_secs`, so a
    /// worker that is actively between claim and attempt cannot have its funds
    /// refunded underneath it.
    pub async fn expire_safe(
        pool: &PgPool,
        claim_timeout_secs: i64,
    ) -> Result<Vec<Withdrawal>, sqlx::Error> {
        let mut tx = pool.begin().await?;

        let candidates = sqlx::query_as::<_, Withdrawal>(
            r#"SELECT * FROM withdrawals
               WHERE expires_at < NOW()
                 AND (status = 'pending'
                      OR (status = 'processing'
                          AND attempted_at IS NULL
                          AND claimed_at < NOW() - make_interval(secs => $1)))
               FOR UPDATE"#,
        )
        .bind(claim_timeout_secs)
        .fetch_all(&mut *tx)
        .await?;

        let mut expired = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let row = sqlx::query_as::<_, Withdrawal>(
                r#"UPDATE withdrawals
                   SET status = 'expired',
                       failure_reason = 'expired',
                       updated_at = NOW()
                   WHERE id = $1 AND status IN ('pending', 'processing')
                   RETURNING *"#,
            )
            .bind(candidate.id)
            .fetch_one(&mut *tx)
            .await?;

            release_lock(&mut tx, row.user_uid, row.total, row.id).await?;

            expired.push(row);
        }

        tx.commit().await?;
        Ok(expired)
    }

    /// Return a claimed-but-not-attempted row to `pending` for a later pass.
    ///
    /// Used after a transient failure (quote, liquidity, transfer) where no
    /// exchange call was made, so the funds stay locked and the row is retried.
    pub async fn release_unattempted(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE withdrawals
               SET status = 'pending', claimed_at = NULL, updated_at = NOW()
               WHERE id = $1 AND status = 'processing' AND attempted_at IS NULL"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Reset an attempted row that the exchange definitively did not accept.
    ///
    /// Used for retryable responses such as 429/401/403. The next pass claims
    /// it again; funds stay locked throughout.
    pub async fn release_attempt(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE withdrawals
               SET status = 'pending',
                   claimed_at = NULL,
                   attempted_at = NULL,
                   attempted_amount = NULL,
                   attempted_fee = NULL,
                   updated_at = NOW()
               WHERE id = $1 AND status = 'processing' AND attempted_at IS NOT NULL"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a claimed-but-never-attempted row expired and release its lock.
    pub async fn mark_expired(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, Withdrawal>(
            r#"UPDATE withdrawals
               SET status = 'expired',
                   failure_reason = 'expired',
                   updated_at = NOW()
               WHERE id = $1 AND status IN ('pending', 'processing')
                 AND attempted_at IS NULL
               RETURNING *"#,
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            release_lock(&mut tx, row.user_uid, row.total, row.id).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Rows whose create call may have reached the exchange but is unconfirmed.
    pub async fn processing_attempted(pool: &PgPool) -> Result<Vec<Withdrawal>, sqlx::Error> {
        sqlx::query_as::<_, Withdrawal>(
            r#"SELECT * FROM withdrawals
               WHERE status = 'processing' AND attempted_at IS NOT NULL
               ORDER BY attempted_at"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Rows accepted by the exchange and awaiting confirmation.
    pub async fn submitted(pool: &PgPool) -> Result<Vec<Withdrawal>, sqlx::Error> {
        sqlx::query_as::<_, Withdrawal>(
            "SELECT * FROM withdrawals WHERE status = 'submitted' ORDER BY updated_at",
        )
        .fetch_all(pool)
        .await
    }

    /// Count non-terminal withdrawals for `user_uid` (pending cap).
    pub async fn count_active_for_user(pool: &PgPool, user_uid: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM withdrawals
               WHERE user_uid = $1 AND status IN ('pending', 'processing', 'submitted')"#,
        )
        .bind(user_uid)
        .fetch_one(pool)
        .await
    }

    /// Count withdrawals created by `user_uid` in the last `hours` (daily cap).
    pub async fn count_since_for_user(
        pool: &PgPool,
        user_uid: Uuid,
        hours: i64,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM withdrawals
               WHERE user_uid = $1
                 AND created_at > NOW() - make_interval(hours => $2::int)"#,
        )
        .bind(user_uid)
        .bind(hours)
        .fetch_one(pool)
        .await
    }

    /// Persist the intent to call the exchange, before the network call.
    ///
    /// Returns `false` when the row is no longer `processing`; callers must not
    /// call the exchange in that case.
    pub async fn mark_attempt(
        pool: &PgPool,
        id: Uuid,
        amount: Decimal,
        fee: Decimal,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE withdrawals
               SET attempted_at = NOW(),
                   attempted_amount = $1,
                   attempted_fee = $2,
                   updated_at = NOW()
               WHERE id = $3 AND status = 'processing'"#,
        )
        .bind(amount)
        .bind(fee)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Record a successful create call.
    ///
    /// Returns `false` when the row moved out of `processing` in the meantime;
    /// callers must surface that as an error.
    pub async fn mark_submitted(
        pool: &PgPool,
        id: Uuid,
        kucoin_withdrawal_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE withdrawals
               SET status = 'submitted',
                   kucoin_withdrawal_id = $1,
                   submitted_at = NOW(),
                   updated_at = NOW()
               WHERE id = $2 AND status = 'processing'"#,
        )
        .bind(kucoin_withdrawal_id)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Mark a non-submitted row failed and release its lock exactly once.
    pub async fn mark_failed(pool: &PgPool, id: Uuid, reason: &str) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, Withdrawal>(
            r#"UPDATE withdrawals
               SET status = 'failed',
                   failure_reason = $1,
                   updated_at = NOW()
               WHERE id = $2 AND status IN ('pending', 'processing', 'submitted')
               RETURNING *"#,
        )
        .bind(reason)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            release_lock(&mut tx, row.user_uid, row.total, row.id).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Mark a submitted row completed, settle the actual fee, consume the lock.
    ///
    /// `fee - actual_fee` is refunded to available balance when positive and
    /// debited when negative. A negative delta larger than the user's balance is
    /// absorbed by the platform rather than failing a completed withdrawal.
    pub async fn mark_completed(
        pool: &PgPool,
        id: Uuid,
        actual_fee: Decimal,
        tx_hash: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, Withdrawal>(
            "SELECT * FROM withdrawals WHERE id = $1 AND status = 'submitted' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(());
        };

        // Lock the user row so the platform-absorb clamp cannot race another
        // settlement for the same user.
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE uid = $1 FOR UPDATE")
            .bind(row.user_uid)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let fee_delta = row.fee - actual_fee;
        let applied_delta = if fee_delta < Decimal::ZERO {
            fee_delta.max(-user.balance)
        } else {
            fee_delta
        };
        if applied_delta != fee_delta {
            log::error!(
                "[WITHDRAW] platform absorbed {} of fee delta on {}",
                fee_delta - applied_delta,
                row.id
            );
        }

        let user = sqlx::query_as::<_, User>(
            r#"UPDATE users
               SET locked_balance = locked_balance - $1,
                   balance = balance + $2
               WHERE uid = $3 AND locked_balance >= $1
               RETURNING *"#,
        )
        .bind(row.total)
        .bind(applied_delta)
        .bind(row.user_uid)
        .fetch_one(&mut *tx)
        .await?;

        ledger::record(
            &mut tx,
            row.user_uid,
            LedgerKind::WithdrawalSettle,
            applied_delta,
            -row.total,
            &user,
            "withdrawal",
            Some(row.id),
        )
        .await?;

        sqlx::query(
            r#"UPDATE withdrawals
               SET status = 'completed',
                   actual_fee = $1,
                   tx_hash = $2,
                   completed_at = NOW(),
                   updated_at = NOW()
               WHERE id = $3 AND status = 'submitted'"#,
        )
        .bind(actual_fee)
        .bind(tx_hash)
        .bind(row.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

/// Return `total` from locked to available and journal the refund.
async fn release_lock(
    tx: &mut Transaction<'_, Postgres>,
    user_uid: Uuid,
    total: Decimal,
    withdrawal_id: Uuid,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"UPDATE users
           SET balance = balance + $1,
               locked_balance = locked_balance - $1
           WHERE uid = $2 AND locked_balance >= $1
           RETURNING *"#,
    )
    .bind(total)
    .bind(user_uid)
    .fetch_one(&mut **tx)
    .await?;

    ledger::record(
        tx,
        user_uid,
        LedgerKind::WithdrawalRefund,
        total,
        -total,
        &user,
        "withdrawal",
        Some(withdrawal_id),
    )
    .await?;

    Ok(user)
}
