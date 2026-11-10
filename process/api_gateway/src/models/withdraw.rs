use serde::{Deserialize, Serialize};

/// Body for `POST /api/transactions/withdraw`.
#[derive(Deserialize)]
pub struct WithdrawPayloadRequest {
    pub nonce: String,
    pub message: String,
    pub signature: String,
}

/// Quote returned before the user signs.
#[derive(Serialize)]
pub struct WithdrawQuoteResponse {
    pub destination: String,
    pub currency: String,
    pub chain: String,
    pub chain_id: u64,
    pub chain_name: String,
    pub fee: String,
    pub min: String,
    pub precision: u32,
    pub available_balance: String,
    pub max_withdrawable: String,
}

/// Withdrawal shown in the activity list.
#[derive(Serialize)]
pub struct WithdrawView {
    pub id: uuid::Uuid,
    pub amount: String,
    pub fee: String,
    pub total: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&share::db::withdrawals::Withdrawal> for WithdrawView {
    fn from(withdrawal: &share::db::withdrawals::Withdrawal) -> Self {
        Self {
            id: withdrawal.id,
            amount: withdrawal.amount.to_string(),
            fee: withdrawal.fee.to_string(),
            total: withdrawal.total.to_string(),
            status: withdrawal.status.clone(),
            tx_hash: withdrawal.tx_hash.clone(),
            failure_reason: withdrawal.failure_reason.clone(),
            created_at: withdrawal.created_at,
            completed_at: withdrawal.completed_at,
        }
    }
}
