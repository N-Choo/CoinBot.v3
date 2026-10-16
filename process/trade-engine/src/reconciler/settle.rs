//! Settle orders the exchange reports as final.
//!
//! Reads each open order's exchange state and decides the consequence: a fill
//! or a cancel. The realized-PnL math lives in `pnl` and the DB writes in
//! `ledger`.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::margin::MarginOrderStatus;
use rust_decimal::Decimal;
use share::db::trade::{Status, TradeOrder, TradeOrderFilter, TradeOrders};
use sqlx::{Pool, Postgres};

use super::exchange::{OrderOutcome, classify_order, fetch_order};
use super::pnl::{cap_loss, close_credit, fee_in_quote, realized_pnl, split_symbol};
use crate::ledger::{self, Fill};
use crate::symbol::margin_symbol;

/// Settle every order the exchange reports as open.
pub(super) async fn settle_open_orders(db: &Pool<Postgres>, kcc: &KuCoinClient) {
    let orders = match TradeOrderFilter::new()
        .with_status(Status::Open)
        .execute(db)
        .await
    {
        Ok(orders) => orders,
        Err(e) => {
            log::error!("[RECONCILE] open-orders query failed: {e}");
            return;
        }
    };

    for o in orders {
        let Some(order_id) = o.exchange_order_id.as_deref() else {
            continue;
        };
        let symbol = margin_symbol(&o.ticker);

        match fetch_order(kcc, order_id, &symbol).await {
            Ok(Some(data)) => match classify_order(data.active, &data.deal_size) {
                OrderOutcome::Working => continue,
                OrderOutcome::Cancelled => settle_cancelled(db, &o).await,
                OrderOutcome::Filled => settle_filled(db, &o, &symbol, &data).await,
            },
            Ok(None) => continue,
            Err(e) => {
                log::warn!("[RECONCILE] get_order failed for order {}: {e}", o.id);
                continue;
            }
        }
    }
}

/// Settle an order that finished with no fill.
async fn settle_cancelled(db: &Pool<Postgres>, o: &TradeOrder) {
    if let Err(e) = ledger::record_cancelled(db, o).await {
        log::error!("[RECONCILE] cancel settle failed for order {}: {e}", o.id);
    }
}

/// Settle a filled order. Amounts are parsed strictly; a malformed field aborts
/// the settle (retried next cycle) rather than committing a bogus 0 credit.
async fn settle_filled(
    db: &Pool<Postgres>,
    o: &TradeOrder,
    symbol: &str,
    data: &MarginOrderStatus,
) {
    let (Ok(deal_size), Ok(deal_funds), Ok(fee)) = (
        data.deal_size.parse::<Decimal>(),
        data.deal_funds.parse::<Decimal>(),
        data.fee.parse::<Decimal>(),
    ) else {
        log::warn!(
            "[RECONCILE] order {} has unparseable amounts; retrying",
            o.id
        );
        return;
    };
    let fill_price = if deal_size.is_zero() {
        Decimal::ZERO
    } else {
        deal_funds / deal_size
    };
    let (base, quote) = split_symbol(symbol);
    // Fee for THIS (exit) leg, expressed in the contract's quote currency.
    let exit_fee_q = fee_in_quote(fee, data.fee_currency.as_deref(), &base, &quote, fill_price);

    // For a close, derive realized PnL from the parent entry: its actual
    // transacted basis (filled_qty * fill_price), its fee, and this leg's fee.
    // Computed before opening the tx so the parent lookup does not borrow a
    // second pool connection while the tx holds one.
    let (credit, pnl) = if o.action == "close" {
        match o.parent_order_id {
            None => (deal_funds, None),
            Some(pid) => {
                let entry = match TradeOrders::find(db, pid).await {
                    Ok(entry) => entry,
                    Err(e) => {
                        log::error!("[RECONCILE] parent lookup failed for close {}: {e}", o.id);
                        return; // retry next cycle
                    }
                };
                let entry_price = entry.fill_price.unwrap_or(Decimal::ZERO);
                // Prefer the entry's real fill for the cost basis; fall back to
                // the committed fund (never a silent zero) when it is missing.
                let entry_basis = if !entry.filled_qty.is_zero() && entry.fill_price.is_some() {
                    entry.filled_qty * entry_price
                } else {
                    log::warn!(
                        "[RECONCILE] parent entry {} lacks a real fill; using committed fund as basis",
                        entry.id
                    );
                    entry.funds
                };
                let entry_fee_q = fee_in_quote(
                    entry.fee,
                    entry.fee_currency.as_deref(),
                    &base,
                    &quote,
                    entry_price,
                );
                let raw_pnl = realized_pnl(
                    &entry.side,
                    entry_basis,
                    deal_funds,
                    entry_fee_q + exit_fee_q,
                );
                let pnl = cap_loss(raw_pnl, entry.funds);
                if pnl != raw_pnl {
                    log::warn!(
                        "[RECONCILE] close {} raw pnl {raw_pnl} exceeds committed capital {}; capping",
                        o.id,
                        entry.funds
                    );
                }
                (close_credit(entry.funds, pnl), Some(pnl))
            }
        }
    } else {
        (Decimal::ZERO, None)
    };

    let fill = Fill {
        deal_size,
        fill_price,
        fee,
        fee_currency: data.fee_currency.as_deref(),
        pnl,
    };
    // Only an exit returns cash.
    let credit_to_add = (o.action == "close").then(|| credit.max(Decimal::ZERO));
    if let Err(e) = ledger::record_filled(db, o, &fill, credit_to_add).await {
        log::error!("[RECONCILE] settle filled failed for order {}: {e}", o.id);
        return;
    }
    log::info!(
        "[RECONCILE] {symbol} order {} filled {} @ {fill_price} (fee {fee}, pnl {})",
        o.id,
        data.deal_size,
        pnl.map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
}

#[cfg(test)]
mod tests {
    //! DB-backed tests for exchange-reported settlement of orders.
    //!
    //! `settle_filled`/`settle_cancelled` stay private; these inline tests call
    //! them directly with a hand-built `MarginOrderStatus` (no network). Require
    //! `DATABASE_URL`; each test skips when it is unset/unreachable and cleans up
    //! the rows it creates.

    use super::*;
    use crate::ledger;
    use share::db::contracts::{Contract, ContractFilter, Contracts};
    use share::db::user::User;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        sqlx::migrate!("../migrations").run(&pool).await.ok()?;
        Some(pool)
    }

    struct Fixture {
        pool: PgPool,
        wallet: String,
        contract: Contract,
    }

    impl Fixture {
        async fn reload_contract(&self) -> Contract {
            ContractFilter::new()
                .with_user(self.contract.user_uid)
                .execute(&self.pool)
                .await
                .unwrap()
                .into_iter()
                .next()
                .unwrap()
        }

        async fn cleanup(&self) {
            sqlx::query("DELETE FROM trade_orders WHERE contract_id = $1")
                .bind(self.contract.id)
                .execute(&self.pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM contracts WHERE id = $1")
                .bind(self.contract.id)
                .execute(&self.pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM users WHERE wallet_address = $1")
                .bind(&self.wallet)
                .execute(&self.pool)
                .await
                .unwrap();
        }
    }

    async fn setup(init_fund: Decimal) -> Option<Fixture> {
        let pool = test_pool().await?;
        let wallet = format!("0xtest{}", Uuid::new_v4().simple());
        let mut tx = pool.begin().await.unwrap();
        let user = User::upsert(&mut tx, &wallet).await.unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        Contracts::create(
            &mut tx,
            user.uid,
            "0xsig",
            "msg",
            &format!("nonce-{}", Uuid::new_v4()),
            "BTC/USDT",
            init_fund,
            init_fund,
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
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        Some(Fixture {
            pool,
            wallet,
            contract,
        })
    }

    fn comment(prefix: &str) -> String {
        format!("{prefix}-{}", Uuid::new_v4())
    }

    /// Place (`open`) and fully fill an entry order that reserved `fund`.
    async fn filled_entry(fx: &Fixture, side: &str, fund: Decimal) -> TradeOrder {
        let entry = ledger::create_pending_open(
            &fx.pool,
            &fx.contract,
            side,
            &comment("entry"),
            Decimal::from(1),
            fund,
            Decimal::ZERO,
            Decimal::ZERO,
        )
        .await
        .unwrap();

        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
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
        TradeOrders::find(&fx.pool, entry.id).await.unwrap()
    }

    /// Reserve, place, and return a live close order linked to `entry`.
    async fn live_exit(fx: &Fixture, entry: &TradeOrder, side: &str) -> TradeOrder {
        let close = TradeOrders::reserve_exit(
            &fx.pool,
            entry,
            side,
            &comment("exit"),
            Decimal::from(1),
            Decimal::ZERO,
        )
        .await
        .unwrap()
        .expect("fresh exit reserved");

        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, close.id, "exch-exit")
            .await
            .unwrap();
        tx.commit().await.unwrap();
        TradeOrders::find(&fx.pool, close.id).await.unwrap()
    }

    fn status(deal_size: &str, deal_funds: &str, fee: &str) -> MarginOrderStatus {
        MarginOrderStatus {
            active: false,
            deal_size: deal_size.to_string(),
            deal_funds: deal_funds.to_string(),
            fee: fee.to_string(),
            fee_currency: Some("USDT".to_string()),
        }
    }

    /// A profitable long close: pnl 18, contract credited committed fund + pnl.
    #[tokio::test]
    async fn settle_filled_close_credits_entry_fund_plus_pnl() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = filled_entry(&fx, "buy", Decimal::from(100)).await;
        let close = live_exit(&fx, &entry, "sell").await;

        settle_filled(&fx.pool, &close, "BTC-USDT", &status("1", "120", "2")).await;

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "filled");
        assert_eq!(stored.pnl, Some(Decimal::from(18)));
        assert_eq!(stored.filled_qty, Decimal::from(1));
        assert_eq!(stored.fill_price, Some(Decimal::from(120)));

        // 1000 - 100 reserved on the entry + 118 credited on the close.
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1018));

        fx.cleanup().await;
    }

    /// A short whose loss exceeds the committed fund is capped at the fund, so
    /// no cash is credited (credit floored at zero).
    #[tokio::test]
    async fn settle_filled_close_caps_loss_at_committed_fund() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = filled_entry(&fx, "sell", Decimal::from(100)).await;
        let close = live_exit(&fx, &entry, "buy").await;

        // Sold at 100 (basis), bought back at 250: raw pnl -150, capped to -100.
        settle_filled(&fx.pool, &close, "BTC-USDT", &status("1", "250", "0")).await;

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "filled");
        assert_eq!(stored.pnl, Some(Decimal::from(-100)), "loss capped at fund");

        let contract = fx.reload_contract().await;
        assert_eq!(
            contract.available_fund,
            Decimal::from(900),
            "no credit when the loss consumes the committed fund"
        );

        fx.cleanup().await;
    }

    /// A filled entry credits nothing and keeps a NULL pnl.
    #[tokio::test]
    async fn settle_filled_entry_credits_nothing_and_leaves_pnl_null() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = ledger::create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &comment("entry"),
            Decimal::from(1),
            Decimal::from(100),
            Decimal::ZERO,
            Decimal::ZERO,
        )
        .await
        .unwrap();
        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let entry = TradeOrders::find(&fx.pool, entry.id).await.unwrap();

        settle_filled(&fx.pool, &entry, "BTC-USDT", &status("1", "100", "0.1")).await;

        let stored = TradeOrders::find(&fx.pool, entry.id).await.unwrap();
        assert_eq!(stored.status, "filled");
        assert_eq!(stored.fill_price, Some(Decimal::from(100)));
        assert!(stored.pnl.is_none(), "entries leave pnl NULL");

        let contract = fx.reload_contract().await;
        assert_eq!(
            contract.available_fund,
            Decimal::from(900),
            "entry fill moves no cash"
        );

        fx.cleanup().await;
    }

    /// A cancelled entry releases its reserved fund.
    #[tokio::test]
    async fn settle_cancelled_entry_releases_fund() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = ledger::create_pending_open(
            &fx.pool,
            &fx.contract,
            "buy",
            &comment("entry"),
            Decimal::from(1),
            Decimal::from(100),
            Decimal::ZERO,
            Decimal::ZERO,
        )
        .await
        .unwrap();
        let mut tx = fx.pool.begin().await.unwrap();
        TradeOrders::mark_placed(&mut tx, entry.id, "exch-entry")
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let entry = TradeOrders::find(&fx.pool, entry.id).await.unwrap();

        settle_cancelled(&fx.pool, &entry).await;

        let stored = TradeOrders::find(&fx.pool, entry.id).await.unwrap();
        assert_eq!(stored.status, "cancelled");
        let contract = fx.reload_contract().await;
        assert_eq!(contract.available_fund, Decimal::from(1000));
        assert_eq!(contract.total_traded_fund, Decimal::ZERO);

        fx.cleanup().await;
    }

    /// A cancelled exit changes no funds.
    #[tokio::test]
    async fn settle_cancelled_exit_changes_no_funds() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = filled_entry(&fx, "buy", Decimal::from(100)).await;
        let close = live_exit(&fx, &entry, "sell").await;
        let before = fx.reload_contract().await;

        settle_cancelled(&fx.pool, &close).await;

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(stored.status, "cancelled");
        let after = fx.reload_contract().await;
        assert_eq!(after.available_fund, before.available_fund);
        assert_eq!(after.total_traded_fund, before.total_traded_fund);

        fx.cleanup().await;
    }

    /// Unparseable fill amounts abort the settle without touching the DB, so the
    /// order stays open for the next reconcile cycle.
    #[tokio::test]
    async fn settle_filled_aborts_on_unparseable_amounts() {
        let Some(fx) = setup(Decimal::from(1000)).await else {
            return;
        };
        let entry = filled_entry(&fx, "buy", Decimal::from(100)).await;
        let close = live_exit(&fx, &entry, "sell").await;
        let before = fx.reload_contract().await;

        settle_filled(&fx.pool, &close, "BTC-USDT", &status("abc", "xyz", "0")).await;

        let stored = TradeOrders::find(&fx.pool, close.id).await.unwrap();
        assert_eq!(
            stored.status, "open",
            "aborted settle leaves the order open"
        );
        assert!(stored.pnl.is_none());
        let after = fx.reload_contract().await;
        assert_eq!(after.available_fund, before.available_fund);
        assert_eq!(after.total_traded_fund, before.total_traded_fund);

        fx.cleanup().await;
    }
}
