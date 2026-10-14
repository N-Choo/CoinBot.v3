//! Pure fee/PnL math for a closed round trip.
//!
//! No I/O: given the entry fill, the close fill, and each leg's fee, this
//! computes the realized PnL and the credit to return to the contract. Kept
//! side-effect-free so the money math is unit-testable in isolation.

use rust_decimal::Decimal;

/// Split a KuCoin symbol `"XRP-USDT"` into `(base, quote)`.
pub(super) fn split_symbol(symbol: &str) -> (String, String) {
    match symbol.split_once('-') {
        Some((b, q)) => (b.to_string(), q.to_string()),
        None => (symbol.to_string(), String::new()),
    }
}

/// Express an order's `fee` in the contract's quote currency.
///
/// KuCoin charges the fee in the asset named by `fee_currency` (often the quote,
/// e.g. USDT, but sometimes the base). A base-currency fee is converted with the
/// order's fill price; anything else is returned unchanged (best effort — the
/// platform trades USDT-quoted pairs).
pub(super) fn fee_in_quote(
    fee: Decimal,
    fee_currency: Option<&str>,
    base: &str,
    quote: &str,
    fill_price: Decimal,
) -> Decimal {
    match fee_currency {
        Some(c) if c.eq_ignore_ascii_case(quote) => fee,
        Some(c) if c.eq_ignore_ascii_case(base) => fee * fill_price,
        _ => {
            if !fee.is_zero() {
                log::warn!("[RECONCILE] fee {fee} has no/unknown currency; treating as quote");
            }
            fee
        }
    }
}

/// Realized PnL of a closed round trip, in quote currency, net of fees.
///
/// `entry_basis` is what the entry actually transacted (`filled_qty * fill_price`):
/// for a short it is the proceeds of selling the borrowed base; for a long it is
/// the cost of buying the base. A short profits when it buys back cheaper.
pub(super) fn realized_pnl(
    entry_side: &str,
    entry_basis: Decimal,
    close_proceeds: Decimal,
    fees_quote: Decimal,
) -> Decimal {
    match entry_side {
        "sell" => entry_basis - close_proceeds - fees_quote,
        _ => close_proceeds - entry_basis - fees_quote,
    }
}

/// Credit returned to the contract on close: the committed entry fund plus (or
/// minus) the realized PnL.
pub(super) fn close_credit(entry_funds: Decimal, pnl: Decimal) -> Decimal {
    entry_funds + pnl
}

/// Cap a raw PnL at the capital committed to the round trip: a loss cannot
/// exceed the funds the contract risked, so the persisted PnL always matches
/// the contract's balance change.
pub(super) fn cap_loss(raw_pnl: Decimal, committed: Decimal) -> Decimal {
    raw_pnl.max(-committed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn splits_symbol_into_base_and_quote() {
        assert_eq!(
            split_symbol("XRP-USDT"),
            ("XRP".to_string(), "USDT".to_string())
        );
    }

    #[test]
    fn fee_stays_as_is_when_charged_in_the_quote() {
        assert_eq!(
            fee_in_quote(dec("0.0013"), Some("USDT"), "XRP", "USDT", dec("1.32484")),
            dec("0.0013")
        );
        // Missing currency is treated as quote (best effort).
        assert_eq!(
            fee_in_quote(dec("0.0013"), None, "XRP", "USDT", dec("1.32484")),
            dec("0.0013")
        );
    }

    #[test]
    fn fee_in_base_is_converted_at_the_fill_price() {
        // 0.01 XRP fee at 1.32484 USDT/XRP -> 0.0132484 USDT.
        assert_eq!(
            fee_in_quote(dec("0.01"), Some("XRP"), "XRP", "USDT", dec("1.32484")),
            dec("0.0132484")
        );
    }

    #[test]
    fn realized_pnl_is_net_of_fees_for_both_sides() {
        // Long: bought at 100, sold at 120, 2 fees -> 18.
        assert_eq!(
            realized_pnl("buy", dec("100"), dec("120"), dec("2")),
            dec("18")
        );
        // Short: sold at 100, bought back at 80, 2 fees -> 18.
        assert_eq!(
            realized_pnl("sell", dec("100"), dec("80"), dec("2")),
            dec("18")
        );
    }

    #[test]
    fn realized_pnl_short_can_lose_more_than_capital() {
        // Short: sold at 100, bought back at 250, 2 fees -> -152.
        assert_eq!(
            realized_pnl("sell", dec("100"), dec("250"), dec("2")),
            dec("-152")
        );
    }

    #[test]
    fn credit_is_entry_fund_plus_pnl() {
        assert_eq!(close_credit(dec("100"), dec("18")), dec("118"));
        assert_eq!(close_credit(dec("100"), dec("-50")), dec("50"));
    }

    #[test]
    fn loss_is_capped_at_committed_capital() {
        assert_eq!(cap_loss(dec("18"), dec("100")), dec("18"));
        assert_eq!(cap_loss(dec("-50"), dec("100")), dec("-50"));
        assert_eq!(cap_loss(dec("-250"), dec("100")), dec("-100"));
    }
}
