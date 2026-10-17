use crate::strategy::config::Config;
use crate::strategy::structure::BosKind;
use crate::strategy::types::{Candle, Zone, ZoneKind, ZoneState};
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Build a zone after a BOS, from the bounded candle window. `Err(reason)` is a rejection tag.
/// `activated_at_index` is set by the caller (it owns the absolute candle counter).
pub fn create_zone(window: &VecDeque<Candle>, kind: BosKind, atr: Decimal, symbol: &str, cfg: &Config) -> Result<Zone, &'static str> {
    if atr <= Decimal::ZERO { return Err("atr_zero"); }
    let n = window.len();
    if n == 0 { return Err("no_candles"); }
    let at = |i: usize| window.get(i).unwrap();
    let breakout = n - 1;
    let lookback_start = breakout.saturating_sub(cfg.zone_search_lookback as usize);

    let mut chosen = None;
    for i in (lookback_start..breakout).rev() {
        let c = at(i);
        let opposite = match kind { BosKind::Bullish => c.close < c.open, BosKind::Bearish => c.close > c.open };
        if opposite { chosen = Some(i); break; }
    }
    let zi = chosen.ok_or("no_opposite_candle")?;

    let body = (at(breakout).close - at(breakout).open).abs();
    if body < cfg.impulse_body_threshold_atr * atr { return Err("weak_impulse"); }

    let zc = at(zi);
    let (low, high) = (zc.low, zc.high);
    let height_atr = (high - low) / atr;
    if height_atr < cfg.min_zone_height_atr { return Err("zone_too_small"); }
    if height_atr > cfg.max_zone_height_atr { return Err("zone_too_large"); }

    let start = (zi + 1 + cfg.mitigation_grace_candles as usize).min(breakout);
    for i in start..breakout {
        let c = at(i);
        let mitigated = match kind { BosKind::Bullish => c.low <= high, BosKind::Bearish => c.high >= low };
        if mitigated { return Err("mitigated"); }
    }

    let zone_kind = match kind { BosKind::Bullish => ZoneKind::Demand, BosKind::Bearish => ZoneKind::Supply };
    let kind_tag = match zone_kind { ZoneKind::Demand => "demand", ZoneKind::Supply => "supply" };
    let bc = at(breakout);
    let id = format!("{}:{}:{}:{}", symbol, bc.open_time_ms, zc.open_time_ms, kind_tag);
    Ok(Zone {
        id, kind: zone_kind, low, high, atr,
        breakout_time_ms: bc.open_time_ms,
        zone_candle_time_ms: zc.open_time_ms,
        activated_at_index: 0, // set by caller (absolute index)
        state: ZoneState::Active, terminal_reason: None, consume_reason: None, attempt_made: false,
        window_high: bc.high, window_low: bc.low, // window starts at the breakout candle
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn dec(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }
    fn candle(o: &str, h: &str, l: &str, c: &str) -> Candle {
        Candle { open_time_ms: 0, open: dec(o), high: dec(h), low: dec(l), close: dec(c) }
    }
    fn win(v: Vec<Candle>) -> VecDeque<Candle> { v.into() }

    // index 1 is the bearish demand candle (low 0.98 / high 1.06); index 4 is the breakout.
    fn base() -> Vec<Candle> {
        vec![
            candle("1.00","1.05","0.95","1.00"),
            candle("1.02","1.06","0.98","0.99"),
            candle("1.00","1.05","0.98","1.04"),
            candle("1.04","1.10","1.07","1.09"), // low 1.07 > zone high 1.06 -> not mitigated
            candle("1.09","1.25","1.08","1.24"), // breakout, body 0.15
        ]
    }

    #[test]
    fn creates_demand_zone_with_window_starting_at_breakout() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let z = create_zone(&win(base()), BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap();
        assert_eq!(z.kind, ZoneKind::Demand);
        assert_eq!(z.low, dec("0.98"));
        assert_eq!(z.high, dec("1.06"));
        assert_eq!(z.window_high, dec("1.25")); // breakout candle high (included)
        assert_eq!(z.window_low, dec("1.08"));  // breakout candle low (included)
    }

    #[test]
    fn rejects_when_mitigated() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        let mut c = base();
        c[3] = candle("1.04","1.10","1.05","1.09"); // low 1.05 <= zone high 1.06 -> mitigated
        assert_eq!(create_zone(&win(c), BosKind::Bullish, dec("0.1"), "XRP-USDT", &cfg).unwrap_err(), "mitigated");
    }

    #[test]
    fn rejects_atr_zero() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        assert_eq!(create_zone(&win(base()), BosKind::Bullish, dec("0"), "XRP-USDT", &cfg).unwrap_err(), "atr_zero");
    }

    // Breakout body is large (0.34) so the impulse gate never fires before the height bound.
    fn base_big_breakout() -> Vec<Candle> {
        vec![
            candle("1.00","1.05","0.95","1.00"),
            candle("1.02","1.06","0.98","0.99"), // bearish demand candle: h 1.06, l 0.98
            candle("1.02","1.07","1.00","1.05"),
            candle("1.05","1.12","1.08","1.11"), // low 1.08 > 1.06 -> not mitigated
            candle("1.11","1.46","1.10","1.45"), // breakout: body 0.34
        ]
    }

    #[test]
    fn rejects_zone_too_small_and_too_large() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // impulse 1.5*0.2 = 0.30 <= body 0.34 (passes); height 0.08/0.2 = 0.4 < 0.5 -> too small
        assert_eq!(create_zone(&win(base_big_breakout()), BosKind::Bullish, dec("0.2"), "XRP-USDT", &cfg).unwrap_err(), "zone_too_small");
        // impulse 1.5*0.02 = 0.03 <= body 0.34 (passes); height 0.08/0.02 = 4.0 > 3.0 -> too large
        assert_eq!(create_zone(&win(base_big_breakout()), BosKind::Bullish, dec("0.02"), "XRP-USDT", &cfg).unwrap_err(), "zone_too_large");
    }

    #[test]
    fn creates_supply_zone_on_bearish_bos() {
        let cfg = Config::default_for(&["XRP-USDT"]);
        // The most recent bullish candle before the breakout is index 3 (height 0.10 -> 1.0 ATR, in bounds).
        let v = vec![
            candle("1.00","1.05","0.95","1.00"), // 0
            candle("1.00","1.03","0.98","1.02"), // 1
            candle("1.02","1.05","1.00","1.04"), // 2
            candle("1.06","1.14","1.04","1.12"), // 3 bullish supply candle: h 1.14, l 1.04
            candle("1.12","1.13","0.93","0.94"), // 4 bearish breakout: body 0.18
        ];
        let z = create_zone(&win(v), BosKind::Bearish, dec("0.1"), "XRP-USDT", &cfg).unwrap();
        assert_eq!(z.kind, ZoneKind::Supply);
        assert_eq!(z.low, dec("1.04"));
        assert_eq!(z.high, dec("1.14"));
    }
}
