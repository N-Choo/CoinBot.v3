//! Append-only journal of every balance mutation.
//!
//! Each row is written in the same transaction as the `users` update it
//! describes, so `balance` and `locked_balance` can always be reconstructed
//! from the journal plus its starting point.

use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::user::User;

/// What caused a balance mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerKind {
    Deposit,
    ContractLock,
    ContractPayout,
    WithdrawalLock,
    WithdrawalRefund,
    WithdrawalSettle,
}

impl LedgerKind {
    /// Stable string stored in `ledger_entries.kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::ContractLock => "contract_lock",
            Self::ContractPayout => "contract_payout",
            Self::WithdrawalLock => "withdrawal_lock",
            Self::WithdrawalRefund => "withdrawal_refund",
            Self::WithdrawalSettle => "withdrawal_settle",
        }
    }
}

/// Append one entry after the corresponding `users` update in the same `tx`.
///
/// `user` must be the row returned by that update so the balance snapshots are
/// exact.
#[allow(clippy::too_many_arguments)]
pub async fn record(
    tx: &mut Transaction<'_, Postgres>,
    user_uid: Uuid,
    kind: LedgerKind,
    delta_available: Decimal,
    delta_locked: Decimal,
    user: &User,
    ref_type: &str,
    ref_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO ledger_entries
               (user_uid, kind, delta_available, delta_locked,
                balance_after, locked_after, ref_type, ref_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(user_uid)
    .bind(kind.as_str())
    .bind(delta_available)
    .bind(delta_locked)
    .bind(user.balance)
    .bind(user.locked_balance)
    .bind(ref_type)
    .bind(ref_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::user::User;
    use sqlx::PgPool;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url)
            .await
            .expect("DATABASE_URL is set but Postgres is unreachable");
        sqlx::migrate!("../../process/migrations")
            .run(&pool)
            .await
            .expect("database migrations failed");
        Some(pool)
    }

    #[tokio::test]
    async fn record_appends_an_entry() {
        let Some(pool) = pool().await else { return };
        let wallet = format!("0xledger{}", uuid::Uuid::new_v4().simple());

        let mut tx = pool.begin().await.unwrap();
        let user = User::upsert(&mut tx, &wallet).await.unwrap();
        let user = user
            .add_balance(&mut tx, rust_decimal::Decimal::from(10))
            .await
            .unwrap();
        record(
            &mut tx,
            user.uid,
            LedgerKind::Deposit,
            rust_decimal::Decimal::from(10),
            rust_decimal::Decimal::ZERO,
            &user,
            "deposit",
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let (kind, delta, after): (String, rust_decimal::Decimal, rust_decimal::Decimal) =
            sqlx::query_as(
                "SELECT kind, delta_available, balance_after
                   FROM ledger_entries WHERE user_uid = $1",
            )
            .bind(user.uid)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "deposit");
        assert_eq!(delta, rust_decimal::Decimal::from(10));
        assert_eq!(after, rust_decimal::Decimal::from(10));

        cleanup(&pool, user.uid).await;
    }

    async fn cleanup(pool: &PgPool, uid: uuid::Uuid) {
        sqlx::query("DELETE FROM ledger_entries WHERE user_uid = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE uid = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
    }
}
