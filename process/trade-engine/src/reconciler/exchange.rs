//! KuCoin margin order I/O and outcome classification.
//!
//! Thin wrappers over the margin HF order endpoints plus the pure rule that maps
//! an exchange order's `active` flag and dealt size to a settlement decision.

use kucoin::client::rest::KuCoinClient;
use kucoin::types::margin::{MarginOrderId, MarginOrderStatus};
use rust_decimal::Decimal;
use share::ProcessError;

/// Fetch an order by its exchange id via the margin HF endpoint.
pub(super) async fn fetch_order(
    kcc: &KuCoinClient,
    order_id: &str,
    symbol: &str,
) -> Result<Option<MarginOrderStatus>, ProcessError> {
    let res = kcc
        .margin()
        .get_order(order_id, symbol)
        .await
        .map_err(|e| ProcessError::Network(format!("get_order failed: {e}")))?;
    Ok(res.data)
}

/// Fetch an order by the clientOid we assigned (`comment`). A 404 means the
/// order was never placed, which we surface as `Ok(None)` (not an error) so the
/// short grace window applies. KuCoin returns this for a missing margin order
/// (`code:126043`, HTTP 404).
pub(super) async fn fetch_by_client_oid(
    kcc: &KuCoinClient,
    client_oid: &str,
    symbol: &str,
) -> Result<Option<MarginOrderId>, ProcessError> {
    match kcc
        .margin()
        .get_order_by_client_oid(client_oid, symbol)
        .await
    {
        Ok(res) => Ok(res.data),
        Err(e) if e.is_not_found() => Ok(None),
        Err(e) => Err(ProcessError::Network(format!(
            "client-order lookup failed: {e}"
        ))),
    }
}

/// Outcome of reconciling one exchange order.
#[derive(Debug, PartialEq)]
pub(super) enum OrderOutcome {
    /// Still working (or unparseable, retried) on the exchange.
    Working,
    /// Finished with no fill (cancelled).
    Cancelled,
    /// Filled.
    Filled,
}

/// Classify an exchange order from its `active` flag and dealt size.
///
/// An unparseable dealt size is treated as `Working` so a decode hiccup retries
/// rather than mis-cancelling a real fill.
pub(super) fn classify_order(active: bool, deal_size: &str) -> OrderOutcome {
    if active {
        return OrderOutcome::Working;
    }
    match deal_size.parse::<Decimal>() {
        Ok(v) if v > Decimal::ZERO => OrderOutcome::Filled,
        Ok(_) => OrderOutcome::Cancelled,
        Err(_) => OrderOutcome::Working,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_working_order() {
        assert_eq!(classify_order(true, "0"), OrderOutcome::Working);
        assert_eq!(classify_order(true, "5"), OrderOutcome::Working);
    }

    #[test]
    fn classifies_unfilled_as_cancelled() {
        assert_eq!(classify_order(false, "0"), OrderOutcome::Cancelled);
        assert_eq!(classify_order(false, "-1"), OrderOutcome::Cancelled);
    }

    #[test]
    fn classifies_filled() {
        assert_eq!(classify_order(false, "2"), OrderOutcome::Filled);
    }

    #[test]
    fn unparseable_size_retries_instead_of_cancelling() {
        assert_eq!(classify_order(false, "abc"), OrderOutcome::Working);
    }
}
