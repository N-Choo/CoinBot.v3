//! Close a `closing` contract's positions and finalize its payout.

use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use log::{error, info};
use share::db::contracts::{Contract, ContractFilter, Contracts, Status};
use share::db::fee;
use share::db::loan::MarginLoan;
use share::db::trade::TradeOrders;
use share::db::user::User;
use sqlx::PgPool;
use uuid::Uuid;

use crate::position_exit;
use crate::settlement::{fee as fee_math, interest, repay};
use crate::symbol::margin_symbol;

const POLL_SECS: u64 = 12;

/// Background loop.
pub async fn run(pool: PgPool, kcc: KuCoinClient) {
    let mut backoff = 0u64;
    loop {
        match sweep(&pool, &kcc).await {
            Ok(()) => backoff = 0,
            Err(e) => {
                error!("[SETTLE] pass failed: {e}");
                if crate::kucoin_margin::is_rate_limited(&e) {
                    backoff = if backoff == 0 {
                        60
                    } else {
                        (backoff * 2).min(300)
                    };
                    log::warn!("[SETTLE] backing off {backoff}s");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS.max(backoff))).await;
    }
}

/// Process every contract in `closing`, isolating per-contract failures.
pub async fn sweep(pool: &PgPool, kcc: &KuCoinClient) -> Result<(), String> {
    let contracts = ContractFilter::new()
        .with_status(Status::Closing)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    for c in contracts {
        if let Err(e) = settle(pool, kcc, &c).await {
            error!("[SETTLE] contract {} failed: {e}", c.id);
            if crate::kucoin_margin::is_rate_limited(&e) {
                return Err(e);
            }
        }
    }
    Ok(())
}

async fn settle(pool: &PgPool, kcc: &KuCoinClient, c: &Contract) -> Result<(), String> {
    let held = TradeOrders::held_entries_for_contract(pool, c.id)
        .await
        .map_err(|e| e.to_string())?;
    if !held.is_empty() {
        let symbols: Vec<String> = held.iter().map(|e| margin_symbol(&e.ticker)).collect();
        let prices = position_exit::last_prices(kcc, &symbols).await?;
        for entry in &held {
            let symbol = margin_symbol(&entry.ticker);
            let last = *prices
                .get(&symbol)
                .ok_or_else(|| format!("no price for {symbol}"))?;
            position_exit::close_at_market(pool, kcc, entry, last)
                .await
                .map_err(|e| format!("close {}: {e}", entry.id))?;
        }
        return Ok(());
    }

    if TradeOrders::has_unsettled_for_contract(pool, c.id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(());
    }

    let currencies: Vec<String> = MarginLoan::unrepaid_for_contract(pool, c.id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|l| l.currency)
        .collect();
    for currency in currencies {
        interest::sweep_currency(pool, kcc, &currency).await?;
    }
    repay::repay_pending(pool, kcc, Some(c.id)).await?;

    if !MarginLoan::unrepaid_for_contract(pool, c.id)
        .await
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        return Ok(());
    }

    finalize(pool, c.id).await
}

/// One transactional payout. Safe to call repeatedly.
pub async fn finalize(pool: &PgPool, contract_id: Uuid) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let Some(c) = Contracts::find_for_update(&mut tx, contract_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    if c.status != "closing" {
        return Ok(());
    }
    if TradeOrders::has_unsettled_for_contract_tx(&mut tx, c.id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(());
    }
    if TradeOrders::has_open_position_for_contract_tx(&mut tx, c.id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(());
    }
    if !MarginLoan::unrepaid_for_contract_tx(&mut tx, c.id)
        .await
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        return Ok(());
    }

    let payout = fee_math::compute(c.available_fund, c.borrow_interest, c.init_fund);
    let wallet = User::wallet_for_uid_tx(&mut tx, c.user_uid)
        .await
        .map_err(|e| e.to_string())?;
    User::release_locked_and_credit(&mut tx, &wallet, c.init_fund, payout.payout)
        .await
        .map_err(|e| format!("release locked failed: {e}"))?;
    let target = c.settlement_target.as_deref().unwrap_or("cancelled");
    Contracts::finalize(&mut tx, c.id, target, payout.platform_fee, payout.payout)
        .await
        .map_err(|e| e.to_string())?;
    if payout.platform_fee > rust_decimal::Decimal::ZERO {
        fee::insert(&mut tx, c.id, c.user_uid, payout.platform_fee, "profit")
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    info!(
        "[SETTLE] contract {} settled payout={} fee={}",
        c.id, payout.payout, payout.platform_fee
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use share::db::contracts::{ContractFilter, Contracts};
    use share::db::user::User;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        sqlx::migrate!("../migrations").run(&pool).await.ok()?;
        Some(pool)
    }

    #[tokio::test]
    async fn finalize_credits_once_and_releases_locked() {
        let Some(pool) = pool().await else {
            return;
        };
        let wallet = format!("0xtest{}", Uuid::new_v4().simple());
        let mut tx = pool.begin().await.unwrap();
        let user = User::upsert(&mut tx, &wallet).await.unwrap();
        let user = user
            .add_balance(&mut tx, Decimal::from(1000))
            .await
            .unwrap();
        user.lock_balance(&mut tx, Decimal::from(1000))
            .await
            .unwrap();
        Contracts::create(
            &mut tx,
            user.uid,
            "sig",
            "msg",
            &format!("n-{}", Uuid::new_v4()),
            "BTC/USDT",
            Decimal::from(1000),
            Decimal::from(1180),
            Decimal::ZERO,
            2.0,
            5.0,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let contract = ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
            .unwrap()[0]
            .clone();
        sqlx::query(
            "UPDATE contracts SET status='closing', settlement_target='cancelled', borrow_interest=20 WHERE id=$1",
        )
        .bind(contract.id)
        .execute(&pool)
        .await
        .unwrap();

        finalize(&pool, contract.id).await.unwrap();
        finalize(&pool, contract.id).await.unwrap();

        let after = User::find_by_wallet(&pool, &wallet).await.unwrap().unwrap();
        assert_eq!(after.balance, Decimal::from(1152));
        assert_eq!(after.locked_balance, Decimal::ZERO);
        let c = ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
            .unwrap()[0]
            .clone();
        assert_eq!(c.status, "cancelled");
        assert_eq!(c.platform_fee, Decimal::from(8));
        assert_eq!(c.net_payout, Some(Decimal::from(1152)));

        sqlx::query("DELETE FROM platform_fees WHERE contract_id=$1")
            .bind(contract.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM contracts WHERE id=$1")
            .bind(contract.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE wallet_address=$1")
            .bind(&wallet)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn finalize_refuses_an_open_position() {
        let Some(pool) = pool().await else {
            return;
        };
        let wallet = format!("0xtest{}", Uuid::new_v4().simple());
        let mut tx = pool.begin().await.unwrap();
        let user = User::upsert(&mut tx, &wallet).await.unwrap();
        let user = user
            .add_balance(&mut tx, Decimal::from(1000))
            .await
            .unwrap();
        user.lock_balance(&mut tx, Decimal::from(1000))
            .await
            .unwrap();
        Contracts::create(
            &mut tx,
            user.uid,
            "sig",
            "msg",
            &format!("n-{}", Uuid::new_v4()),
            "BTC/USDT",
            Decimal::from(1000),
            Decimal::from(1000),
            Decimal::ZERO,
            2.0,
            5.0,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let contract = ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
            .unwrap()[0]
            .clone();

        let entry = crate::ledger::create_pending_open(
            &pool,
            &contract,
            "buy",
            &format!("c-{}", Uuid::new_v4()),
            Decimal::from(1),
            Decimal::from(100),
            None,
            None,
        )
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry", None, None)
            .await
            .unwrap();
        TradeOrders::mark_filled(
            &mut tx,
            entry.id,
            Decimal::from(1),
            Decimal::from(100),
            Decimal::ZERO,
            None,
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        sqlx::query(
            "UPDATE contracts SET status='closing', settlement_target='cancelled' WHERE id=$1",
        )
        .bind(contract.id)
        .execute(&pool)
        .await
        .unwrap();

        finalize(&pool, contract.id).await.unwrap();

        let after = User::find_by_wallet(&pool, &wallet).await.unwrap().unwrap();
        assert_eq!(
            after.balance,
            Decimal::ZERO,
            "no payout with an open position"
        );
        assert_eq!(after.locked_balance, Decimal::from(1000));
        let c = ContractFilter::new()
            .with_user(user.uid)
            .execute(&pool)
            .await
            .unwrap()[0]
            .clone();
        assert_eq!(c.status, "closing");

        sqlx::query("DELETE FROM trade_orders WHERE contract_id=$1")
            .bind(contract.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM contracts WHERE id=$1")
            .bind(contract.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE wallet_address=$1")
            .bind(&wallet)
            .execute(&pool)
            .await
            .unwrap();
    }
}
