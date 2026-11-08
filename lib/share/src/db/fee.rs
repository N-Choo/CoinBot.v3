//! Platform fee ledger.

use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Insert a platform fee exactly once per `(contract_id, kind)`.
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    contract_id: Uuid,
    user_uid: Uuid,
    amount: Decimal,
    kind: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO platform_fees (contract_id, user_uid, amount, kind)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (contract_id, kind) DO NOTHING"#,
    )
    .bind(contract_id)
    .bind(user_uid)
    .bind(amount)
    .bind(kind)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
