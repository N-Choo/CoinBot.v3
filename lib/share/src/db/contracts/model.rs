use uuid::Uuid;

pub struct Contracts;

impl Contracts {
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        user_uid: Uuid,
        signature: String,
        message: String,
        nonce: String,
        ticker: String,
        amount: String,
        sl_pct: f32,
        tp_pct: f32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active')"#,
        )
        .bind(user_uid)
        .bind(signature)
        .bind(message)
        .bind(nonce)
        .bind(ticker)
        .bind(amount)
        .bind(sl_pct)
        .bind(tp_pct)
        .execute(pool)
        .await?;
        Ok(())
    }
}
