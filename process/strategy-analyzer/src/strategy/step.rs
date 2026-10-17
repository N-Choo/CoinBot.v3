use crate::strategy::config::Config;
use crate::strategy::error::StrategyError;
use crate::strategy::structure::{
    apply_bos, detect_bos, newly_confirmed_high, newly_confirmed_low,
};
use crate::strategy::types::*;
use crate::strategy::zones::{create_zone, invalidate_on_flip, update_lifecycle};
use rust_decimal::Decimal;

pub(crate) const TIMEFRAME_MS: i64 = 4 * 60 * 60 * 1000;

/// Pure step. `state` is borrowed; on error the caller's state is untouched
/// (validate-then-commit: a new state is produced only on success).
pub fn step(
    state: &StrategyState,
    input: Input,
    cfg: &Config,
) -> Result<(StrategyState, Vec<Output>), StrategyError> {
    cfg.validate()?;
    // Per-tick state clone is O(window): the rolling window is bounded (WINDOW_CANDLES), so this is
    // not O(history). Left as-is; revisit only if 1m profiling shows the clone dominates.
    let mut next = state.clone();
    let outputs = match input {
        Input::PriceTick(t) => on_tick(&mut next, t, cfg)?,
        Input::ClosedCandle(c) => on_candle(&mut next, c, cfg)?,
    };
    Ok((next, outputs))
}

fn on_tick(state: &mut StrategyState, t: Tick, cfg: &Config) -> Result<Vec<Output>, StrategyError> {
    if t.price <= Decimal::ZERO {
        return Err(StrategyError::MalformedTick {
            price: t.price.to_string(),
        });
    }
    let last = state.last_tick_time_ms.or(state.last_candle_time_ms);
    if let Some(prev) = last {
        if t.time_ms < prev {
            return Err(StrategyError::OutOfOrderTick {
                got_ms: t.time_ms,
                last_ms: prev,
            });
        }
    }
    let outputs =
        crate::strategy::signals::try_entry(state, state.last_price, t.time_ms, t.price, cfg)?;
    state.last_tick_time_ms = Some(t.time_ms);
    state.last_price = Some(t.price);
    Ok(outputs)
}

fn on_candle(
    state: &mut StrategyState,
    c: Candle,
    cfg: &Config,
) -> Result<Vec<Output>, StrategyError> {
    if c.high < c.low || c.high <= Decimal::ZERO || c.low <= Decimal::ZERO {
        return Err(StrategyError::MalformedCandle {
            at_ms: c.open_time_ms,
            why: "bad range/price",
        });
    }
    if let Some(prev) = state.last_candle_time_ms {
        if c.open_time_ms == prev {
            return Err(StrategyError::DuplicateCandle(c.open_time_ms));
        }
        if c.open_time_ms < prev {
            return Err(StrategyError::OutOfOrderCandle {
                got_ms: c.open_time_ms,
                prev_ms: prev,
            });
        }
        if c.open_time_ms != prev + TIMEFRAME_MS {
            return Err(StrategyError::GappedCandle {
                expected_ms: prev + TIMEFRAME_MS,
                got_ms: c.open_time_ms,
            });
        }
    }
    let mut out = Vec::new();

    // ATR through the PREVIOUS candle is the no-look-ahead value used for BOS/zone `A`.
    let atr_before = state.atr;

    // Record into the bounded window and advance counters.
    state.window.push_back(c);
    if state.window.len() > crate::strategy::types::WINDOW_CANDLES {
        state.window.pop_front();
    }
    state.candle_count += 1;
    state.last_candle_time_ms = Some(c.open_time_ms);

    // Update window extremes for every live zone (window = breakout candle through latest close).
    for z in state.zones.iter_mut() {
        if z.state == ZoneState::Active {
            if c.high > z.window_high {
                z.window_high = c.high;
            }
            if c.low < z.window_low {
                z.window_low = c.low;
            }
        }
    }

    // Running ATR (incremental; fields live on the state).
    let mut atr_state = crate::strategy::indicators::atr::AtrState {
        value: state.atr,
        seed_sum: state.atr_seed_sum,
        seen: state.atr_seen,
        period: cfg.atr_period,
    };
    let atr_now = atr_state.update(state.prev_close, c.high, c.low);
    state.atr = atr_state.value;
    state.atr_seed_sum = atr_state.seed_sum;
    state.atr_seen = atr_state.seen;

    let atr_is_zero = atr_now == Some(Decimal::ZERO);
    if atr_is_zero {
        // In-band: ATR == 0 skips ONLY BOS + zone creation. Pivots and lifecycle still run.
        out.push(Output::StateChange { reason: "atr_zero" });
    }

    // Pivots (absolute index = candle_count - 1 - right).
    let window: Vec<Candle> = state.window.iter().copied().collect();
    let (left, right) = (cfg.pivot_left as usize, cfg.pivot_right as usize);
    if let Some(mut p) = newly_confirmed_high(&window, left, right) {
        p.index = state.candle_count - 1 - right as u64;
        state.swing_highs.push(p);
    }
    if let Some(mut p) = newly_confirmed_low(&window, left, right) {
        p.index = state.candle_count - 1 - right as u64;
        state.swing_lows.push(p);
    }

    // BOS + zones (skipped only when ATR == 0).
    if !atr_is_zero {
        if let Some(atr_b) = atr_before {
            if let Some((kind, broken_idx)) = detect_bos(state, atr_b, cfg) {
                let before = state.trend;
                apply_bos(state, kind, broken_idx);
                if state.trend != before {
                    out.extend(
                        invalidate_on_flip(&mut state.zones, state.trend)
                            .into_iter()
                            .map(Output::ZoneEvent),
                    );
                }
                match create_zone(&state.window, kind, atr_b, &state.symbol.clone(), cfg) {
                    Ok(mut z) => {
                        z.activated_at_index = state.candle_count - 1;
                        state.zones_awaiting_first_tick.push(z.id.clone());
                        out.push(Output::ZoneEvent(ZoneEvent {
                            zone_id: z.id.clone(),
                            kind: ZoneEventKind::Created,
                            reason: None,
                        }));
                        state.zones.push(z);
                    }
                    Err(tag) => out.push(Output::StateChange { reason: tag }),
                }
            }
        }
    }

    // Lifecycle (expiry + close-beyond-edge) for the just-closed candle.
    out.extend(
        update_lifecycle(
            &mut state.zones,
            &c,
            state.candle_count - 1,
            cfg.zone_expiry_candles,
        )
        .into_iter()
        .map(Output::ZoneEvent),
    );

    // Bound state growth so per-tick cloning stays small at 1m volume: drop terminal zones and
    // broken pivots (never needed again), and age out unbroken pivots that have fallen out of the
    // retained candle window. Keep only *eligible* pivots: a pivot is eligible while its absolute
    // index is still inside the last `WINDOW_CANDLES` candles, so a long trend cannot grow
    // `swing_highs`/`swing_lows` without bound. Then clear any valid index whose pivot was pruned.
    state.zones.retain(|z| !z.is_terminal());
    let cutoff = state.candle_count.saturating_sub(WINDOW_CANDLES as u64);
    state.swing_highs.retain(|p| !p.broken && p.index >= cutoff);
    state.swing_lows.retain(|p| !p.broken && p.index >= cutoff);

    // Prune stale ids whose zone no longer exists / is no longer Active.
    let live: Vec<String> = state
        .zones
        .iter()
        .filter(|z| z.state == ZoneState::Active)
        .map(|z| z.id.clone())
        .collect();
    state
        .zones_awaiting_first_tick
        .retain(|id| live.contains(id));

    if state
        .valid_low_idx
        .is_some_and(|i| !state.swing_lows.iter().any(|p| p.index == i))
    {
        state.valid_low_idx = None;
    }
    if state
        .valid_high_idx
        .is_some_and(|i| !state.swing_highs.iter().any(|p| p.index == i))
    {
        state.valid_high_idx = None;
    }

    state.prev_close = Some(c.close);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn candle(ms: i64) -> Candle {
        Candle {
            open_time_ms: ms,
            open: dec("1.0"),
            high: dec("1.2"),
            low: dec("0.9"),
            close: dec("1.1"),
        }
    }

    #[test]
    fn accepts_contiguous_candles() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let (s, _) = step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS)), &cfg).unwrap();
        assert_eq!(s.window.len(), 2);
    }

    #[test]
    fn rejects_duplicate_and_out_of_order() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(
            step(&s, Input::ClosedCandle(candle(0)), &cfg),
            Err(StrategyError::DuplicateCandle(_))
        ));
        assert!(matches!(
            step(&s, Input::ClosedCandle(candle(-5)), &cfg),
            Err(StrategyError::OutOfOrderCandle { .. })
        ));
    }

    #[test]
    fn rejects_gap() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(matches!(
            step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS * 2)), &cfg),
            Err(StrategyError::GappedCandle { .. })
        ));
    }

    #[test]
    fn error_leaves_state_unchanged() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        let (s, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        let snapshot = s.clone();
        let _ = step(&s, Input::ClosedCandle(candle(0)), &cfg);
        assert_eq!(s, snapshot);
    }

    #[test]
    fn rejects_malformed_and_out_of_order_tick() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let s = StrategyState::new("XRP-USDT", 1);
        assert!(matches!(
            step(
                &s,
                Input::PriceTick(Tick {
                    time_ms: 1,
                    price: dec("-1"),
                    is_bar_open: false
                }),
                &cfg
            ),
            Err(StrategyError::MalformedTick { .. })
        ));
        let (s, _) = step(
            &s,
            Input::PriceTick(Tick {
                time_ms: 10,
                price: dec("1.0"),
                is_bar_open: true,
            }),
            &cfg,
        )
        .unwrap();
        assert!(matches!(
            step(
                &s,
                Input::PriceTick(Tick {
                    time_ms: 5,
                    price: dec("1.0"),
                    is_bar_open: false
                }),
                &cfg
            ),
            Err(StrategyError::OutOfOrderTick { .. })
        ));
    }

    #[test]
    fn atr_zero_is_in_band_not_error() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        let flat = Candle {
            open_time_ms: 0,
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
        };
        for i in 0..14 {
            let (ns, outs) = step(
                &s,
                Input::ClosedCandle(Candle {
                    open_time_ms: i * TIMEFRAME_MS,
                    ..flat
                }),
                &cfg,
            )
            .unwrap();
            s = ns;
            if outs
                .iter()
                .any(|o| matches!(o, Output::StateChange { reason: "atr_zero" }))
            {
                // Candle still fully processed: counter and window advanced (no early return).
                assert_eq!(s.candle_count, (i + 1) as u64);
                assert_eq!(s.window.len(), (i + 1) as usize);
                return;
            }
        }
        panic!("expected atr_zero state change");
    }

    #[test]
    fn old_unbroken_pivots_are_aged_out() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.candle_count = 100;
        s.last_candle_time_ms = Some(0);
        s.window.push_back(candle(0));
        // index 5 is outside the retained 64-candle window for candle_count 100; index 99 is inside.
        s.swing_highs = vec![Pivot {
            index: 5,
            time_ms: 0,
            price: dec("1.2"),
            broken: false,
        }];
        s.swing_lows = vec![Pivot {
            index: 99,
            time_ms: 0,
            price: dec("0.9"),
            broken: false,
        }];
        s.valid_low_idx = Some(5); // points at the aged-out pivot
        let (ns, _) = step(&s, Input::ClosedCandle(candle(TIMEFRAME_MS)), &cfg).unwrap();
        assert!(ns.swing_highs.is_empty(), "old unbroken high aged out");
        assert_eq!(ns.swing_lows.len(), 1, "in-window pivot kept");
        assert_eq!(ns.swing_lows[0].index, 99);
        assert_eq!(ns.valid_low_idx, None, "valid index cleared with its pivot");
    }

    #[test]
    fn stale_awaiting_first_tick_ids_are_pruned() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.zones_awaiting_first_tick = vec!["gone".into()];
        let (ns, _) = step(&s, Input::ClosedCandle(candle(0)), &cfg).unwrap();
        assert!(ns.zones_awaiting_first_tick.is_empty());
    }

    #[test]
    fn step_rejects_invalid_config() {
        let mut cfg = Config::default_for(&["XRP-USDT"]);
        cfg.target_r = dec("1.0"); // below engine_net_floor -> invalid
        let s = StrategyState::new("XRP-USDT", 1);
        assert!(matches!(
            step(&s, Input::ClosedCandle(candle(0)), &cfg),
            Err(StrategyError::ConfigInvalid(_))
        ));
    }

    #[test]
    fn signal_serializes_with_string_decimals() {
        use crate::strategy::types::{Direction, Signal, TrendState};
        let sig = Signal {
            schema_version: 1,
            event_type: "strategy.entry_signal".into(),
            signal_id: "z:0".into(),
            strategy: "supply_demand_4h".into(),
            strategy_version: "0.1.0".into(),
            config_version: "abc".into(),
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
            structure_obstacle_price: Some(dec("2.30")),
            obstacle_rr: Some(dec("6.0")),
            triggered_at_ms: 1,
            candle_timestamp_ms: 0,
        };
        let j = serde_json::to_string(&sig).unwrap();
        assert!(
            j.contains("\"stop_loss\":\"1.95\""),
            "decimals must serialize as strings: {j}"
        );
    }
}
