use crate::strategy::config::{Config, TargetMode};
use crate::strategy::decimal::round_price;
use crate::strategy::types::{Direction, Pivot, Zone, ZoneKind};
use rust_decimal::Decimal;

pub struct Levels {
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub gross_rr: Decimal,
    pub obstacle: Option<Decimal>,
    pub obstacle_rr: Option<Decimal>,
}

pub fn obstacle_for_long(trigger: Decimal, swing_highs: &[Pivot], active_supply: &[Zone], window_high: Decimal) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(swing_highs.iter().filter(|p| !p.broken && p.price > trigger).map(|p| p.price));
    c.extend(active_supply.iter().filter(|z| z.low > trigger).map(|z| z.low));
    if window_high > trigger { c.push(window_high); } // window includes the breakout candle
    c.into_iter().min()
}

pub fn obstacle_for_short(trigger: Decimal, swing_lows: &[Pivot], active_demand: &[Zone], window_low: Decimal) -> Option<Decimal> {
    let mut c: Vec<Decimal> = Vec::new();
    c.extend(swing_lows.iter().filter(|p| !p.broken && p.price < trigger).map(|p| p.price));
    c.extend(active_demand.iter().filter(|z| z.high < trigger).map(|z| z.high));
    if window_low < trigger { c.push(window_low); }
    c.into_iter().max()
}

/// Rounded levels + the mode's gate. `None` = veto.
pub fn compute_levels(
    direction: Direction,
    zone: &Zone,
    trigger: Decimal,
    obstacle: Option<Decimal>,
    price_scale: u32,
    cfg: &Config,
) -> Option<Levels> {
    let t = round_price(trigger, price_scale);
    let buffer = cfg.stop_buffer_atr * zone.atr;
    let (stop_raw, risk) = match direction {
        Direction::Long => (zone.low - buffer, t - (zone.low - buffer)),
        Direction::Short => (zone.high + buffer, (zone.high + buffer) - t),
    };
    if risk <= Decimal::ZERO { return None; }
    let stop = round_price(stop_raw, price_scale);
    let risk = match direction { Direction::Long => t - stop, Direction::Short => stop - t };
    if risk <= Decimal::ZERO { return None; }

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
                if rr < cfg.target_r { return None; }
            } else if !cfg.allow_no_obstacle {
                return None;
            }
            let tp = match direction { Direction::Long => t + cfg.target_r * risk, Direction::Short => t - cfg.target_r * risk };
            (round_price(tp, price_scale), cfg.target_r)
        }
        TargetMode::Structural => {
            let o = match obstacle_rounded {
                Some(o) => o,
                None => {
                    if !cfg.allow_no_obstacle { return None; }
                    let tp = match direction { Direction::Long => t + cfg.target_r * risk, Direction::Short => t - cfg.target_r * risk };
                    return Some(Levels { stop_loss: stop, take_profit: round_price(tp, price_scale), gross_rr: cfg.target_r, obstacle: None, obstacle_rr: None });
                }
            };
            let buf = round_price(cfg.structural_buffer_atr * zone.atr, price_scale);
            let tp = match direction { Direction::Long => o - buf, Direction::Short => o + buf };
            let tp = round_price(tp, price_scale);
            let gross = match direction { Direction::Long => (tp - t) / risk, Direction::Short => (t - tp) / risk };
            if gross < cfg.min_gross_rr { return None; }
            (tp, gross)
        }
    };
    // Recompute gross from the rounded levels that go into the payload.
    let gross = match direction { Direction::Long => (tp - t) / risk, Direction::Short => (t - tp) / risk };
    Some(Levels { stop_loss: stop, take_profit: tp, gross_rr: gross, obstacle: obstacle_rounded, obstacle_rr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::ZoneState;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn zone(kind: ZoneKind, low: &str, high: &str) -> Zone {
        Zone { id: "z".into(), kind, low: dec(low), high: dec(high), atr: dec("0.1"),
            breakout_time_ms: 0, zone_candle_time_ms: 0, activated_at_index: 0,
            state: ZoneState::Active, terminal_reason: None, consume_reason: None, attempt_made: false,
            window_high: dec(high), window_low: dec(low) }
    }

    #[test]
    fn long_levels_and_obstacle_veto() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let ok = compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.20")), 2, &cfg).unwrap();
        assert_eq!(ok.stop_loss, dec("1.95"));
        assert_eq!(ok.take_profit, dec("2.15"));
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.05")), 2, &cfg).is_none());
    }

    #[test]
    fn short_sign_is_correct() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Supply, "1.90", "1.94");
        let s = compute_levels(Direction::Short, &z, dec("1.90"), Some(dec("1.70")), 2, &cfg).unwrap();
        assert_eq!(s.stop_loss, dec("1.95"));
        assert_eq!(s.take_profit, dec("1.75"));
    }

    #[test]
    fn coarse_price_scale_gate_uses_rounded_levels() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = zone(ZoneKind::Demand, "1.960", "2.000");
        // trigger 2.001 rounds to 2.00 at scale 2; stop 1.95; risk 0.05; TP = 2.00 + 3*0.05 = 2.15
        let l = compute_levels(Direction::Long, &z, dec("2.001"), Some(dec("2.500")), 2, &cfg).unwrap();
        assert_eq!(l.stop_loss, dec("1.95"));
        assert_eq!(l.take_profit, dec("2.15"));
        assert_eq!(l.gross_rr, dec("3.0")); // derived from the ROUNDED levels, against a known value
    }

    #[test]
    fn structural_mode_targets_the_obstacle() {
        let cfg = Config { target_mode: TargetMode::Structural, ..Config::default_for(&["XRP-USDT"]) };
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        // obstacle 2.50, buffer = 0.1*atr(0.1) = 0.01 -> TP = 2.49; risk 0.05 -> gross 9.8
        let l = compute_levels(Direction::Long, &z, dec("2.00"), Some(dec("2.50")), 2, &cfg).unwrap();
        assert_eq!(l.take_profit, dec("2.49"));
        assert_eq!(l.gross_rr, dec("9.8"));
    }

    #[test]
    fn no_obstacle_respects_allow_no_obstacle() {
        let z = zone(ZoneKind::Demand, "1.96", "2.00");
        let allow = Config { allow_no_obstacle: true, ..Config::default_for(&["XRP-USDT"]) };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &allow).is_some());
        let deny = Config { allow_no_obstacle: false, ..Config::default_for(&["XRP-USDT"]) };
        assert!(compute_levels(Direction::Long, &z, dec("2.00"), None, 2, &deny).is_none());
    }
}
