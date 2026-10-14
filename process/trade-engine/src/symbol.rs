//! KuCoin symbol formatting shared by the order paths.

/// KuCoin margin symbol: `"BTC/USDT"` -> `"BTC-USDT"`.
pub(crate) fn margin_symbol(ticker: &str) -> String {
    ticker.replace('/', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_margin_symbol() {
        assert_eq!(margin_symbol("BTC/USDT"), "BTC-USDT");
        assert_eq!(margin_symbol("BTC-USDT"), "BTC-USDT");
    }
}
