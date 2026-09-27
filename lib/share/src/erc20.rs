// ── ERC20 transfer() calldata decoder ──
// Selector: keccak256("transfer(address,uint256)")[0..4] = 0xa9059cbb
// Layout:   selector(4) | address(32 left-padded) | uint256(32 BE)

use ethers::abi::{decode, ParamType};

/// Decoder for ERC-20 `transfer(address,uint256)` calldata.
pub struct Erc20;

impl Erc20 {
    /// Decode the transferred amount as a decimal string.
    ///
    /// Returns `"0"` when the selector is missing or the calldata is malformed.
    pub fn decode_amount(input: &[u8]) -> String {
        const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

        if input.len() < 4 || input[..4] != TRANSFER_SELECTOR {
            return "0".into();
        }

        match Self::decode_transfer(&input[4..]) {
            Some((_, amount)) => amount.to_string(),
            None => "0".into(),
        }
    }

    /// Decode the recipient address as a `0x`-prefixed hex string.
    ///
    /// Returns `None` when the selector is missing or the calldata is malformed.
    pub fn decode_recipient(input: &[u8]) -> Option<String> {
        const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

        if input.len() < 4 || input[..4] != TRANSFER_SELECTOR {
            return None;
        }

        Self::decode_transfer(&input[4..]).map(|(recipient, _)| format!("0x{:x}", recipient))
    }

    /// ABI-decode `(address recipient, uint256 amount)` from transfer calldata.
    fn decode_transfer(data: &[u8]) -> Option<(ethers::types::Address, ethers::types::U256)> {
        match decode(&[ParamType::Address, ParamType::Uint(256)], data) {
            Ok(tokens) => {
                let recipient = tokens[0].clone().into_address()?;
                let amount = tokens[1].clone().into_uint()?;
                Some((recipient, amount))
            }
            Err(_) => None,
        }
    }
}
