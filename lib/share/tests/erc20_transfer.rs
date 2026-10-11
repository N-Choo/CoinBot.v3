//! Integration tests for [`share::erc20::Erc20`] calldata decoding.

// ERC20 transfer() calldata layout:
//
//   offset  size  field
//   ──────  ────  ──────────────────────────
//   0       4     function selector  (0xa9059cbb)
//   4       32    recipient address  (left-padded to 32 bytes)
//   36      32    amount             (uint256 big-endian)
//   ──────  ────
//           68    total bytes
//
// Address encoding: 12 zero bytes + 20 address bytes (left-padded)
// Amount encoding:  32 byte big-endian uint256

use share::erc20::Erc20;

/// Encode `transfer(recipient, amount)` calldata.
fn build_input(recipient: &[u8; 20], amount_raw: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(68);
    data.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
    let mut addr = [0u8; 32];
    addr[12..].copy_from_slice(recipient);
    data.extend_from_slice(&addr);
    let mut amt = [0u8; 32];
    amt[24..].copy_from_slice(&amount_raw.to_be_bytes());
    data.extend_from_slice(&amt);
    data
}

/// The platform deposit wallet bytes.
fn platform() -> [u8; 20] {
    "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c"
        .parse::<ethers::types::Address>()
        .unwrap()
        .into()
}

/// An arbitrary non-platform address.
fn attacker() -> [u8; 20] {
    "0xdead000000000000000000000000000000000000"
        .parse::<ethers::types::Address>()
        .unwrap()
        .into()
}

/// A well-formed transfer decodes to its amount.
#[test]
fn test_amount_valid_transfer() {
    let input = build_input(&platform(), 1_000_000);
    assert_eq!(Erc20::decode_amount(&input), "1000000");
}

/// A `u64::MAX` amount decodes without loss.
#[test]
fn test_amount_large_value() {
    let input = build_input(&platform(), u64::MAX);
    assert_eq!(Erc20::decode_amount(&input), u64::MAX.to_string());
}

/// A zero amount decodes to `"0"`.
#[test]
fn test_amount_zero() {
    let input = build_input(&platform(), 0);
    assert_eq!(Erc20::decode_amount(&input), "0");
}

/// A wrong function selector yields `"0"`.
#[test]
fn test_amount_wrong_selector() {
    assert_eq!(Erc20::decode_amount(&[0x00, 0x01, 0x02, 0x03, 0xff]), "0");
}

/// Calldata shorter than the selector yields `"0"`.
#[test]
fn test_amount_too_short() {
    assert_eq!(Erc20::decode_amount(&[0xa9]), "0");
}

/// A transfer to the platform wallet decodes to its address.
#[test]
fn test_recipient_valid() {
    let input = build_input(&platform(), 500);
    assert_eq!(
        Erc20::decode_recipient(&input).as_deref(),
        Some("0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c")
    );
}

/// A transfer to a non-platform address decodes to that address.
#[test]
fn test_recipient_attacker() {
    let input = build_input(&attacker(), 500);
    assert_eq!(
        Erc20::decode_recipient(&input).as_deref(),
        Some("0xdead000000000000000000000000000000000000")
    );
}

/// A wrong function selector yields `None`.
#[test]
fn test_recipient_wrong_selector() {
    assert_eq!(Erc20::decode_recipient(&[0x00, 0x01, 0x02, 0x03]), None);
}

/// Calldata shorter than the selector yields `None`.
#[test]
fn test_recipient_too_short() {
    assert_eq!(Erc20::decode_recipient(&[0xa9, 0x05]), None);
}
