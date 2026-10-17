use crate::strategy::types::{Direction, Signal, Tick};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimParams {
    pub fee_bps: Decimal,
    pub entry_slip_bps: Decimal,
    pub stop_slip_bps: Decimal, // worse than entry slip (stops hit in fast moves)
    pub borrow_bps_per_day: Decimal,
    pub latency_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimOutcome {
    TakeProfit(Decimal),
    StopLoss(Decimal),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimResult {
    pub outcome: SimOutcome,
    pub entry_ms: i64,
    pub exit_ms: i64,
}

/// Simulate one signal across the FULL tick stream from entry (the caller concatenates
/// ticks across subsequent candles until resolution). Take-profit fills AT its limit price;
/// the stop fills at the stop price, or the tick price if gapped past it, plus slippage.
pub fn simulate(sig: &Signal, ticks: &[Tick], params: SimParams) -> SimResult {
    let risk = match sig.direction {
        Direction::Long => sig.trigger_price - sig.stop_loss,
        Direction::Short => sig.stop_loss - sig.trigger_price,
    };
    let idx = params.latency_ticks as usize;
    if risk <= Decimal::ZERO || idx >= ticks.len() {
        let ms = ticks.first().map(|t| t.time_ms).unwrap_or(0);
        return SimResult {
            outcome: SimOutcome::Unresolved,
            entry_ms: ms,
            exit_ms: ms,
        };
    }
    let raw = ticks[idx].price;
    let slip = raw * params.entry_slip_bps / Decimal::from(10_000);
    let entry = match sig.direction {
        Direction::Long => raw + slip,
        Direction::Short => raw - slip,
    };
    let entry_ms = ticks[idx].time_ms;

    for t in &ticks[idx..] {
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
            // A stopped position is a loss, so R is negative. NB: this is the negation of the
            // plan's `(entry - fill - cost)` expression, which returned a *positive* value and
            // contradicted its own assertions (~-1R mid-bar, ~-2R gapped).
            let r = match sig.direction {
                Direction::Long => (fill - entry + cost) / risk,
                Direction::Short => (entry - fill + cost) / risk,
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
    fn p() -> SimParams {
        SimParams {
            fee_bps: dec("5"),
            entry_slip_bps: dec("5"),
            stop_slip_bps: dec("15"),
            borrow_bps_per_day: dec("2"),
            latency_ticks: 1,
        }
    }

    #[test]
    fn take_profit_fills_at_limit_not_tick_extreme() {
        // TP is 2.15; the last tick is 2.20 (the synthetic extreme). The fill must be AT 2.15.
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
            SimOutcome::TakeProfit(x) => {
                assert!(x > dec("2.9") && x <= dec("3.0"), "clean TP ~3R, got {x}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn midpath_stop_fills_at_stop_price() {
        // The 1.90 tick is NOT a candle open: a normal mid-bar touch fills AT the 1.95 stop (~ -1R).
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
            SimOutcome::StopLoss(x) => assert!(
                x < dec("-1.0") && x > dec("-1.1"),
                "mid-bar stop ~-1R, got {x}"
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn gapped_stop_fills_at_open() {
        // The next candle OPENS at 1.90, already beyond the 1.95 stop -> fill at 1.90 (~ -2R).
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
            SimOutcome::StopLoss(x) => assert!(x < dec("-1.5"), "gap stop ~-2R, got {x}"),
            other => panic!("{other:?}"),
        }
    }
}
