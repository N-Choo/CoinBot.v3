//! Per-contract margin loans and KuCoin interest charges.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// One short entry's base-asset borrow.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MarginLoan {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub entry_order_id: Uuid,
    pub currency: String,
    pub principal: Decimal,
    pub outstanding: Decimal,
    pub loan_apply_id: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub position_closed_at: Option<DateTime<Utc>>,
    pub interest_base: Decimal,
    pub interest_usdt: Decimal,
    pub repay_claimed_at: Option<DateTime<Utc>>,
    pub repay_buy_order_id: Option<String>,
    pub repay_buy_client_oid: Option<String>,
    pub repay_attempted_at: Option<DateTime<Utc>>,
    pub repay_attempted_amount: Option<Decimal>,
    pub repay_attempted_cost_usdt: Option<Decimal>,
    pub repay_order_no: Option<String>,
    pub repaid_at: Option<DateTime<Utc>>,
    pub repaid_cost_usdt: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl MarginLoan {
    /// Insert a loan for a filled short entry.
    pub async fn create(
        tx: &mut Transaction<'_, Postgres>,
        contract_id: Uuid,
        entry_order_id: Uuid,
        currency: &str,
        principal: Decimal,
        loan_apply_id: Option<&str>,
        opened_at: DateTime<Utc>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO margin_loans
                   (contract_id, entry_order_id, currency, principal, outstanding, loan_apply_id, opened_at)
               VALUES ($1, $2, $3, $4, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(contract_id)
        .bind(entry_order_id)
        .bind(currency)
        .bind(principal)
        .bind(loan_apply_id)
        .bind(opened_at)
        .fetch_one(&mut **tx)
        .await
    }

    /// Distinct currencies with at least one unrepaid loan.
    pub async fn open_currencies(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT currency FROM margin_loans WHERE repaid_at IS NULL")
            .fetch_all(pool)
            .await
    }

    /// Earliest open loan timestamp for `currency`.
    pub async fn earliest_open_at(
        pool: &PgPool,
        currency: &str,
    ) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT MIN(opened_at) FROM margin_loans WHERE currency = $1 AND repaid_at IS NULL",
        )
        .bind(currency)
        .fetch_one(pool)
        .await
    }

    /// Open loans for `currency` at `created_time_ms`, locked for allocation.
    pub async fn open_for_charge(
        tx: &mut Transaction<'_, Postgres>,
        currency: &str,
        created_time_ms: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM margin_loans
               WHERE currency = $1
                 AND opened_at <= to_timestamp($2::double precision / 1000.0)
                 AND repaid_at IS NULL
               ORDER BY opened_at
               FOR UPDATE"#,
        )
        .bind(currency)
        .bind(created_time_ms)
        .fetch_all(&mut **tx)
        .await
    }

    /// Add a capitalised interest allocation to a loan.
    pub async fn apply_interest(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        amount_base: Decimal,
        amount_usdt: Decimal,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE margin_loans
               SET outstanding = outstanding + $1,
                   interest_base = interest_base + $1,
                   interest_usdt = interest_usdt + $2,
                   updated_at = NOW()
               WHERE id = $3"#,
        )
        .bind(amount_base)
        .bind(amount_usdt)
        .bind(id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Mark the loan's position closed (interest may still accrue until repaid).
    pub async fn close_for_entry(
        tx: &mut Transaction<'_, Postgres>,
        entry_order_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE margin_loans l
               SET position_closed_at = NOW(), updated_at = NOW()
               WHERE l.entry_order_id = $1 AND l.position_closed_at IS NULL
                 AND COALESCE((
                     SELECT SUM(c.filled_qty) FROM trade_orders c
                     WHERE c.parent_order_id = $1 AND c.status = 'filled'
                 ), 0) >= (SELECT filled_qty FROM trade_orders WHERE id = $1)"#,
        )
        .bind(entry_order_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Loans for `contract_id` that are not fully repaid.
    pub async fn unrepaid_for_contract(
        pool: &PgPool,
        contract_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM margin_loans WHERE contract_id = $1 AND repaid_at IS NULL ORDER BY opened_at",
        )
        .bind(contract_id)
        .fetch_all(pool)
        .await
    }

    /// Transaction-scoped variant of [`Self::unrepaid_for_contract`].
    pub async fn unrepaid_for_contract_tx(
        tx: &mut Transaction<'_, Postgres>,
        contract_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM margin_loans WHERE contract_id = $1 AND repaid_at IS NULL ORDER BY opened_at",
        )
        .bind(contract_id)
        .fetch_all(&mut **tx)
        .await
    }

    /// Every loan whose position is closed but whose debt is not settled.
    pub async fn all_unrepaid(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM margin_loans
               WHERE position_closed_at IS NOT NULL AND repaid_at IS NULL
               ORDER BY position_closed_at"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Number of unrepaid loans for `currency` across all contracts.
    pub async fn unrepaid_count_for_currency(
        pool: &PgPool,
        currency: &str,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM margin_loans WHERE currency = $1 AND repaid_at IS NULL",
        )
        .bind(currency)
        .fetch_one(pool)
        .await
    }

    /// Claim a loan for repayment. `None` means another worker owns it or it is
    /// already settled.
    pub async fn claim_for_repay(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE margin_loans
               SET repay_claimed_at = NOW(), updated_at = NOW()
               WHERE id = $1 AND repaid_at IS NULL
                 AND (repay_claimed_at IS NULL
                      OR repay_claimed_at < NOW() - INTERVAL '5 minutes')
               RETURNING *"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Persist the clientOid before the exchange call so an ambiguous placement
    /// can be recovered on the next pass.
    pub async fn set_repay_buy_client(
        pool: &PgPool,
        id: Uuid,
        client_oid: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE margin_loans SET repay_buy_client_oid = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(client_oid)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Record the market buy used to acquire the base asset for repayment.
    pub async fn set_repay_buy(
        pool: &PgPool,
        id: Uuid,
        order_id: &str,
        client_oid: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE margin_loans
               SET repay_buy_order_id = $1, repay_buy_client_oid = $2, updated_at = NOW()
               WHERE id = $3"#,
        )
        .bind(order_id)
        .bind(client_oid)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Forget a shortfall buy that never filled.
    pub async fn clear_repay_buy(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE margin_loans SET repay_buy_order_id = NULL, repay_buy_client_oid = NULL, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Persist a repay intent before the exchange call. If the call succeeds but
    /// the DB write that follows fails, the next pass reconciles this intent
    /// against KuCoin's repay history instead of repaying again.
    pub async fn set_repay_attempt(
        pool: &PgPool,
        id: Uuid,
        amount: Decimal,
        cost_usdt: Decimal,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE margin_loans
               SET repay_attempted_at = NOW(),
                   repay_attempted_amount = $1,
                   repay_attempted_cost_usdt = $2,
                   updated_at = NOW()
               WHERE id = $3 AND repaid_at IS NULL"#,
        )
        .bind(amount)
        .bind(cost_usdt)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Forget a repay intent that the exchange never executed.
    pub async fn clear_repay_attempt(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE margin_loans
               SET repay_attempted_at = NULL,
                   repay_attempted_amount = NULL,
                   repay_attempted_cost_usdt = NULL,
                   updated_at = NOW()
               WHERE id = $1 AND repaid_at IS NULL"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Release a claim so the next pass can retry.
    pub async fn release_claim(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE margin_loans SET repay_claimed_at = NULL, updated_at = NOW() WHERE id = $1 AND repaid_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a loan fully settled, store its actual cost, and charge the cost to
    /// its contract — all in one transaction.
    pub async fn mark_repaid(
        pool: &PgPool,
        id: Uuid,
        cost_usdt: Decimal,
        buy_order_id: Option<&str>,
        repay_order_no: Option<&str>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let loan = sqlx::query_as::<_, Self>(
            r#"UPDATE margin_loans
               SET repaid_at = NOW(), repay_claimed_at = NOW(),
                   repaid_cost_usdt = $1, repay_buy_order_id = $2,
                   repay_order_no = $3, updated_at = NOW()
               WHERE id = $4 AND repaid_at IS NULL
               RETURNING *"#,
        )
        .bind(cost_usdt)
        .bind(buy_order_id)
        .bind(repay_order_no)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(loan) = &loan {
            sqlx::query(
                "UPDATE contracts SET borrow_interest = borrow_interest + $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(cost_usdt)
            .bind(loan.contract_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(loan)
    }
}

/// One interest charge returned by KuCoin's interest history.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InterestCharge {
    pub id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub day_ratio: Option<Decimal>,
    pub created_time: i64,
    pub ingested_at: DateTime<Utc>,
}

impl InterestCharge {
    /// Insert a charge once; `None` means it was already ingested.
    pub async fn insert(
        tx: &mut Transaction<'_, Postgres>,
        currency: &str,
        amount: Decimal,
        day_ratio: Option<Decimal>,
        created_time: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO margin_interest_charges (currency, amount, day_ratio, created_time)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (currency, created_time) DO NOTHING
               RETURNING *"#,
        )
        .bind(currency)
        .bind(amount)
        .bind(day_ratio)
        .bind(created_time)
        .fetch_optional(&mut **tx)
        .await
    }

    /// Last processed charge timestamp for `currency`.
    pub async fn cursor(pool: &PgPool, currency: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT last_created_time FROM interest_cursors WHERE currency = $1")
            .bind(currency)
            .fetch_optional(pool)
            .await
    }

    /// Advance the cursor after allocations commit.
    pub async fn set_cursor(
        tx: &mut Transaction<'_, Postgres>,
        currency: &str,
        created_time: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO interest_cursors (currency, last_created_time)
               VALUES ($1, $2)
               ON CONFLICT (currency) DO UPDATE
                 SET last_created_time = GREATEST(interest_cursors.last_created_time, EXCLUDED.last_created_time),
                     updated_at = NOW()"#,
        )
        .bind(currency)
        .bind(created_time)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
