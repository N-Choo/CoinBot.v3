use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub type Ms = i64;

/// Bounded rolling window of closed candles kept in state.
pub const WINDOW_CANDLES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candle {
    pub open_time_ms: Ms,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick {
    pub time_ms: Ms,
    pub price: Decimal,
    pub is_bar_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendState {
    Unknown,
    Bullish,
    Bearish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneKind {
    Demand,
    Supply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneState {
    Active,
    Consumed,
    Invalidated,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumeReason {
    SignalPublished,
    MissedTouch,
    EngineAccepted,
    EngineRejectedStrategy,
    EngineRejectedTransient,
    ExpiredUndelivered,
    // First-touch vetoes: the zone is consumed by the veto (spec §6 one attempt per zone,
    // §7.1 first touch), so a later/deeper tick cannot re-trigger it.
    VetoedObstacleTooClose,
    VetoedObstacleNone,
    VetoedRiskNonPositive,
}

/// Why a zone became terminal without a signal. Separate from `ConsumeReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalReason {
    TrendFlip,
    CloseBeyondEdge,
    Expired,
}

/// A zone event carries either a terminal reason or a consumption reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneEventReason {
    Terminal(TerminalReason),
    Consumed(ConsumeReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub kind: ZoneKind,
    pub low: Decimal,
    pub high: Decimal,
    pub atr: Decimal, // ATR at activation
    pub breakout_time_ms: Ms,
    pub zone_candle_time_ms: Ms,
    pub activated_at_index: u64, // absolute candle index at activation
    pub state: ZoneState,
    pub terminal_reason: Option<TerminalReason>,
    pub consume_reason: Option<ConsumeReason>,
    pub attempt_made: bool,
    pub window_high: Decimal, // max high from breakout candle through latest closed candle
    pub window_low: Decimal,  // min low over the same window (includes the breakout candle)
}

impl Zone {
    /// Phase 0: every non-Active state is terminal.
    pub fn is_terminal(&self) -> bool {
        !matches!(self.state, ZoneState::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    pub schema_version: u32,
    pub event_type: String,
    pub signal_id: String,
    pub strategy: String,
    pub strategy_version: String,
    pub config_version: String,
    pub symbol: String,
    pub market: String,
    pub direction: Direction,
    pub timeframe: String,
    pub trend_state: TrendState,
    pub zone_id: String,
    pub zone_low: Decimal,
    pub zone_high: Decimal,
    pub zone_height_atr: Decimal,
    pub atr: Decimal,
    pub trigger_price: Decimal,
    pub reference_entry_price: Decimal,
    pub stop_loss: Decimal,
    pub take_profit: Decimal,
    pub target_mode: String,
    pub target_r: Decimal,
    pub min_gross_rr: Decimal,
    pub gross_rr: Decimal,
    pub structure_obstacle_price: Option<Decimal>,
    pub obstacle_rr: Option<Decimal>,
    pub triggered_at_ms: Ms,
    pub candle_timestamp_ms: Ms,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pivot {
    pub index: u64,
    pub time_ms: Ms,
    pub price: Decimal,
    pub broken: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyState {
    pub schema_version: u32,
    pub symbol: String,
    /// Bounded rolling window of the most recent closed candles (capacity WINDOW_CANDLES).
    pub window: VecDeque<Candle>,
    /// Absolute count of closed candles seen (window positions are derived from this).
    pub candle_count: u64,
    pub prev_close: Option<Decimal>,
    /// Running Wilder ATR through the last processed candle.
    pub atr: Option<Decimal>,
    pub atr_seed_sum: Decimal,
    pub atr_seen: u32,
    pub trend: TrendState,
    /// Absolute pivot indices for the valid low/high (spec §6.3).
    pub valid_low_idx: Option<u64>,
    pub valid_high_idx: Option<u64>,
    /// Pivot.index is ABSOLUTE (== candle_count - 1 - right at confirmation time).
    pub swing_highs: Vec<Pivot>,
    pub swing_lows: Vec<Pivot>,
    pub zones: Vec<Zone>,
    pub zones_awaiting_first_tick: Vec<String>,
    pub last_candle_time_ms: Option<Ms>,
    pub last_tick_time_ms: Option<Ms>,
    pub last_price: Option<Decimal>,
    pub missed_touch_count: u64,
}

impl StrategyState {
    pub fn new(symbol: &str, schema_version: u32) -> Self {
        StrategyState {
            schema_version,
            symbol: symbol.to_string(),
            window: VecDeque::new(),
            candle_count: 0,
            prev_close: None,
            atr: None,
            atr_seed_sum: Decimal::ZERO,
            atr_seen: 0,
            trend: TrendState::Unknown,
            valid_low_idx: None,
            valid_high_idx: None,
            swing_highs: Vec::new(),
            swing_lows: Vec::new(),
            zones: Vec::new(),
            zones_awaiting_first_tick: Vec::new(),
            last_candle_time_ms: None,
            last_tick_time_ms: None,
            last_price: None,
            missed_touch_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    ClosedCandle(Candle),
    PriceTick(Tick),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneEventKind {
    Created,
    Invalidated,
    Expired,
    Consumed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneEvent {
    pub zone_id: String,
    pub kind: ZoneEventKind,
    pub reason: Option<ZoneEventReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// Signal is intentionally kept unboxed: the Phase 0 output API pattern-matches
// `Output::EntrySignal(sig)` directly and the enum is not hot-path allocated.
#[allow(clippy::large_enum_variant)]
pub enum Output {
    ZoneEvent(ZoneEvent),
    EntrySignal(Signal),
    StateChange { reason: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> Zone {
        Zone {
            id: "z".into(),
            kind: ZoneKind::Demand,
            low: Decimal::ZERO,
            high: Decimal::ONE,
            atr: Decimal::ONE,
            breakout_time_ms: 0,
            zone_candle_time_ms: 0,
            activated_at_index: 0,
            state: ZoneState::Active,
            terminal_reason: None,
            consume_reason: None,
            attempt_made: false,
            window_high: Decimal::ONE,
            window_low: Decimal::ZERO,
        }
    }

    #[test]
    fn zone_terminal_rule() {
        let mut z = zone();
        assert!(!z.is_terminal());
        z.state = ZoneState::Consumed;
        assert!(z.is_terminal());
        z.state = ZoneState::Invalidated;
        assert!(z.is_terminal());
        z.state = ZoneState::Expired;
        assert!(z.is_terminal());
    }

    #[test]
    fn state_round_trips_json() {
        let s = StrategyState::new("XRP-USDT", 1);
        let j = serde_json::to_string(&s).unwrap();
        let back: StrategyState = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }
}
