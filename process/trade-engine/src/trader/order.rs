//! Exchange request building: margin order shape, borrow/repay rules, side mapping. Pure, no I/O.

use kucoin::{
    types::margin::MarginOrderRequest,
    types::spot::{Side, TradeType},
};

/// Whether a margin order opens a new position or closes an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderIntent {
    Open,
    Close,
}

/// Build a cross-margin market order.
///
/// `size` is the base-asset quantity. Only short positions borrow/repay: a
/// short entry (`sell`) borrows the base asset (and is set to repay); a short
/// close (`buy`) repays it. Long orders never borrow or repay.
pub fn margin_market_order(
    ticker: &str,
    side: Side,
    size: String,
    client_oid: &str,
    intent: OrderIntent,
) -> MarginOrderRequest {
    let short_open = intent == OrderIntent::Open && side == Side::Sell;
    let short_close = intent == OrderIntent::Close && side == Side::Buy;

    let mut request = MarginOrderRequest::new(TradeType::Market, ticker, side);
    request.size = Some(size);
    request.client_oid = client_oid.to_string();
    request.auto_borrow = Some(short_open);
    request.auto_repay = Some(short_open || short_close);
    request
}

/// Map an already-uppercased signal action to a KuCoin side and its wire string.
pub(super) fn order_side(action: &str) -> Option<(Side, &'static str)> {
    match action {
        "BUY" => Some((Side::Buy, "buy")),
        "SELL" => Some((Side::Sell, "sell")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_buy_and_sell() {
        assert_eq!(order_side("BUY"), Some((Side::Buy, "buy")));
        assert_eq!(order_side("SELL"), Some((Side::Sell, "sell")));
    }

    #[test]
    fn rejects_unknown_or_lowercase_actions() {
        assert_eq!(order_side("buy"), None);
        assert_eq!(order_side("HOLD"), None);
        assert_eq!(order_side(""), None);
    }

    #[test]
    fn short_open_borrows_and_repays() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Sell,
            "15.9".to_string(),
            "cid-1",
            OrderIntent::Open,
        );
        assert_eq!(req.symbol, "XRP-USDT");
        assert_eq!(req.client_oid, "cid-1");
        assert_eq!(req.size.as_deref(), Some("15.9"));
        assert_eq!(req.side, Side::Sell);
        assert!(matches!(req.order_type, TradeType::Market));
        assert_eq!(req.auto_borrow, Some(true));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn long_open_does_not_borrow_or_repay() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            "10".to_string(),
            "cid-2",
            OrderIntent::Open,
        );
        assert_eq!(req.side, Side::Buy);
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn short_close_repays_but_does_not_borrow() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            "10".to_string(),
            "cid-3",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn long_close_does_not_borrow_or_repay() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Sell,
            "10".to_string(),
            "cid-4",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }
}
