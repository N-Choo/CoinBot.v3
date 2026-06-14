#[cfg(test)]
mod tests {
    use ethers::types::{Address, Bytes, Transaction as EthTx, TxHash, U256};

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

    fn random_wallet() -> Address {
        "0xdead000000000000000000000000000000000000"
            .parse()
            .unwrap()
    }

    fn dummy_tx_hash() -> TxHash {
        [0u8; 32].into()
    }

    fn erc20_transfer_input(recipient: Address, amount: U256) -> Bytes {
        let selector = [0xa9u8, 0x05, 0x9c, 0xbb];
        let mut data = Vec::with_capacity(4 + 64);
        data.extend_from_slice(&selector);

        let mut addr_bytes = [0u8; 32];
        addr_bytes[12..].copy_from_slice(recipient.as_ref());
        data.extend_from_slice(&addr_bytes);

        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);
        data.into()
    }

    #[test]
    fn test_usdt_to_platform_wallet_is_accepted() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: erc20_transfer_input(platform_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };
        assert_eq!(tx.to, Some(usdt_contract()));

        let recipient = share::erc20::Erc20::decode_recipient(&tx.input);
        assert!(recipient.is_some());
        assert_eq!(
            recipient.unwrap().parse::<Address>().ok(),
            Some(platform_wallet())
        );
    }

    #[test]
    fn test_wrong_recipient_is_rejected() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: erc20_transfer_input(random_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };

        let recipient = share::erc20::Erc20::decode_recipient(&tx.input);
        assert!(recipient.is_some());
        assert_ne!(
            recipient.unwrap().parse::<Address>().ok(),
            Some(platform_wallet())
        );
    }

    #[test]
    fn test_non_usdt_contract_is_rejected() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(random_wallet()),
            input: erc20_transfer_input(platform_wallet(), U256::from(1_000_000_000u64)),
            ..Default::default()
        };
        assert_ne!(tx.to, Some(usdt_contract()));
    }

    #[test]
    fn test_malformed_input_decodes_to_none() {
        let tx = EthTx {
            hash: dummy_tx_hash(),
            from: random_wallet(),
            to: Some(usdt_contract()),
            input: Bytes::from(vec![0x00, 0x01, 0x02]),
            ..Default::default()
        };
        assert!(share::erc20::Erc20::decode_recipient(&tx.input).is_none());
        assert_eq!(share::erc20::Erc20::decode_amount(&tx.input), "0");
    }
}
