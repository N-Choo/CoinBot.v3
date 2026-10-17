use crate::strategy::config::{Config, TargetMode};
use crate::strategy::decimal::{round_price, round_ratio_for_output, round_to};
use crate::strategy::error::StrategyError;
use crate::strategy::types::*;
use rust_decimal::Decimal;

pub struct Levels {
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub gross_rr: Decimal,
    pub obstacle: Option<Decimal>,
    pub obstacle_rr: Option<Decimal>,
}

pub fn obstacle_for_long(
    trigger: Decimal,
    swing_highs: &[Pivot],
    active_supply: &[Zone],
    window_high: Decimal,
) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(
        swing_highs
            .iter()
            .filter(|p| !p.broken && p.price > trigger)
            .map(|p| p.price),
    );
    c.extend(
        active_supply
            .iter()
            .filter(|z| z.low > trigger)
            .map(|z| z.low),
    );
    if window_high > trigger {
        c.push(window_high);
    } // window includes the breakout candle
    c.into_iter().min()
}

pub fn obstacle_for_short(
    trigger: Decimal,
    swing_lows: &[Pivot],
    active_demand: &[Zone],
    window_low: Decimal,
) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(
        swing_lows
            .iter()
            .filter(|p| !p.broken && p.price < trigger)
            .map(|p| p.price),
    );
    c.extend(
        active_demand
            .iter()
            .filter(|z| z.high < trigger)
            .map(|z| z.high),
    );
    if window_low < trigger {
        c.push(window_low);
    }
    c.into_iter().max()
}

/// Rounded levels + the mode's gate. `Err(tag)` = veto, `tag` names the reason
/// (`risk_nonpositive` / `obstacle_none` / `obstacle_too_close`).
pub fn compute_levels(
    direction: Direction,
    zone: &Zone,
    trigger: Decimal,
    obstacle: Option<Decimal>,
    price_scale: u32,
    cfg: &Config,
) -> Result<Levels, &'static str> {
    let t = round_price(trigger, price_scale);
    let buffer = cfg.stop_buffer_atr * zone.atr;
    let (stop_raw, risk) = match direction {
        Direction::Long => (zone.low - buffer, t - (zone.low - buffer)),
        Direction::Short => (zone.high + buffer, (zone.high + buffer) - t),
    };
    if risk <= Decimal::ZERO {
        return Err("risk_nonpositive");
    }
    let stop = round_price(stop_raw, price_scale);
    let risk = match direction {
        Direction::Long => t - stop,
        Direction::Short => stop - t,
    };
    if risk <= Decimal::ZERO {
        return Err("risk_nonpositive");
    }

    let obstacle_rounded = obstacle.map(|o| round_price(o, price_scale));
    let obstacle_rr = obstacle_rounded.map(|o| match direction {
        Direction::Long => (o - t) / risk,
        Direction::Short => (t - o) / risk,
    });

    // The per-mode gross is only used as a gate; the payload gross is recomputed from the
    // rounded levels below (so the `_gross_rr` value here is intentionally unused).
    let (tp, _gross_rr) = match cfg.target_mode {
        TargetMode::FixedR => {
            // Gate = obstacle veto only (no tautological gross gate).
            if let Some(rr) = obstacle_rr {
                if rr < cfg.target_r {
                    return Err("obstacle_too_close");
                }
            } else if !cfg.allow_no_obstacle {
                return Err("obstacle_none");
            }
            let tp = match direction {
                Direction::Long => t + cfg.target_r * risk,
                Direction::Short => t - cfg.target_r * risk,
            };
            (round_price(tp, price_scale), cfg.target_r)
        }
        TargetMode::Structural => {
            let o = match obstacle_rounded {
                Some(o) => o,
                None => {
                    if !cfg.allow_no_obstacle {
                        return Err("obstacle_none");
                    }
                    let tp = match direction {
                        Direction::Long => t + cfg.target_r * risk,
                        Direction::Short => t - cfg.target_r * risk,
                    };
                    return Ok(Levels {
                        stop_loss: stop,
                        take_profit: round_price(tp, price_scale),
                        gross_rr: cfg.target_r,
                        obstacle: None,
                        obstacle_rr: None,
                    });
                }
            };
            let buf = round_price(cfg.structural_buffer_atr * zone.atr, price_scale);
            let tp = match direction {
                Direction::Long => o - buf,
                Direction::Short => o + buf,
            };
            let tp = round_price(tp, price_scale);
            let gross = match direction {
                Direction::Long => (tp - t) / risk,
                Direction::Short => (t - tp) / risk,
            };
            if gross < cfg.min_gross_rr {
                return Err("obstacle_too_close");
            }
            (tp, gross)
        }
    };
    // Recompute gross from the rounded levels that go into the payload.
    let gross = match direction {
        Direction::Long => (tp - t) / risk,
        Direction::Short => (t - tp) / risk,
    };
    Ok(Levels {
        stop_loss: stop,
        take_profit: tp,
        gross_rr: gross,
        obstacle: obstacle_rounded,
        obstacle_rr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::ZoneState;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn zone(kind: ZoneKind, low: &str, high: &str) -> Zone {
        Zone {
            id: "z".into(),
            kind,
            low: dec(low),
            high: dec(high),
            atr: dec("0.1"),
            breakout_time_ms: 0,
            zone_candle_time_ms: 0,
            activated_at_index: 0,
            state: ZoneState::Active,
            terminal_reason: None,
            consume_reason: None,
            attempt_made: false,
            window_high: dec(high),
            window_low: dec(low),
        }
    }

    #[test]
    fn long_levels_and_obstacle_veto() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let ok =
            compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.20")), 2, &cfg).unwrap();
        assert_eq!(ok.stop_loss, dec("1.95"));
        assert_eq!(ok.take_profit, dec("2.15"));
        assert!(
            compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.05")), 2, &cfg).is_err()
        );
    }

    #[test]
    fn short_sign_is_correct() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Supply, "1.90", "1.94");
        let s = compute_levels(
            Direction::Short,
            &z,
            dec("1.90"),
            Some(dec("1.70")),
            2,
            &cfg,
        )
        .unwrap();
        assert_eq!(s.stop_loss, dec("1.95"));
        assert_eq!(s.take_profit, dec("1.75"));
    }

    #[test]
    fn coarse_price_scale_gate_uses_rounded_levels() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.960", "2.000");
        // trigger 2.001 rounds to 2.00 at scale 2; stop 1.95; risk 0.05; TP = 2.00 + 3*0.05 = 2.15
        let l = compute_levels(
            Direction::Long,
            &z,
            dec("2.001"),
            Some(dec("2.500")),
            2,
            &cfg,
        )
        .unwrap();
        assert_eq!(l.stop_loss, dec("1.95"));
        assert_eq!(l.take_profit, dec("2.15"));
        assert_eq!(l.gross_rr, dec("3.0")); // derived from the ROUNDED levels, against a known value
    }

    #[test]
    fn structural_mode_targets_the_obstacle() {
        let cfg = Config {
            target_mode: TargetMode::Structural,
            ..Config::default_for(&["XRP-USDT"])
        };
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        // obstacle 2.50, buffer = 0.1*atr(0.1) = 0.01 -> TP = 2.49; risk 0.05 -> gross 9.8
        let l =
            compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.50")), 2, &cfg).unwrap();
        assert_eq!(l.take_profit, dec("2.49"));
        assert_eq!(l.gross_rr, dec("9.8"));
    }

    #[test]
    fn no_obstacle_respects_allow_no_obstacle() {
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let allow = Config {
            allow_no_obstacle: true,
            ..Config::default_for(&["XRP-USDT"])
        };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &allow).is_ok());
        let deny = Config {
            allow_no_obstacle: false,
            ..Config::default_for(&["XRP-USDT"])
        };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &deny).is_err());
    }
}

pub fn is_first_touch(
    direction: Direction,
    zone: &Zone,
    prev_price: Decimal,
    price: Decimal,
) -> bool {
    match direction {
        Direction::Long => prev_price > zone.high && price <= zone.high,
        Direction::Short => prev_price < zone.low && price >= zone.low,
    }
}

pub fn is_missed_touch(direction: Direction, zone: &Zone, price: Decimal) -> bool {
    match direction {
        Direction::Long => price <= zone.high,
        Direction::Short => price >= zone.low,
    }
}

fn direction_for(trend: TrendState) -> Option<Direction> {
    match trend {
        TrendState::Bullish => Some(Direction::Long),
        TrendState::Bearish => Some(Direction::Short),
        TrendState::Unknown => None,
    }
}

fn zone_kind_for(dir: Direction) -> ZoneKind {
    match dir {
        Direction::Long => ZoneKind::Demand,
        Direction::Short => ZoneKind::Supply,
    }
}

/// Distance from `prev_price` to the zone's proximal (touch) edge. Only meaningful for a crossed
/// zone, where `prev_price` is already beyond that edge.
fn touch_distance(dir: Direction, zone: &Zone, prev: Decimal) -> Decimal {
    match dir {
        Direction::Long => prev - zone.high,
        Direction::Short => zone.low - prev,
    }
}

/// On the first tick after activation, a freshly-created zone that is already inside/beyond is a
/// `missed_touch` (spec §7.2). The awaiting set is cleared for **all** zones (not just one selected
/// zone) and stale ids are dropped, so a deeper zone is not later judged against a much later price.
fn observe_first_tick(state: &mut StrategyState, dir: Direction, price: Decimal) -> Vec<Output> {
    let mut out = Vec::new();
    let awaiting = std::mem::take(&mut state.zones_awaiting_first_tick);
    for id in awaiting {
        let Some(z) = state
            .zones
            .iter_mut()
            .find(|z| z.id == id && z.state == ZoneState::Active)
        else {
            continue; // stale id: the zone no longer exists / is no longer Active
        };
        if is_missed_touch(dir, z, price) {
            z.state = ZoneState::Consumed;
            z.consume_reason = Some(ConsumeReason::MissedTouch);
            state.missed_touch_count += 1;
            out.push(Output::ZoneEvent(ZoneEvent {
                zone_id: id,
                kind: ZoneEventKind::Consumed,
                reason: Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch)),
            }));
        }
    }
    out
}

/// Evaluate one crossed `Active` zone for a first-touch entry. A signal or a veto **consumes** the
/// zone (spec §6 one entry attempt per zone, §7.1 first touch). Returns the emitted output, or
/// `None` when the tick is not actually a first touch for that zone.
fn attempt_zone(
    state: &mut StrategyState,
    dir: Direction,
    zone_id: &str,
    prev: Decimal,
    price: Decimal,
    tick_time_ms: Ms,
    cfg: &Config,
) -> Result<Option<Output>, StrategyError> {
    let zone_snapshot = match state
        .zones
        .iter()
        .find(|z| z.id == zone_id && z.state == ZoneState::Active)
    {
        Some(z) => z.clone(),
        None => return Ok(None),
    };
    if !is_first_touch(dir, &zone_snapshot, prev, price) {
        return Ok(None);
    }

    // A symbol with no configured price scale is a config error, never a silent default.
    let price_scale = match cfg.price_scale.get(&state.symbol) {
        Some(s) => *s,
        None => {
            return Err(StrategyError::ConfigInvalid(format!(
                "no price_scale configured for symbol `{}`",
                state.symbol
            )))
        }
    };

    // Obstacle: swing levels + active opposite zones + the zone's window extremes (includes the
    // breakout candle). Every ATR-scaled level and the payload's `atr`/`zone_height_atr` use the
    // zone's own pre-breakout ATR (`zone.atr`), never the latest `state.atr` (no look-ahead).
    let (active_supply, active_demand): (Vec<Zone>, Vec<Zone>) = (
        state
            .zones
            .iter()
            .filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Supply)
            .cloned()
            .collect(),
        state
            .zones
            .iter()
            .filter(|z| z.state == ZoneState::Active && z.kind == ZoneKind::Demand)
            .cloned()
            .collect(),
    );
    let obstacle = match dir {
        Direction::Long => obstacle_for_long(
            price,
            &state.swing_highs,
            &active_supply,
            zone_snapshot.window_high,
        ),
        Direction::Short => obstacle_for_short(
            price,
            &state.swing_lows,
            &active_demand,
            zone_snapshot.window_low,
        ),
    };
    let levels = match compute_levels(dir, &zone_snapshot, price, obstacle, price_scale, cfg) {
        Ok(l) => l,
        Err(tag) => {
            // §7.1 first touch + §6 "one entry attempt per zone": a first-touch veto CONSUMES
            // the zone (no `StateChange`), so a later/deeper tick cannot re-trigger it.
            let reason = match tag {
                "obstacle_too_close" => ConsumeReason::VetoedObstacleTooClose,
                "obstacle_none" => ConsumeReason::VetoedObstacleNone,
                "risk_nonpositive" => ConsumeReason::VetoedRiskNonPositive,
                other => unreachable!("`compute_levels` veto tag `{other}` has no ConsumeReason"),
            };
            if let Some(z) = state.zones.iter_mut().find(|z| z.id == zone_id) {
                z.state = ZoneState::Consumed;
                z.consume_reason = Some(reason);
                z.attempt_made = true;
            }
            return Ok(Some(Output::ZoneEvent(ZoneEvent {
                zone_id: zone_id.to_string(),
                kind: ZoneEventKind::Consumed,
                reason: Some(ZoneEventReason::Consumed(reason)),
            })));
        }
    };
    // Payload ratios are rounded for serialization only (toward zero, 4 dp): no 28-digit noise.
    let zone_height_atr =
        round_ratio_for_output((zone_snapshot.high - zone_snapshot.low) / zone_snapshot.atr);
    let trigger = round_price(price, price_scale);
    let signal = Signal {
        schema_version: 1,
        event_type: "strategy.entry_signal".to_string(),
        signal_id: format!("{}:0", zone_snapshot.id),
        strategy: "supply_demand_4h".to_string(),
        strategy_version: "0.1.0".to_string(),
        config_version: crate::strategy::config::config_version(&cfg.canonical_json()),
        symbol: state.symbol.clone(),
        market: "margin".to_string(),
        direction: dir,
        timeframe: "4hour".to_string(),
        trend_state: state.trend,
        zone_id: zone_snapshot.id.clone(),
        zone_low: zone_snapshot.low,
        zone_high: zone_snapshot.high,
        zone_height_atr,
        // The pre-breakout ATR the zone was accepted / stop-buffered with (no look-ahead).
        atr: zone_snapshot.atr,
        trigger_price: trigger,
        reference_entry_price: trigger,
        stop_loss: levels.stop_loss,
        take_profit: levels.take_profit,
        target_mode: match cfg.target_mode {
            TargetMode::FixedR => "fixed_r",
            TargetMode::Structural => "structural",
        }
        .to_string(),
        target_r: cfg.target_r,
        min_gross_rr: cfg.min_gross_rr,
        gross_rr: round_ratio_for_output(levels.gross_rr),
        structure_obstacle_price: levels.obstacle,
        obstacle_rr: levels.obstacle_rr.map(round_ratio_for_output),
        triggered_at_ms: tick_time_ms,
        candle_timestamp_ms: state.last_candle_time_ms.unwrap_or(0),
    };
    if let Some(z) = state.zones.iter_mut().find(|z| z.id == zone_id) {
        z.state = ZoneState::Consumed;
        z.consume_reason = Some(ConsumeReason::SignalPublished);
        z.attempt_made = true;
    }
    Ok(Some(Output::EntrySignal(signal)))
}

/// Entry attempt on a tick. Mutates `state`, returns outputs.
/// `tick_time_ms` is the touching tick's timestamp (becomes `triggered_at_ms`).
///
/// **Multi-zone first-touch:** every `Active` zone of the trend's kind crossed in the tick interval
/// `[prev_price, price]` is evaluated, ordered by distance from `prev_price` (nearest first) — not
/// just `select_zone`'s single nearest. The first signal wins; a veto consumes that zone and
/// evaluation continues to the next crossed zone.
pub fn try_entry(
    state: &mut StrategyState,
    prev_price: Option<Decimal>,
    tick_time_ms: Ms,
    price: Decimal,
    cfg: &Config,
) -> Result<Vec<Output>, StrategyError> {
    let mut out = Vec::new();
    let dir = match direction_for(state.trend) {
        Some(d) => d,
        None => return Ok(out),
    };

    // First observation after activation: missed-touch handling for freshly-created zones.
    out.extend(observe_first_tick(state, dir, price));

    let prev = match prev_price {
        Some(p) => p,
        None => return Ok(out),
    };

    let want = zone_kind_for(dir);
    let mut crossed: Vec<(Decimal, String)> = state
        .zones
        .iter()
        .filter(|z| z.state == ZoneState::Active && z.kind == want)
        .filter(|z| is_first_touch(dir, z, prev, price))
        .map(|z| (round_to(touch_distance(dir, z, prev), 8), z.id.clone()))
        .collect();
    crossed.sort_by_key(|a| a.0); // nearest to prev first; round_to keeps the key stable
    for (_, id) in crossed {
        if let Some(o) = attempt_zone(state, dir, &id, prev, price, tick_time_ms, cfg)? {
            let is_signal = matches!(o, Output::EntrySignal(_));
            out.push(o);
            if is_signal {
                break; // the nearest zone takes the entry: one signal per tick
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod touch_tests {
    use super::*;
    use crate::strategy::types::{Pivot, ZoneState};
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn demand() -> Zone {
        Zone {
            id: "z".into(),
            kind: ZoneKind::Demand,
            low: dec("1.96"),
            high: dec("2.00"),
            atr: dec("0.1"),
            breakout_time_ms: 0,
            zone_candle_time_ms: 0,
            activated_at_index: 0,
            state: ZoneState::Active,
            terminal_reason: None,
            consume_reason: None,
            attempt_made: false,
            window_high: dec("2.00"),
            window_low: dec("1.96"),
        }
    }

    #[test]
    fn first_touch_requires_crossing_from_above() {
        assert!(is_first_touch(
            Direction::Long,
            &demand(),
            dec("2.01"),
            dec("2.00")
        ));
        assert!(!is_first_touch(
            Direction::Long,
            &demand(),
            dec("2.01"),
            dec("2.02")
        ));
    }

    #[test]
    fn emits_signal_on_first_touch_with_tick_time() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.zones = vec![demand()];
        s.swing_highs = vec![Pivot {
            index: 0,
            time_ms: 0,
            price: dec("2.30"),
            broken: false,
        }];
        let out = try_entry(&mut s, Some(dec("2.05")), 12345, dec("2.00"), &cfg).unwrap();
        let sig = out
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(sig.triggered_at_ms, 12345); // the TOUCHING tick's time, not the previous one
        assert!(s.zones[0].state == ZoneState::Consumed);
    }

    #[test]
    fn missed_touch_on_first_observation() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.zones = vec![demand()];
        s.zones_awaiting_first_tick = vec!["z".into()];
        let out = try_entry(&mut s, Some(dec("2.10")), 1, dec("1.98"), &cfg).unwrap();
        assert!(out.iter().any(|o| matches!(o, Output::ZoneEvent(_))));
        assert_eq!(s.missed_touch_count, 1);
    }

    fn zone_id(id: &str, low: &str, high: &str, window_high: &str) -> Zone {
        Zone {
            id: id.into(),
            kind: ZoneKind::Demand,
            low: dec(low),
            high: dec(high),
            atr: dec("0.1"),
            breakout_time_ms: 0,
            zone_candle_time_ms: 0,
            activated_at_index: 0,
            state: ZoneState::Active,
            terminal_reason: None,
            consume_reason: None,
            attempt_made: false,
            window_high: dec(window_high),
            window_low: dec(low),
        }
    }

    #[test]
    fn payload_uses_the_zone_pre_breakout_atr_not_state_atr() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("2.0")); // latest ATR (after the breakout): must NOT reach the payload
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        // zone.atr = 0.1; height 2.00 - 1.96 = 0.04 -> 0.04 / 0.1 = 0.4 ATR.
        s.zones = vec![zone_id("z", "1.96", "2.00", "1.0")];
        let out = try_entry(&mut s, Some(dec("2.05")), 1, dec("2.00"), &cfg).unwrap();
        let sig = out
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            sig.atr,
            dec("0.1"),
            "payload ATR is the zone's pre-breakout ATR"
        );
        assert_eq!(sig.zone_height_atr, dec("0.4"), "0.04 / 0.1 = 0.4");
    }

    #[test]
    fn missing_price_scale_is_a_config_error_not_a_default() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("OTHER-USDT", 1); // not in cfg.price_scale
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        s.zones = vec![zone_id("z", "1.96", "2.00", "1.0")];
        let err = try_entry(&mut s, Some(dec("2.05")), 1, dec("2.00"), &cfg).unwrap_err();
        assert!(matches!(err, StrategyError::ConfigInvalid(_)));
    }

    #[test]
    fn first_tick_clears_awaiting_for_all_created_zones() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        s.zones = vec![
            zone_id("a", "1.98", "2.00", "1.0"),
            zone_id("b", "1.90", "1.92", "1.0"),
        ];
        s.zones_awaiting_first_tick = vec!["a".into(), "b".into()];
        // The first tick sits above both zones: neither is missed, and BOTH leave the awaiting set.
        let out = try_entry(&mut s, Some(dec("2.10")), 1, dec("2.05"), &cfg).unwrap();
        assert!(out.is_empty());
        assert!(
            s.zones_awaiting_first_tick.is_empty(),
            "all created zones must be observed on the first tick"
        );
        assert!(s.zones.iter().all(|z| z.state == ZoneState::Active));
    }

    #[test]
    fn multi_zone_first_touch_takes_the_nearest_to_prev() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        // shallow [1.98,2.00] (dist 0.10 from 2.10) and deep [1.95,1.995] (dist 0.105); the tick
        // 1.995 crosses both. Only the nearest (shallow) may be consumed.
        s.zones = vec![
            zone_id("shallow", "1.98", "2.00", "1.0"),
            zone_id("deep", "1.95", "1.995", "1.0"),
        ];
        let out = try_entry(&mut s, Some(dec("2.10")), 1, dec("1.995"), &cfg).unwrap();
        let sigs: Vec<_> = out
            .iter()
            .filter_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(sigs.len(), 1, "exactly one signal for the crossed pair");
        assert_eq!(sigs[0].zone_id, "shallow", "nearest to prev wins");
        assert!(
            s.zones.iter().find(|z| z.id == "deep").unwrap().state == ZoneState::Active,
            "the deeper zone stays Active"
        );
    }

    #[test]
    fn veto_on_the_nearest_zone_continues_to_the_next_crossed_zone() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        // shallow [2.00,2.01] (dist 0.09) vetoes: trigger 1.985 <= stop 2.00 - 0.01 -> risk <= 0.
        // deep [1.95,1.99] (dist 0.11) is crossed too and must still be evaluated after the veto.
        s.zones = vec![
            zone_id("shallow", "2.00", "2.01", "1.0"),
            zone_id("deep", "1.95", "1.99", "1.0"),
        ];
        let out = try_entry(&mut s, Some(dec("2.10")), 1, dec("1.985"), &cfg).unwrap();
        assert!(out.iter().any(|o| matches!(
            o,
            Output::ZoneEvent(ev) if ev.zone_id == "shallow"
                && ev.reason == Some(ZoneEventReason::Consumed(ConsumeReason::VetoedRiskNonPositive))
        )));
        let sig = out
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .expect("the deeper crossed zone must still signal after the veto");
        assert_eq!(sig.zone_id, "deep");
    }

    #[test]
    fn first_tick_non_nearest_zone_touch_is_not_missed() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("0.1"));
        s.trend = TrendState::Bullish;
        s.last_candle_time_ms = Some(0);
        // Two demand zones are awaiting their first tick at the same time. The deeper zone is, by
        // proximal-edge distance, the nearest one to the tick price (1.93), but the tick never
        // reaches it (1.93 > 1.90). The tick instead first-touches the *other* zone [1.90, 2.00]
        // from above (prev 2.05 > zone_high 2.00, price 1.93 <= zone_high): a normal first touch.
        s.zones = vec![
            zone_id("deeper", "1.70", "1.90", "1.90"),
            zone_id("touched", "1.90", "2.00", "2.20"),
        ];
        s.zones_awaiting_first_tick = vec!["deeper".into(), "touched".into()];

        let out = try_entry(&mut s, Some(dec("2.05")), 1, dec("1.93"), &cfg).unwrap();

        let sig = out
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .expect("first tick on a legitimate first touch must publish an entry signal");
        assert_eq!(sig.zone_id, "touched");
        assert_eq!(sig.direction, Direction::Long);

        // It must not be recorded as a missed touch: the price crossed the proximal edge from the
        // correct side on this very tick.
        assert!(
            !out.iter().any(|o| matches!(
                o,
                Output::ZoneEvent(ev)
                    if ev.zone_id == "touched"
                        && ev.reason
                            == Some(ZoneEventReason::Consumed(ConsumeReason::MissedTouch))
            )),
            "the touched zone must not be consumed as missed_touch"
        );
        assert_eq!(s.missed_touch_count, 0);
        assert_eq!(
            s.zones
                .iter()
                .find(|z| z.id == "touched")
                .unwrap()
                .consume_reason,
            Some(ConsumeReason::SignalPublished)
        );

        // The other awaiting zone was never reached, so it must stay Active (not be consumed as a
        // missed touch just because it was still awaiting its first tick).
        let deeper = s.zones.iter().find(|z| z.id == "deeper").unwrap();
        assert_eq!(deeper.state, ZoneState::Active);
        assert_eq!(deeper.consume_reason, None);
    }
}

/// Edge-case accuracy tests (Task 28): the supply (short) path end to end through `step`, and a
/// large-tick symbol proving the R:R gate evaluates the *rounded* payload levels.
#[cfg(test)]
mod edge_tests {
    use super::*;
    use crate::strategy::step::step;
    use crate::strategy::types::{
        Direction, Input, Output, Pivot, StrategyState, Tick, TrendState, ZoneKind, ZoneState,
    };
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn zone(kind: ZoneKind, low: &str, high: &str, window_low: &str) -> Zone {
        Zone {
            id: "z".into(),
            kind,
            low: dec(low),
            high: dec(high),
            atr: dec("1.0"),
            breakout_time_ms: 0,
            zone_candle_time_ms: 0,
            activated_at_index: 0,
            state: ZoneState::Active,
            terminal_reason: None,
            consume_reason: None,
            attempt_made: false,
            window_high: dec(high),
            window_low: dec(window_low),
        }
    }

    #[test]
    fn supply_path_short_signal_end_to_end() {
        // Bearish trend, an active supply zone [104.0, 105.0]; a descending tick rises back up
        // and first-touches the zone's proximal edge (zone_low) from below.
        // atr 1.0: stop = 105.0 + 0.1*1.0 = 105.1; trigger 104.1 -> risk 1.0; TP = 104.1 - 3.0*1.0.
        // The only obstacle below the trigger is the zone window low 101.1 -> rr = 3.0 (== floor).
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut s = StrategyState::new("XRP-USDT", 1);
        s.atr = Some(dec("1.0"));
        s.trend = TrendState::Bearish;
        s.last_price = Some(dec("103.5"));
        s.last_tick_time_ms = Some(0);
        s.last_candle_time_ms = Some(0);
        s.zones = vec![zone(ZoneKind::Supply, "104.0", "105.0", "101.1")];

        let (_, outs) = step(
            &s,
            Input::PriceTick(Tick {
                time_ms: 1,
                price: dec("104.1"),
                is_bar_open: false,
            }),
            &cfg,
        )
        .unwrap();
        let sig = outs
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .expect("a first touch from below must publish a short signal");

        assert_eq!(sig.direction, Direction::Short);
        assert_eq!(sig.trigger_price, dec("104.1"));
        assert!(
            sig.stop_loss > sig.zone_high,
            "short stop sits ABOVE the supply zone ({} vs {})",
            sig.stop_loss,
            sig.zone_high
        );
        assert_eq!(sig.stop_loss, dec("105.1"));
        assert!(sig.take_profit < sig.trigger_price);
        assert_eq!(sig.take_profit, dec("101.1"));
        assert_eq!(sig.structure_obstacle_price, Some(dec("101.1")));
        assert_eq!(sig.obstacle_rr, Some(dec("3.0")));
    }

    #[test]
    fn large_tick_symbol_gate_uses_rounded_payload_levels() {
        // A large-tick symbol (price_scale 0 => tick size 1.0). Raw (unrounded) numbers would
        // veto: trigger 100.4 / stop 98.9 / obstacle 103.5 -> rr = 3.1/1.5 = 2.066 < 3.0. The
        // gate must evaluate the ROUNDED payload levels instead: trigger 100, stop 99,
        // obstacle 104, risk 1.0 -> rr 4.0 >= 3.0, so the signal is accepted.
        let mut cfg = Config::default_for(&["BIG-USDT"]);
        cfg.price_scale.insert("BIG-USDT".to_string(), 0);
        let mut s = StrategyState::new("BIG-USDT", 1);
        s.atr = Some(dec("1.0"));
        s.trend = TrendState::Bullish;
        s.last_price = Some(dec("100.7"));
        s.last_tick_time_ms = Some(0);
        s.last_candle_time_ms = Some(0);
        s.swing_highs = vec![Pivot {
            index: 0,
            time_ms: 0,
            price: dec("103.5"),
            broken: false,
        }];
        // Zone high 100.4 so the touching tick (100.4) first-touches the proximal edge.
        s.zones = vec![zone(ZoneKind::Demand, "99.0", "100.4", "99.0")];

        let (_, outs) = step(
            &s,
            Input::PriceTick(Tick {
                time_ms: 1,
                price: dec("100.4"),
                is_bar_open: false,
            }),
            &cfg,
        )
        .unwrap();
        let sig = outs
            .iter()
            .find_map(|o| {
                if let Output::EntrySignal(sig) = o {
                    Some(sig)
                } else {
                    None
                }
            })
            .expect("the gate must accept when the rounded obstacle_rr clears the floor");

        assert_eq!(sig.trigger_price, dec("100"));
        assert_eq!(sig.stop_loss, dec("99"));
        assert_eq!(sig.take_profit, dec("103"));
        assert_eq!(sig.structure_obstacle_price, Some(dec("104")));
        assert_eq!(sig.obstacle_rr, Some(dec("4.0")));
        assert_eq!(sig.gross_rr, dec("3.0"));
    }
}
