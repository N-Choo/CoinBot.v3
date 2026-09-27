//! EIP-191 (`personal_sign`) wallet signature recovery.

use std::str::FromStr;

use ethers::types::Signature;
use ethers::utils::hash_message;

use crate::error::ProcessError;

/// Recover the signer's lowercase `0x`-prefixed wallet address from a signature
/// over `message`.
///
/// Returns [`ProcessError::Internal`] when the signature cannot be parsed or
/// the address cannot be recovered.
pub fn recover_wallet(signature: &str, message: &str) -> Result<String, ProcessError> {
    let sig = Signature::from_str(signature)
        .map_err(|e| ProcessError::Internal(format!("invalid signature: {e}")))?;

    let address = sig
        .recover(hash_message(message))
        .map_err(|e| ProcessError::Internal(format!("signature recovery failed: {e}")))?;

    Ok(format!("0x{:x}", address).to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::core::rand::{rngs::StdRng, SeedableRng};
    use ethers::signers::{LocalWallet, Signer};

    /// A valid signature recovers the signer's address.
    #[tokio::test]
    async fn recovers_signer_address() {
        let mut rng = StdRng::seed_from_u64(84);
        let wallet = LocalWallet::new(&mut rng);
        let expected = format!("0x{:x}", wallet.address()).to_lowercase();

        let message = "test_nonce";
        let signature = format!("0x{}", wallet.sign_message(message).await.unwrap());

        assert_eq!(recover_wallet(&signature, message).unwrap(), expected);
    }

    /// A malformed signature is rejected.
    #[test]
    fn rejects_malformed_signature() {
        assert!(recover_wallet("not-a-signature", "msg").is_err());
    }
}
