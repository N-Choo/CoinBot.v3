use crate::strategy::types::{Direction, Signal, Tick};
use rust_decimal::Decimal;

/// Fill-model assumptions for the position simulation.
///
/// **Latency is a fill-model assumption, not a tick index.** The caller passes the touching tick as
/// `ticks[0]` and the entry fills at that tick's price plus `entry_slip_bps`. Phase 0 runs on 4H
/// synthesis (~4 synthetic ticks/candle), where a true sub-candle latency is not recoverable, so a
/// live feed would instead inject a **time-based** latency model here (spec §8.4); the 4H-synthesis
/// results are therefore labelled latency-insensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimParams {
    pub fee_bps: Decimal,
    pub entry_slip_bps: Decimal,
    pub stop_slip_bps: Decimal, // worse than entry slip (stops hit in fast moves)
    pub borrow_bps_per_day: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimOutcome {
    TakeProfit(Decimal),
    StopLoss(Decimal),
    /// No resolution within the supplied tick stream: the position is still open at the end of data.
    Unresolved,
    /// The entry price is already at/beyond the stop or the target, so the signal cannot be filled:
    /// emitting a "take profit" at a loss (or a "stop" at a gain) would be bogus.
    Unfillable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimResult {
    pub outcome: SimOutcome,
    pub entry_ms: i64,
    pub exit_ms: i64,
}

/// Simulate one signal across the FULL tick stream from entry (the caller concatenates ticks across
/// subsequent candles until resolution). `ticks[0]` is the **touching tick**; the caller passes that
/// tick first and every later tick after it. Entry = `ticks[0].price` + `entry_slip_bps` (long: +;
/// short: −). Take-profit fills AT its limit price; the stop fills at the stop price, or at the tick
/// price when a candle opens already past it (a gap), plus `stop_slip_bps`.
pub fn simulate(sig: &Signal, ticks: &[Tick], params: SimParams) -> SimResult {
    let risk = match sig.direction {
        Direction::Long => sig.trigger_price - sig.stop_loss,
        Direction::Short => sig.stop_loss - sig.trigger_price,
    };
    let first = match ticks.first() {
        Some(t) => t,
        None => {
            return SimResult {
                outcome: SimOutcome::Unresolved,
                entry_ms: 0,
                exit_ms: 0,
            }
        }
    };
    if risk <= Decimal::ZERO {
        return SimResult {
            outcome: SimOutcome::Unresolved,
            entry_ms: first.time_ms,
            exit_ms: first.time_ms,
        };
    }
    let raw = first.price;
    let slip = raw * params.entry_slip_bps / Decimal::from(10_000);
    let entry = match sig.direction {
        Direction::Long => raw + slip,
        Direction::Short => raw - slip,
    };
    let entry_ms = first.time_ms;

    // Reject an unfillable entry: if the entry is already at/beyond the stop or the target there is
    // no valid fill. Without this a long whose entry gapped past its TP would report TakeProfit at a
    // loss (and one that opened through its stop would report StopLoss at a gain).
    let unfillable = match sig.direction {
        Direction::Long => entry <= sig.stop_loss || entry >= sig.take_profit,
        Direction::Short => entry >= sig.stop_loss || entry <= sig.take_profit,
    };
    if unfillable {
        return SimResult {
            outcome: SimOutcome::Unfillable,
            entry_ms,
            exit_ms: entry_ms,
        };
    }

    for t in ticks {
        let (hit_stop, hit_tp) = match sig.direction {
            Direction::Long => (t.price <= sig.stop_loss, t.price >= sig.take_profit),
            Direction::Short => (t.price >= sig.stop_loss, t.price <= sig.take_profit),
        };
        let days = Decimal::from((t.time_ms - entry_ms).max(0)) / Decimal::from(86_400_000);
        let cost = entry * (params.fee_bps * Decimal::from(2) + params.borrow_bps_per_day * days)
            / Decimal::from(10_000);
        if hit_stop {
            // Normal mid-bar touch fills AT the stop; only a tick that is a candle open already
            // beyond the stop (a gap) fills at the tick price.
            let gapped = t.is_bar_open
                && match sig.direction {
                    Direction::Long => t.price < sig.stop_loss,
                    Direction::Short => t.price > sig.stop_loss,
                };
            let base = if gapped { t.price } else { sig.stop_loss };
            let s = base * params.stop_slip_bps / Decimal::from(10_000);
            let fill = match sig.direction {
                Direction::Long => base - s,
                Direction::Short => base + s,
            };
            // A stop is a loss and costs WORSEN it. Long stop R = (fill − entry − cost)/risk;
            // short stop R = (entry − fill − cost)/risk (spec §8.4). (The plan's `(entry − fill −
            // cost)` long form returned a *positive* value and contradicted its own assertions.)
            let r = match sig.direction {
                Direction::Long => (fill - entry - cost) / risk,
                Direction::Short => (entry - fill - cost) / risk,
            };
            return SimResult {
                outcome: SimOutcome::StopLoss(r),
                entry_ms,
                exit_ms: t.time_ms,
            };
        }
        if hit_tp {
            let fill = sig.take_profit; // limit price, not the tick extreme
            let r = match sig.direction {
                Direction::Long => (fill - entry - cost) / risk,
                Direction::Short => (entry - fill - cost) / risk,
            };
            return SimResult {
                outcome: SimOutcome::TakeProfit(r),
                entry_ms,
                exit_ms: t.time_ms,
            };
        }
    }
    SimResult {
        outcome: SimOutcome::Unresolved,
        entry_ms,
        exit_ms: ticks.last().map(|t| t.time_ms).unwrap_or(entry_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::{Direction, Signal, TrendState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    /// A long signal: trigger 2.00, stop 1.95, TP 2.15 -> risk 0.05.
    fn long_sig() -> Signal {
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
            zone_low: dec("1.96"),
            zone_high: dec("2.00"),
            zone_height_atr: dec("0.4"),
            atr: dec("0.1"),
            trigger_price: dec("2.00"),
            reference_entry_price: dec("2.00"),
            stop_loss: dec("1.95"),
            take_profit: dec("2.15"),
            target_mode: "fixed_r".into(),
            target_r: dec("3.0"),
            min_gross_rr: dec("3.0"),
            gross_rr: dec("3.0"),
            structure_obstacle_price: Some(dec("2.5")),
            obstacle_rr: Some(dec("10")),
            triggered_at_ms: 0,
            candle_timestamp_ms: 0,
        }
    }
    /// Pre-registered cost shape: 5 bps fee/side, 5 bps entry slip, 15 bps (worse) stop slip.
    /// **Borrow is set to 0** so every R below is an exact hand-derived `Decimal`; borrow interest
    /// is a time-scaled term that changes no sign and is exercised through `run`/reporting.
    fn p() -> SimParams {
        SimParams {
            fee_bps: dec("5"),
            entry_slip_bps: dec("5"),
            stop_slip_bps: dec("15"),
            borrow_bps_per_day: Decimal::ZERO,
        }
    }

    #[test]
    fn clean_take_profit_is_about_three_r() {
        // ticks[0] = 2.00 (the touching tick). entry = 2.00 + 2.00*5bps = 2.001.
        // cost = entry*(2*5bps) = 2.001*0.001 = 0.002001 (borrow 0).
        // TP 2.15 is reached at t=2. R = (2.15 - 2.001 - 0.002001)/0.05 = 0.146999/0.05 = 2.93998.
        let t = vec![
            Tick {
                time_ms: 0,
                price: dec("2.00"),
                is_bar_open: true,
            },
            Tick {
                time_ms: 1,
                price: dec("2.00"),
                is_bar_open: false,
            },
            Tick {
                time_ms: 2,
                price: dec("2.20"),
                is_bar_open: false,
            },
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::TakeProfit(x) => assert_eq!(x, dec("2.93998"), "hand-derived 3R winner"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mid_bar_stop_fills_at_stop_price_with_costs() {
        // entry = 2.001. The 1.90 tick is NOT a candle open, so a mid-bar touch fills AT the 1.95
        // stop with 15 bps slip: fill = 1.95 - 1.95*15bps = 1.947075. cost = 0.002001.
        // R = (1.947075 - 2.001 - 0.002001)/0.05 = -0.055926/0.05 = -1.11852.
        let t = vec![
            Tick {
                time_ms: 0,
                price: dec("2.00"),
                is_bar_open: true,
            },
            Tick {
                time_ms: 1,
                price: dec("2.00"),
                is_bar_open: false,
            },
            Tick {
                time_ms: 2,
                price: dec("1.90"),
                is_bar_open: false,
            },
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::StopLoss(x) => assert_eq!(x, dec("-1.11852"), "hand-derived mid-bar stop"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn later_candle_gapped_stop_fills_at_the_open() {
        // entry = 2.001. The last tick OPENS at 1.90 (< 1.95 stop) -> a gap fill at the tick:
        // fill = 1.90 - 1.90*15bps = 1.89715. cost = 0.002001.
        // R = (1.89715 - 2.001 - 0.002001)/0.05 = -0.105851/0.05 = -2.11702.
        let t = vec![
            Tick {
                time_ms: 0,
                price: dec("2.00"),
                is_bar_open: true,
            },
            Tick {
                time_ms: 1,
                price: dec("2.00"),
                is_bar_open: false,
            },
            Tick {
                time_ms: 2,
                price: dec("1.90"),
                is_bar_open: true,
            },
        ];
        match simulate(&long_sig(), &t, p()).outcome {
            SimOutcome::StopLoss(x) => assert_eq!(x, dec("-2.11702"), "hand-derived gapped stop"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn entry_already_at_or_beyond_the_stop_is_unfillable() {
        // ticks[0] = 1.94 (below the 1.95 stop): entry = 1.94*1.0005 = 1.940970 <= stop -> unfillable.
        let t = vec![Tick {
            time_ms: 0,
            price: dec("1.94"),
            is_bar_open: true,
        }];
        assert_eq!(
            simulate(&long_sig(), &t, p()).outcome,
            SimOutcome::Unfillable
        );
    }

    #[test]
    fn entry_already_at_or_beyond_the_target_is_unfillable() {
        // ticks[0] = 2.15 (= TP): entry = 2.15*1.0005 = 2.151075 >= TP -> unfillable (never a
        // negative-R "take profit").
        let t = vec![Tick {
            time_ms: 0,
            price: dec("2.15"),
            is_bar_open: true,
        }];
        assert_eq!(
            simulate(&long_sig(), &t, p()).outcome,
            SimOutcome::Unfillable
        );
    }
}
