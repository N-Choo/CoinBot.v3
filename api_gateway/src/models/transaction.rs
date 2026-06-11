use serde::Deserialize;

#[derive(serde::Deserialize)]
pub struct DepositPayloadRequest {
    #[serde(deserialize_with = "validate_tx_hash")]
    pub tx_hash: String,
}

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
