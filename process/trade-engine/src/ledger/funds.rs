//! Fund movements on a contract's balance.
//!
//! Transaction-scoped helpers: they never open a transaction, so the caller in
//! `orders` decides what commits together. Private to the ledger.

use rust_decimal::Decimal;
use share::db::contracts::Contracts;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::Result;

/// Reserve `amount` from the contract's available fund (on order placement).
pub(super) async fn reserve(
    tx: &mut Transaction<'_, Postgres>,
    contract_id: Uuid,
    amount: Decimal,
) -> Result<()> {
    Contracts::consume_fund(tx, contract_id, amount).await?;
    Ok(())
}

/// Return an unused reservation of `amount` to the available fund.
pub(super) async fn release(
    tx: &mut Transaction<'_, Postgres>,
    contract_id: Uuid,
    amount: Decimal,
) -> Result<()> {
    Contracts::release_fund(tx, contract_id, amount).await?;
    Ok(())
}

/// Finalize a close: credit `amount` (committed fund plus realized PnL, floored at
/// zero by the caller) back to the contract's available fund.
pub(super) async fn finalize(
    tx: &mut Transaction<'_, Postgres>,
    contract_id: Uuid,
    amount: Decimal,
) -> Result<()> {
    Contracts::add_fund(tx, contract_id, amount).await?;
    Ok(())
}
