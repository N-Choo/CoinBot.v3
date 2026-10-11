use serde::Deserialize;

/// Request body for `POST /api/transactions/deposit`.
#[derive(serde::Deserialize)]
pub struct DepositPayloadRequest {
    /// Ethereum transaction hash of the USDT transfer to verify.
    #[serde(deserialize_with = "validate_tx_hash")]
    pub tx_hash: String,
}

/// Reject malformed transaction hashes during deserialization.
///
/// A valid hash is exactly 66 characters and `0x`-prefixed (32-byte hex).
fn validate_tx_hash<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.len() != 66 || !s.starts_with("0x") {
        return Err(serde::de::Error::custom("invalid tx hash"));
    }
    Ok(s)
}
