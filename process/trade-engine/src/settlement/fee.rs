//! Terminal payout and global performance-fee math. Pure, no I/O.

use rust_decimal::Decimal;

/// Result of settling one contract.
#[derive(Debug, Clone, PartialEq)]
pub struct Payout {
    pub net_equity: Decimal,
    pub profit: Decimal,
    pub platform_fee: Decimal,
    pub payout: Decimal,
}

/// 5% of net profit, charged only when the contract ends positive.
pub fn compute(available_fund: Decimal, borrow_interest: Decimal, init_fund: Decimal) -> Payout {
    let net_equity = available_fund - borrow_interest;
    let profit = net_equity - init_fund;
    let platform_fee = if profit > Decimal::ZERO {
        (profit * Decimal::new(5, 2)).round_dp(8).min(profit)
    } else {
        Decimal::ZERO
    };
    let payout = (net_equity - platform_fee).max(Decimal::ZERO).round_dp(8);
    Payout {
        net_equity,
        profit,
        platform_fee,
        payout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn takes_five_percent_of_positive_net_profit() {
        let p = compute(dec("1180"), dec("0"), dec("1000"));
        assert_eq!(p.profit, dec("180"));
        assert_eq!(p.platform_fee, dec("9"));
        assert_eq!(p.payout, dec("1171"));
    }

    #[test]
    fn charges_nothing_on_a_loss() {
        let p = compute(dec("940"), dec("0"), dec("1000"));
        assert_eq!(p.profit, dec("-60"));
        assert_eq!(p.platform_fee, Decimal::ZERO);
        assert_eq!(p.payout, dec("940"));
    }

    #[test]
    fn subtracts_borrow_interest_before_fee() {
        let p = compute(dec("1180"), dec("20"), dec("1000"));
        assert_eq!(p.net_equity, dec("1160"));
        assert_eq!(p.profit, dec("160"));
        assert_eq!(p.platform_fee, dec("8"));
        assert_eq!(p.payout, dec("1152"));
    }

    #[test]
    fn floors_negative_equity_at_zero() {
        let p = compute(dec("10"), dec("25"), dec("1000"));
        assert_eq!(p.net_equity, dec("-15"));
        assert_eq!(p.platform_fee, Decimal::ZERO);
        assert_eq!(p.payout, Decimal::ZERO);
    }

    #[test]
    fn zero_profit_has_no_fee() {
        let p = compute(dec("1000"), dec("0"), dec("1000"));
        assert_eq!(p.platform_fee, Decimal::ZERO);
        assert_eq!(p.payout, dec("1000"));
    }
}
