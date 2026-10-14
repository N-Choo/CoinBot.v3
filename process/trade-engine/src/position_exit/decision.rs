//! Pure TP/SL decision logic.
//!
//! No I/O: given a position's side and the last price, decide whether its
//! take-profit or stop-loss has been reached.

use rust_decimal::Decimal;

/// Which protective level a position hit.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ExitReason {
    TakeProfit,
    StopLoss,
}

impl ExitReason {
    /// Short label used in logs.
    pub(super) fn label(&self) -> &'static str {
        match self {
            ExitReason::TakeProfit => "TP",
            ExitReason::StopLoss => "SL",
        }
    }
}

/// Decide whether `side`'s position has reached `tp` or `sl` at price `last`.
///
/// Long (`buy`) exits high on TP / low on SL; short (`sell`) is inverted.
pub(super) fn evaluate_exit(
    side: &str,
    last: Decimal,
    tp: Decimal,
    sl: Decimal,
) -> Option<ExitReason> {
    match side {
        "buy" => {
            if last >= tp {
                Some(ExitReason::TakeProfit)
            } else if last <= sl {
                Some(ExitReason::StopLoss)
            } else {
                None
            }
        }
        "sell" => {
            if last <= tp {
                Some(ExitReason::TakeProfit)
            } else if last >= sl {
                Some(ExitReason::StopLoss)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn long_exits_on_tp_and_sl() {
        let (tp, sl) = (dec("110"), dec("90"));
        assert_eq!(
            evaluate_exit("buy", dec("110"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("buy", dec("120"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("buy", dec("90"), tp, sl),
            Some(ExitReason::StopLoss)
        );
        assert_eq!(evaluate_exit("buy", dec("100"), tp, sl), None);
    }

    #[test]
    fn short_exits_on_tp_and_sl() {
        let (tp, sl) = (dec("90"), dec("110"));
        assert_eq!(
            evaluate_exit("sell", dec("90"), tp, sl),
            Some(ExitReason::TakeProfit)
        );
        assert_eq!(
            evaluate_exit("sell", dec("110"), tp, sl),
            Some(ExitReason::StopLoss)
        );
        assert_eq!(evaluate_exit("sell", dec("100"), tp, sl), None);
    }

    #[test]
    fn unknown_side_never_exits() {
        assert_eq!(evaluate_exit("hold", dec("999"), dec("1"), dec("2")), None);
    }
}
