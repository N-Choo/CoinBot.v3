//! Ingest KuCoin hourly interest charges and allocate them to open loans.

use std::time::Duration;

use kucoin::client::rest::KuCoinClient;
use rust_decimal::Decimal;
use share::db::loan::{InterestCharge, MarginLoan};
use sqlx::PgPool;
use uuid::Uuid;

use crate::kucoin_margin;

const POLL_SECS: u64 = 60;

/// Split `charge` across `(loan_id, outstanding)` pro-rata. The rounding
/// remainder goes to the last loan so the sum is exact.
pub(crate) fn allocate(charge: Decimal, loans: &[(Uuid, Decimal)]) -> Vec<(Uuid, Decimal)> {
    let total: Decimal = loans.iter().map(|(_, o)| *o).sum();
    if total <= Decimal::ZERO || charge <= Decimal::ZERO {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(loans.len());
    let mut allocated = Decimal::ZERO;
    for (i, (id, outstanding)) in loans.iter().enumerate() {
        let amount = if i + 1 == loans.len() {
            charge - allocated
        } else {
            (charge * *outstanding / total).round_dp(18)
        };
        allocated += amount;
        out.push((*id, amount));
    }
    out
}

/// Background loop.
pub async fn run(pool: PgPool, kcc: KuCoinClient) {
    loop {
        if let Err(e) = sweep(&pool, &kcc).await {
            log::error!("[INTEREST] sweep failed: {e}");
        }
        tokio::time::sleep(Duration::from_secs(POLL_SECS)).await;
    }
}

/// Process every currency with an open loan.
pub async fn sweep(pool: &PgPool, kcc: &KuCoinClient) -> Result<(), String> {
    let currencies = MarginLoan::open_currencies(pool)
        .await
        .map_err(|e| e.to_string())?;
    for currency in currencies {
        sweep_currency(pool, kcc, &currency).await?;
    }
    Ok(())
}

/// Process one currency. Safe to call concurrently: charge insertion is
/// idempotent and loans are row-locked during allocation.
pub async fn sweep_currency(
    pool: &PgPool,
    kcc: &KuCoinClient,
    currency: &str,
) -> Result<(), String> {
    let price = kucoin_margin::last_price(kcc, currency).await?;
    let end = utc_now_ms();
    let start = match InterestCharge::cursor(pool, currency)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(cursor) => cursor,
        None => MarginLoan::earliest_open_at(pool, currency)
            .await
            .map_err(|e| e.to_string())?
            .map(|t| t.timestamp_millis())
            .unwrap_or(end - 86_400_000),
    };

    let mut page = 1;
    loop {
        let data = kucoin_margin::interest_page(kcc, currency, start, end, page).await?;
        for item in &data.items {
            let amount: Decimal = item
                .interest_amount
                .parse()
                .map_err(|_| format!("bad interest amount {}", item.interest_amount))?;
            let day_ratio = item.day_ratio.as_deref().and_then(|v| v.parse().ok());

            let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
            let Some(charge) =
                InterestCharge::insert(&mut tx, currency, amount, day_ratio, item.created_time)
                    .await
                    .map_err(|e| e.to_string())?
            else {
                tx.commit().await.map_err(|e| e.to_string())?;
                continue;
            };

            let loans = MarginLoan::open_for_charge(&mut tx, currency, charge.created_time)
                .await
                .map_err(|e| e.to_string())?;
            let pairs: Vec<(Uuid, Decimal)> = loans.iter().map(|l| (l.id, l.outstanding)).collect();
            for (id, base) in allocate(charge.amount, &pairs) {
                MarginLoan::apply_interest(&mut tx, id, base, base * price)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            InterestCharge::set_cursor(&mut tx, currency, charge.created_time)
                .await
                .map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())?;
        }

        if page >= data.total_page {
            break;
        }
        page += 1;
    }
    Ok(())
}

fn utc_now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use uuid::Uuid;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn single_loan_gets_the_whole_charge() {
        let a = Uuid::new_v4();
        assert_eq!(allocate(dec("5"), &[(a, dec("100"))]), vec![(a, dec("5"))]);
    }

    #[test]
    fn two_loans_split_by_outstanding() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let out = allocate(dec("9"), &[(a, dec("100")), (b, dec("200"))]);
        assert_eq!(out[0], (a, dec("3")));
        assert_eq!(out[1], (b, dec("6")));
    }

    #[test]
    fn rounding_remainder_goes_to_the_last_loan() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let out = allocate(dec("1"), &[(a, dec("1")), (b, dec("2"))]);
        assert_eq!(out.iter().map(|(_, v)| *v).sum::<Decimal>(), dec("1"));
    }

    #[test]
    fn zero_outstanding_allocates_nothing() {
        let a = Uuid::new_v4();
        assert!(allocate(dec("1"), &[(a, Decimal::ZERO)]).is_empty());
    }
}
