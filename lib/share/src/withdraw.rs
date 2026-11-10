//! Parsing, validation, and EIP-712 recovery for signed withdrawal intents.
//!
//! The API gateway and the withdrawal worker share this module so both apply
//! exactly the same authorization rules.
//!
//! Users sign an EIP-712 `Withdrawal` struct in the `CoinBot` / version `1`
//! domain on Ethereum mainnet. The stored `message` is the JSON representation
//! of the same fields, kept for audit and worker re-verification.

use std::collections::BTreeMap;
use std::str::FromStr;

use ethers::core::types::transaction::eip712::{EIP712Domain, Eip712DomainType, TypedData, Types};
use ethers::types::{Signature, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// EIP-712 domain name.
pub const EIP712_DOMAIN_NAME: &str = "CoinBot";
/// EIP-712 domain version.
pub const EIP712_DOMAIN_VERSION: &str = "1";
/// Only Ethereum mainnet ERC20 USDT is supported in v1.
pub const CHAIN: &str = "eth";
/// Ethereum mainnet chain id.
pub const CHAIN_ID: u64 = 1;
/// Maximum intent lifetime.
pub const MAX_TTL_SECS: i64 = 300;
/// Tolerance for a client clock that is slightly ahead.
pub const CLOCK_SKEW_SECS: i64 = 60;

/// Decoded signed withdrawal intent. Field names match the EIP-712 struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawIntent {
    pub nonce: String,
    pub currency: String,
    pub chain: String,
    pub chain_id: u64,
    pub destination: String,
    pub amount: String,
    pub fee: String,
    pub total: String,
    pub issued_at: i64,
    pub expires_at: i64,
}

impl WithdrawIntent {
    pub fn amount_decimal(&self) -> Result<Decimal, String> {
        parse_decimal(&self.amount, "amount")
    }

    pub fn fee_decimal(&self) -> Result<Decimal, String> {
        parse_decimal(&self.fee, "fee")
    }

    pub fn total_decimal(&self) -> Result<Decimal, String> {
        parse_decimal(&self.total, "total")
    }
}

/// Build the exact EIP-712 payload the client signs.
///
/// Keep this in sync with `WITHDRAW_TYPES` / `WITHDRAW_DOMAIN` in the SPA.
pub fn typed_data(intent: &WithdrawIntent) -> TypedData {
    let field = |name: &str, ty: &str| Eip712DomainType {
        name: name.to_string(),
        r#type: ty.to_string(),
    };

    let mut types: Types = BTreeMap::new();
    types.insert(
        "Withdrawal".to_string(),
        vec![
            field("nonce", "string"),
            field("currency", "string"),
            field("chain", "string"),
            field("chain_id", "uint256"),
            field("destination", "address"),
            field("amount", "string"),
            field("fee", "string"),
            field("total", "string"),
            field("issued_at", "uint256"),
            field("expires_at", "uint256"),
        ],
    );

    let mut message = BTreeMap::new();
    message.insert("nonce".into(), serde_json::json!(intent.nonce));
    message.insert("currency".into(), serde_json::json!(intent.currency));
    message.insert("chain".into(), serde_json::json!(intent.chain));
    message.insert("chain_id".into(), serde_json::json!(intent.chain_id));
    message.insert("destination".into(), serde_json::json!(intent.destination));
    message.insert("amount".into(), serde_json::json!(intent.amount));
    message.insert("fee".into(), serde_json::json!(intent.fee));
    message.insert("total".into(), serde_json::json!(intent.total));
    message.insert("issued_at".into(), serde_json::json!(intent.issued_at));
    message.insert("expires_at".into(), serde_json::json!(intent.expires_at));

    TypedData {
        domain: EIP712Domain {
            name: Some(EIP712_DOMAIN_NAME.to_string()),
            version: Some(EIP712_DOMAIN_VERSION.to_string()),
            chain_id: Some(U256::from(CHAIN_ID)),
            verifying_contract: None,
            salt: None,
        },
        types,
        primary_type: "Withdrawal".to_string(),
        message,
    }
}

/// Parse the stored intent JSON.
///
/// The legacy `CoinBot Withdrawal v1\n` prefix is accepted for rows created
/// before the EIP-712 switch.
pub fn parse(message: &str) -> Result<WithdrawIntent, String> {
    let body = message
        .strip_prefix("CoinBot Withdrawal v1\n")
        .unwrap_or(message);
    serde_json::from_str(body).map_err(|e| format!("invalid withdrawal message: {e}"))
}

/// Validate the intent against the session wallet and live exchange values.
///
/// Returns `(amount, fee, total)` on success.
pub fn validate(
    intent: &WithdrawIntent,
    wallet: &str,
    now_secs: i64,
    min: Decimal,
    precision: u32,
) -> Result<(Decimal, Decimal, Decimal), String> {
    if intent.nonce.is_empty() {
        return Err("missing nonce".into());
    }
    if intent.currency != "USDT" || intent.chain != CHAIN || intent.chain_id != CHAIN_ID {
        return Err("unsupported currency or chain".into());
    }
    if !is_address(&intent.destination) {
        return Err("invalid destination address".into());
    }
    if !intent.destination.eq_ignore_ascii_case(wallet) {
        return Err("destination must be the session wallet".into());
    }

    let amount = intent.amount_decimal()?;
    let fee = intent.fee_decimal()?;
    let total = intent.total_decimal()?;

    if amount <= Decimal::ZERO || fee < Decimal::ZERO {
        return Err("amount must be positive".into());
    }
    if amount + fee != total {
        return Err("total must equal amount + fee".into());
    }
    if amount < min {
        return Err(format!("amount must be at least {min}"));
    }
    if amount.normalize().scale() > precision {
        return Err(format!("amount supports at most {precision} decimals"));
    }
    if intent.expires_at <= now_secs {
        return Err("withdrawal intent expired".into());
    }
    if intent.issued_at > now_secs + CLOCK_SKEW_SECS
        || intent.expires_at - intent.issued_at > MAX_TTL_SECS
    {
        return Err("invalid withdrawal window".into());
    }

    Ok((amount, fee, total))
}

/// Verify that `signature` is a valid EIP-712 signature by `wallet`.
pub fn verify(signature: &str, intent: &WithdrawIntent, wallet: &str) -> Result<(), String> {
    let signature =
        Signature::from_str(signature).map_err(|e| format!("invalid signature: {e}"))?;
    let signer = signature
        .recover_typed_data(&typed_data(intent))
        .map_err(|e| format!("signature recovery failed: {e}"))?;

    if format!("0x{signer:x}").eq_ignore_ascii_case(wallet) {
        Ok(())
    } else {
        Err("signature does not match the session wallet".into())
    }
}

fn parse_decimal(value: &str, field: &str) -> Result<Decimal, String> {
    Decimal::from_str(value).map_err(|_| format!("{field} must be a number"))
}

fn is_address(value: &str) -> bool {
    value.len() == 42
        && value.starts_with("0x")
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const WALLET: &str = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c";

    fn intent(destination: &str) -> WithdrawIntent {
        WithdrawIntent {
            nonce: "n1".into(),
            currency: "USDT".into(),
            chain: "eth".into(),
            chain_id: 1,
            destination: destination.into(),
            amount: "95".into(),
            fee: "5.5".into(),
            total: "100.5".into(),
            issued_at: 1_731_150_000,
            expires_at: 1_731_150_300,
        }
    }

    fn message() -> String {
        serde_json::to_string(&intent(WALLET)).unwrap()
    }

    #[test]
    fn parses_a_plain_message() {
        let parsed = parse(&message()).unwrap();
        assert_eq!(parsed.nonce, "n1");
        assert_eq!(parsed.amount_decimal().unwrap().to_string(), "95");
    }

    #[test]
    fn accepts_the_legacy_prefix() {
        let legacy = format!("CoinBot Withdrawal v1\n{}", message());
        assert_eq!(parse(&legacy).unwrap().nonce, "n1");
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse("not-json").is_err());
    }

    #[test]
    fn validates_total_and_window() {
        let parsed = parse(&message()).unwrap();
        let (amount, fee, total) =
            validate(&parsed, WALLET, 1_731_150_100, Decimal::from(30), 6).unwrap();
        assert_eq!(amount, Decimal::from(95));
        assert_eq!(fee, Decimal::from_str("5.5").unwrap());
        assert_eq!(total, Decimal::from_str("100.5").unwrap());
    }

    #[test]
    fn rejects_destination_mismatch() {
        let parsed = parse(&message()).unwrap();
        assert!(validate(
            &parsed,
            "0xdead000000000000000000000000000000000000",
            1_731_150_100,
            Decimal::ZERO,
            6
        )
        .is_err());
    }

    #[test]
    fn rejects_an_expired_intent() {
        let parsed = parse(&message()).unwrap();
        assert!(validate(&parsed, WALLET, 1_731_150_301, Decimal::ZERO, 6).is_err());
    }

    #[tokio::test]
    async fn recovers_the_typed_signer() {
        use ethers::core::rand::{rngs::StdRng, SeedableRng};
        use ethers::signers::{LocalWallet, Signer};

        let mut rng = StdRng::seed_from_u64(7);
        let wallet = LocalWallet::new(&mut rng);
        let address = format!("0x{:x}", wallet.address());
        let parsed = intent(&address);

        let signature = wallet.sign_typed_data(&typed_data(&parsed)).await.unwrap();
        let signature = format!("0x{signature}");

        verify(&signature, &parsed, &address).unwrap();
        assert!(verify(&signature, &parsed, WALLET).is_err());
    }
}
