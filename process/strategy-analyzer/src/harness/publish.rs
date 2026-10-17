//! Publisher boundary (Phase 2): the **only** place core `Decimal` prices cross into the engine's
//! `f64` contract (`lib/share/src/models/signals.rs::TradeSignal`, live spec §7).
//!
//! Core logic and persisted state stay `Decimal`; the engine contract is `f64`. This module exists
//! so that the lossy `Decimal -> f64` step is explicit, centralized, and pinned by a round-trip test
//! rather than sprinkled through the mapping.
//!
//! **Known engine defect — must be fixed engine-side before Phase 2.** The trade engine sizes the
//! order by quantizing the **quantity** to the symbol's base increment
//! (`process/trade-engine/src/trader/mod.rs::compute_quantity(.., &info.base_increment, ..)`), but
//! it passes the entry / stop / take-profit **prices** through to the exchange **unquantized**
//! (`TradeSignal::{entry_price, sl_price, tp_price}`). The analyzer rounds SL/TP to its configured
//! `price_scale` (from the symbol's `priceIncrement`), which may be finer or coarser than the
//! exchange's own increment, so the engine can receive a price the exchange will reject or silently
//! re-quote. Until the engine quantizes prices, closing that gap is engine-side work — not an
//! analyzer change.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// `Decimal -> f64` at the publish edge (live spec §7). `None` only when the value cannot be
/// represented as an `f64` (out of range): a caller must skip publishing such a signal rather than
/// substitute a default.
pub fn price_f64(value: Decimal) -> Option<f64> {
    value.to_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::decimal::round_price;
    use crate::strategy::types::{Direction, Signal, TrendState};
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// A minimal long `Signal`: only the three price fields matter to the publisher boundary; the
    /// rest are fixed placeholders.
    fn signal(entry: Decimal, stop: Decimal, tp: Decimal) -> Signal {
        Signal {
            schema_version: 1,
            event_type: "strategy.entry_signal".into(),
            signal_id: "z:0".into(),
            strategy: "supply_demand_4h".into(),
            strategy_version: "0.1.0".into(),
            config_version: "x".into(),
            symbol: "XRP-USDT".into(),
            market: "margin".into(),
            direction: Direction::Long,
            timeframe: "4hour".into(),
            trend_state: TrendState::Bullish,
            zone_id: "z".into(),
            zone_low: stop,
            zone_high: entry,
            zone_height_atr: dec("0.4"),
            atr: dec("0.1"),
            trigger_price: entry,
            reference_entry_price: entry,
            stop_loss: stop,
            take_profit: tp,
            target_mode: "fixed_r".into(),
            target_r: dec("3.0"),
            min_gross_rr: dec("3.0"),
            gross_rr: dec("3.0"),
            structure_obstacle_price: None,
            obstacle_rr: None,
            triggered_at_ms: 0,
            candle_timestamp_ms: 0,
        }
    }

    /// The publisher's `Decimal -> f64 -> Decimal` round-trip for a signal's three price fields.
    fn round_trip(sig: &Signal) -> (Decimal, Decimal, Decimal) {
        let back = |v: f64| Decimal::from_f64(v).expect("f64 is finite");
        (
            back(price_f64(sig.reference_entry_price).unwrap()),
            back(price_f64(sig.stop_loss).unwrap()),
            back(price_f64(sig.take_profit).unwrap()),
        )
    }

    /// Prices quoted at several symbol scales (incl. a large-tick symbol at scale 0) survive the
    /// publisher boundary: re-rounding the round-tripped value to the quote's own scale recovers it
    /// exactly, because the `f64` drift is far below one tick.
    #[test]
    fn publisher_boundary_preserves_price_at_its_scale() {
        let cases: &[(u32, &str, &str, &str)] = &[
            (0, "104", "99", "110"),        // large-tick symbol (whole units)
            (1, "104.5", "103.9", "106.3"), // the 4H happy fixture's 1 dp scale
            (2, "104.25", "103.75", "106.75"),
            (4, "104.5625", "103.4375", "106.1875"),
            (8, "104.00390625", "103.00195312", "106.0078125"),
        ];
        for &(scale, e, s, t) in cases {
            let sig = signal(dec(e), dec(s), dec(t));
            let (re, rs, rt) = round_trip(&sig);
            assert_eq!(
                round_price(re, scale),
                dec(e),
                "entry drift at scale {scale}"
            );
            assert_eq!(
                round_price(rs, scale),
                dec(s),
                "stop drift at scale {scale}"
            );
            assert_eq!(round_price(rt, scale), dec(t), "tp drift at scale {scale}");
        }
    }

    /// Binary-exact (dyadic) quotes survive bit-for-bit: the round-trip has zero drift.
    #[test]
    fn publisher_boundary_is_exact_for_dyadic_prices() {
        let cases = [
            ("104.5", "103.75", "106.25"),
            ("104.5625", "103.4375", "106.1875"),
        ];
        for (e, s, t) in cases {
            let sig = signal(dec(e), dec(s), dec(t));
            assert_eq!(
                round_trip(&sig),
                (dec(e), dec(s), dec(t)),
                "dyadic prices must round-trip exactly"
            );
        }
    }

    /// For non-dyadic quotes the raw conversion may differ in the last binary digits; document that
    /// the drift is far below half the finest tick (so it can never cross a price increment).
    #[test]
    fn non_dyadic_drift_stays_below_half_a_tick() {
        let sig = signal(dec("104.5"), dec("103.9"), dec("106.3"));
        let (re, rs, rt) = round_trip(&sig);
        for (orig, back) in [(dec("104.5"), re), (dec("103.9"), rs), (dec("106.3"), rt)] {
            assert!(
                (back - orig).abs() < dec("0.001"),
                "drift {back} vs {orig} is not below half a tick"
            );
        }
    }
}
