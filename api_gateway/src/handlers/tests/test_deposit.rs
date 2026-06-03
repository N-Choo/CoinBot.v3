#[cfg(test)]
mod tests {
    use ethers::types::{Address, Bytes, Transaction as EthTx, TxHash, U256};

    use crate::handlers::transaction::Transaction;

    fn dummy_tx_hash() -> TxHash {
        [0u8; 32].into()
    }

    fn platform_wallet() -> Address {
        "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c"
            .parse()
            .unwrap()
    }

    fn usdt_contract() -> Address {
        "0xdac17f958d2ee523a2206206994597c13d831ec7"
            .parse()
            .unwrap()
    }

    fn usdc_contract() -> Address {
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
            .parse()
            .unwrap()
    }

    fn random_wallet() -> Address {
        "0xdead000000000000000000000000000000000000"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_validate_eth_deposit() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: platform_wallet(),
            to: Some(platform_wallet()),
            value: U256::from(1_000_000),
            ..Default::default()
        };
        let info = Transaction::validate(&tx);
        assert!(info.is_some());
        assert_eq!(info.unwrap().ticker, "ETH");
    }

    #[test]
    fn test_validate_eth_not_to_platform() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(random_wallet()),
            value: U256::from(1_000_000),
            ..Default::default()
        };
        assert!(Transaction::validate(&tx).is_none());
    }

    fn erc20_transfer_input(recipient: Address, amount: U256) -> Bytes {
        let selector = [0xa9u8, 0x05, 0x9c, 0xbb];
        let mut data = Vec::with_capacity(4 + 64);
        data.extend_from_slice(&selector);

        let mut addr_bytes = [0u8; 32];
        addr_bytes[12..].copy_from_slice(recipient.as_ref());
        data.extend_from_slice(&addr_bytes);

        let mut amount_bytes = [0u8; 32];
        let amount_be = amount;
        amount_be.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);
        data.into()
    }

    #[test]
    fn test_validate_erc20_correct_recipient() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: erc20_transfer_input(platform_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };
        let info = Transaction::validate(&tx);
        assert!(info.is_some());
        assert_eq!(info.unwrap().ticker, "USDT");
    }

    #[test]
    fn test_validate_erc20_wrong_recipient() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: erc20_transfer_input(random_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };
        assert!(Transaction::validate(&tx).is_none());
    }

    #[test]
    fn test_validate_erc20_malformed_input() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: Bytes::from(vec![0x00, 0x01, 0x02]),
            ..Default::default()
        };
        assert!(Transaction::validate(&tx).is_none());
    }

    #[test]
    fn test_validate_unknown_token() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(random_wallet()),
            input: erc20_transfer_input(platform_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };
        assert!(Transaction::validate(&tx).is_none());
    }

    #[test]
    fn test_validate_usdc_correct_recipient() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdc_contract()),
            input: erc20_transfer_input(platform_wallet(), U256::from(5_000_000_000u64)),
            ..Default::default()
        };
        let info = Transaction::validate(&tx);
        assert!(info.is_some());
        assert_eq!(info.unwrap().ticker, "USDC");
    }

    #[test]
    fn test_validate_erc20_zero_amount() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: erc20_transfer_input(platform_wallet(), U256::zero()),
            ..Default::default()
        };
        let info = Transaction::validate(&tx);
        assert!(info.is_some());
        assert_eq!(info.unwrap().ticker, "USDT");
    }
}
