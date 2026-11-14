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
    Repay,
}

/// Build a cross-margin market order.
///
/// `size` is the base-asset quantity and `funds` the quote amount. KuCoin
/// requires `funds` (not `size`) for an auto-borrowing buy, so leveraged longs
/// pass `funds` and leave `size` empty; all other orders pass `size`.
pub fn margin_market_order(
    ticker: &str,
    side: Side,
    size: Option<String>,
    funds: Option<String>,
    client_oid: &str,
    intent: OrderIntent,
) -> MarginOrderRequest {
    let open = intent == OrderIntent::Open;
    let close = intent == OrderIntent::Close;
    // A short opens by borrowing the base asset. (`Side` is not `Copy`, so this
    // must run before `side` moves into the request.)
    let short_open = open && side == Side::Sell;

    let mut request = MarginOrderRequest::new(TradeType::Market, ticker, side);
    request.size = size;
    request.funds = funds;
    request.client_oid = client_oid.to_string();
    // Borrow the missing asset on an open (base for a short, quote for a
    // funded long). Repay when a position closes; a short open also sets the
    // flag, matching the exchange's short flow. `repay_quote` settles any USDT
    // debt left by a long close.
    request.auto_borrow = Some(short_open || (open && request.funds.is_some()));
    request.auto_repay = Some(close || short_open);
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
            Some("15.9".to_string()),
            None,
            "cid-1",
            OrderIntent::Open,
        );
        assert_eq!(req.symbol, "XRP-USDT");
        assert_eq!(req.client_oid, "cid-1");
        assert_eq!(req.size.as_deref(), Some("15.9"));
        assert!(req.funds.is_none());
        assert_eq!(req.side, Side::Sell);
        assert!(matches!(req.order_type, TradeType::Market));
        assert_eq!(req.auto_borrow, Some(true));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn unleveraged_long_uses_size_and_does_not_borrow() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            Some("10".to_string()),
            None,
            "cid-2",
            OrderIntent::Open,
        );
        assert_eq!(req.side, Side::Buy);
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn leveraged_long_uses_funds_and_borrows() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            None,
            Some("125".to_string()),
            "cid-l",
            OrderIntent::Open,
        );
        assert!(req.size.is_none());
        assert_eq!(req.funds.as_deref(), Some("125"));
        assert_eq!(req.auto_borrow, Some(true));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn short_close_repays_but_does_not_borrow() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Buy,
            Some("10".to_string()),
            None,
            "cid-3",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(true));
    }

    #[test]
    fn repay_intent_never_borrows_or_repays() {
        let req = margin_market_order(
            "SOL-USDT",
            Side::Buy,
            Some("1".to_string()),
            None,
            "cid-r",
            OrderIntent::Repay,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(false));
    }

    #[test]
    fn long_close_repays_but_does_not_borrow() {
        let req = margin_market_order(
            "XRP-USDT",
            Side::Sell,
            Some("10".to_string()),
            None,
            "cid-4",
            OrderIntent::Close,
        );
        assert_eq!(req.auto_borrow, Some(false));
        assert_eq!(req.auto_repay, Some(true));
    }
}
