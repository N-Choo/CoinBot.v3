// ── ERC20 transfer() calldata decoder ──
// Selector: keccak256("transfer(address,uint256)")[0..4] = 0xa9059cbb
// Layout:   selector(4) | address(32 left-padded) | uint256(32 BE)

use ethers::abi::{decode, ParamType};
use rust_decimal::Decimal;

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

    /// Decode the transferred amount, scaled by `decimals`, as a [`Decimal`]
    /// (e.g. raw `9900000` with `6` decimals -> `9.9`).
    ///
    /// Returns [`Decimal::ZERO`] when the selector is missing or the calldata is
    /// malformed.
    pub fn decode_amount_scaled(input: &[u8], decimals: u32) -> Decimal {
        const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

        if input.len() < 4 || input[..4] != TRANSFER_SELECTOR {
            return Decimal::ZERO;
        }

        match Self::decode_transfer(&input[4..]) {
            Some((_, amount)) => scale_decimal(amount, decimals),
            None => Decimal::ZERO,
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

/// Render `raw`, scaled by `10^decimals`, as a [`Decimal`].
fn scale_decimal(raw: ethers::types::U256, decimals: u32) -> Decimal {
    use std::str::FromStr;

    if decimals == 0 {
        return Decimal::from_str(&raw.to_string()).unwrap_or(Decimal::ZERO);
    }

    let scale = ethers::types::U256::from(10u64).pow(ethers::types::U256::from(decimals as u64));
    let whole = raw / scale;
    let frac = raw % scale;
    if frac.is_zero() {
        return Decimal::from_str(&whole.to_string()).unwrap_or(Decimal::ZERO);
    }

    let padded = format!("{:0>width$}", frac, width = decimals as usize);
    let trimmed = padded.trim_end_matches('0');
    Decimal::from_str(&format!("{whole}.{trimmed}")).unwrap_or(Decimal::ZERO)
}
