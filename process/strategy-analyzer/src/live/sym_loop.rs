//! The per-symbol live loop (live spec §4, §4a, §5a).
//!
//! One loop owns one [`StrategyState`]. It:
//! - warms from candle history (cold start) or loads a validated state file (warm start);
//! - injects the pure [`Input::Resume`] on load and after a price-feed gap, so a cross that happened
//!   while the loop was down can never read as a first touch (§4);
//! - drops prices older than `price_staleness_ms` (§5);
//! - persists after every candle close and every zone consume (§4);
//! - **appends every `ClosedCandle`/`PriceTick` input** to a recorded stream (§3, for live-vs-replay
//!   equivalence);
//! - logs signals/zone events and quarantines a symbol whose candles keep erroring (§8).
//!
//! It is synchronous and network-free by design, so tests drive it with a fake tick/candle stream.

use crate::live::poller::CandleCursor;
use crate::live::state_store::{LoadOutcome, StateStore};
use crate::live::supervisor::LiveError;
use crate::strategy::config::Config;
use crate::strategy::step::step;
use crate::strategy::types::{
    Candle, Input, Output, Signal, StrategyState, Tick, ZoneEvent, ZoneEventKind,
};
use crate::strategy::STATE_SCHEMA_VERSION;
use rust_decimal::Decimal;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Consecutive candle errors before a symbol is quarantined (§8).
pub const CANDLE_ERROR_THRESHOLD: u32 = 3;
/// A state file more than this many candles behind is too old to trust (§4 "a few candles behind").
/// The age threshold in ms is `STATE_MAX_AGE_CANDLES * candle_interval_ms`, computed per loop from
/// the configured interval so it scales with the timeframe.
pub const STATE_MAX_AGE_CANDLES: i64 = 6;

/// A live input for a symbol. Only these two are recorded (`Resume` is re-injected on replay).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveInput {
    Price { price: Decimal, at_ms: i64 },
    Candle(Candle),
}

/// What the loop did in response to an input (also logged).
// Same rationale as the core `Output` enum: `Signal` is the large variant and the enum is not
// hot-path allocated, so it is kept unboxed for direct pattern-matching at the call sites.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveEvent {
    Signal(Signal),
    ZoneEvent(ZoneEvent),
    StateChange {
        reason: String,
    },
    /// The core `Resume` input was injected (on load or after a gap).
    Resume,
    /// A price older than `price_staleness_ms` was not fed.
    StalePrice {
        at_ms: i64,
        age_ms: i64,
    },
    /// An input the core rejected (malformed/out-of-order/duplicate).
    DroppedInput {
        reason: String,
    },
    /// The symbol has been quarantined and the loop should stop.
    Quarantine {
        reason: String,
    },
}

/// The recorded per-symbol input stream: one JSON `Input` per line, append-only for replay.
struct RecordedStream {
    path: PathBuf,
    file: File,
}

impl RecordedStream {
    fn open(dir: &Path, symbol: &str) -> Result<Self, LiveError> {
        std::fs::create_dir_all(dir).map_err(|source| LiveError::Stream {
            path: dir.display().to_string(),
            source,
        })?;
        let path = dir.join(format!("{symbol}.jsonl"));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| LiveError::Stream {
                path: path.display().to_string(),
                source,
            })?;
        Ok(RecordedStream { path, file })
    }

    /// Truncate the stream (used on a cold start, so the stream matches the re-warmed state lineage).
    fn truncate(&mut self) -> Result<(), LiveError> {
        self.file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|source| LiveError::Stream {
                path: self.path.display().to_string(),
                source,
            })?;
        Ok(())
    }

    fn append(&mut self, input: &Input) -> Result<(), LiveError> {
        let line = serde_json::to_string(input)?;
        writeln!(self.file, "{line}").map_err(|source| LiveError::Stream {
            path: self.path.display().to_string(),
            source,
        })?;
        self.file.flush().map_err(|source| LiveError::Stream {
            path: self.path.display().to_string(),
            source,
        })?;
        Ok(())
    }
}

/// One per-symbol loop.
pub struct SymbolLoop {
    symbol: String,
    cfg: Config,
    store: Arc<StateStore>,
    stream: RecordedStream,
    state: StrategyState,
    was_cold: bool,
    /// A `Resume` must be injected before the next fed input.
    needs_resume: bool,
    last_price_at_ms: Option<i64>,
    consecutive_candle_errors: u32,
    quarantined: Option<String>,
    staleness_ms: i64,
}

impl SymbolLoop {
    /// Load a validated state file, or start cold (the caller then warmes from history).
    pub fn new(
        symbol: impl Into<String>,
        cfg: Config,
        store: Arc<StateStore>,
        stream_dir: &Path,
        staleness_ms: i64,
        now_ms: i64,
    ) -> Result<Self, LiveError> {
        let symbol = symbol.into();
        // The staleness threshold scales with the configured interval (6 candles), so a 5-minute run
        // and a 4-hour run both discard state more than six candles behind.
        let max_age_ms = STATE_MAX_AGE_CANDLES * cfg.candle_interval_ms;
        let outcome = store.load(&symbol, now_ms, Some(max_age_ms));
        let (state, was_cold, cold_reason) = match outcome {
            LoadOutcome::Warm(s) => (s, false, None),
            LoadOutcome::Cold(r) => (
                StrategyState::new(&symbol, STATE_SCHEMA_VERSION),
                true,
                Some(r),
            ),
        };
        let mut stream = RecordedStream::open(stream_dir, &symbol)?;
        if was_cold {
            // A cold start re-warms from history; reset the stream so its lineage matches the fresh
            // state (a later replay from empty state reproduces it exactly).
            stream.truncate()?;
            log::info!("symbol={symbol} cold start ({cold_reason:?}); stream reset");
        } else {
            log::info!("symbol={symbol} warm start from state file");
        }
        Ok(SymbolLoop {
            symbol,
            cfg,
            store,
            stream,
            state,
            was_cold,
            needs_resume: true,
            last_price_at_ms: None,
            consecutive_candle_errors: 0,
            quarantined: None,
            staleness_ms,
        })
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// True when this loop started cold and the caller should warm it from history.
    pub fn is_cold(&self) -> bool {
        self.was_cold
    }

    pub fn is_quarantined(&self) -> bool {
        self.quarantined.is_some()
    }

    pub fn quarantine_reason(&self) -> Option<&str> {
        self.quarantined.as_deref()
    }

    pub fn state(&self) -> &StrategyState {
        &self.state
    }

    pub fn stream_path(&self) -> &Path {
        &self.stream.path
    }

    /// The cursor for the candle poller (the last processed candle).
    pub fn candle_cursor(&self) -> CandleCursor {
        CandleCursor {
            last_open_ms: self.state.last_candle_time_ms,
            last_close: self.state.prev_close,
        }
    }

    /// Feed candle history on a cold start. Records the candles so a replay can reproduce the state.
    pub fn warm_from_candles(&mut self, candles: &[Candle]) -> Result<usize, LiveError> {
        let mut fed = 0usize;
        for c in candles {
            match step(&self.state, Input::ClosedCandle(*c), &self.cfg) {
                Ok((ns, outs)) => {
                    self.state = ns;
                    self.stream.append(&Input::ClosedCandle(*c))?;
                    self.log_outputs(&outs);
                    fed += 1;
                }
                Err(e) => {
                    // History must be contiguous (the poller backfills/forward-fills); a gap here
                    // aborts the warm and is logged by the supervisor.
                    return Err(LiveError::Warm(format!(
                        "{} at {}: {e}",
                        self.symbol, c.open_time_ms
                    )));
                }
            }
        }
        Ok(fed)
    }

    /// Process one live input. Core rejections become events/logs, never errors; IO errors propagate.
    pub fn process(&mut self, input: LiveInput, now_ms: i64) -> Result<Vec<LiveEvent>, LiveError> {
        if self.quarantined.is_some() {
            return Ok(Vec::new());
        }
        match input {
            LiveInput::Price { price, at_ms } => self.on_price(price, at_ms, now_ms),
            LiveInput::Candle(c) => self.on_candle(c, now_ms),
        }
    }

    fn on_price(
        &mut self,
        price: Decimal,
        at_ms: i64,
        now_ms: i64,
    ) -> Result<Vec<LiveEvent>, LiveError> {
        let age = now_ms.saturating_sub(at_ms);
        if age > self.staleness_ms {
            // A price older than the staleness limit is not fed; recovery follows §4 (Resume).
            self.needs_resume = true;
            log::warn!(
                "symbol={} stale price age_ms={age} > {} (not fed)",
                self.symbol,
                self.staleness_ms
            );
            return Ok(vec![LiveEvent::StalePrice { at_ms, age_ms: age }]);
        }
        // A feed gap (no price for longer than the staleness limit) also forces a Resume.
        if let Some(last) = self.last_price_at_ms {
            if at_ms.saturating_sub(last) > self.staleness_ms {
                self.needs_resume = true;
            }
        }

        let mut events = Vec::new();
        if self.needs_resume {
            let (ns, _) = step(&self.state, Input::Resume, &self.cfg)
                .map_err(|e| LiveError::Strategy(e.to_string()))?;
            self.state = ns;
            self.needs_resume = false;
            log::info!("symbol={} resume (last price forgotten)", self.symbol);
            events.push(LiveEvent::Resume);
        }

        let tick = Tick {
            time_ms: at_ms,
            price,
            is_bar_open: false,
        };
        match step(&self.state, Input::PriceTick(tick), &self.cfg) {
            Ok((ns, outs)) => {
                self.state = ns;
                self.stream.append(&Input::PriceTick(tick))?;
                self.last_price_at_ms = Some(at_ms);
                let consumed = outs.iter().any(is_zone_consume);
                events.extend(self.translate(outs));
                if consumed {
                    self.persist(now_ms)?;
                }
            }
            Err(e) => {
                log::warn!("symbol={} tick dropped: {e}", self.symbol);
                events.push(LiveEvent::DroppedInput {
                    reason: e.to_string(),
                });
            }
        }
        Ok(events)
    }

    fn on_candle(&mut self, c: Candle, now_ms: i64) -> Result<Vec<LiveEvent>, LiveError> {
        match step(&self.state, Input::ClosedCandle(c), &self.cfg) {
            Ok((ns, outs)) => {
                self.state = ns;
                self.stream.append(&Input::ClosedCandle(c))?;
                self.consecutive_candle_errors = 0;
                let events = self.translate(outs);
                self.persist(now_ms)?;
                Ok(events)
            }
            Err(e) => {
                self.consecutive_candle_errors += 1;
                log::warn!(
                    "symbol={} candle {} error ({}/{}): {e}",
                    self.symbol,
                    c.open_time_ms,
                    self.consecutive_candle_errors,
                    CANDLE_ERROR_THRESHOLD
                );
                let mut events = vec![LiveEvent::DroppedInput {
                    reason: e.to_string(),
                }];
                if self.consecutive_candle_errors >= CANDLE_ERROR_THRESHOLD {
                    self.quarantined = Some(e.to_string());
                    log::warn!(
                        "symbol={} QUARANTINED after {} candle errors: {e}",
                        self.symbol,
                        self.consecutive_candle_errors
                    );
                    events.push(LiveEvent::Quarantine {
                        reason: e.to_string(),
                    });
                }
                Ok(events)
            }
        }
    }

    fn persist(&self, now_ms: i64) -> Result<(), LiveError> {
        self.store.save(&self.state, now_ms)?;
        Ok(())
    }

    fn translate(&self, outs: Vec<Output>) -> Vec<LiveEvent> {
        outs.into_iter()
            .map(|o| match o {
                Output::EntrySignal(s) => {
                    log::info!(
                        "symbol={} SIGNAL direction={:?} entry={} sl={} tp={} zone={} rr={}",
                        self.symbol,
                        s.direction,
                        s.reference_entry_price,
                        s.stop_loss,
                        s.take_profit,
                        s.zone_id,
                        s.gross_rr
                    );
                    LiveEvent::Signal(s)
                }
                Output::ZoneEvent(ev) => {
                    log::info!(
                        "symbol={} zone={} kind={:?} reason={:?}",
                        self.symbol,
                        ev.zone_id,
                        ev.kind,
                        ev.reason
                    );
                    LiveEvent::ZoneEvent(ev)
                }
                Output::StateChange { reason } => {
                    log::info!("symbol={} state_change reason={reason}", self.symbol);
                    LiveEvent::StateChange {
                        reason: reason.to_string(),
                    }
                }
            })
            .collect()
    }

    fn log_outputs(&self, outs: &[Output]) {
        for o in outs {
            match o {
                Output::EntrySignal(s) => log::info!(
                    "symbol={} warm SIGNAL zone={} entry={}",
                    self.symbol,
                    s.zone_id,
                    s.reference_entry_price
                ),
                Output::ZoneEvent(ev) => log::info!(
                    "symbol={} warm zone={} kind={:?} reason={:?}",
                    self.symbol,
                    ev.zone_id,
                    ev.kind,
                    ev.reason
                ),
                Output::StateChange { reason } => {
                    log::info!("symbol={} warm state_change reason={reason}", self.symbol)
                }
            }
        }
    }
}

/// True for a zone output that represents a consumption (a persist trigger, §4).
fn is_zone_consume(o: &Output) -> bool {
    matches!(o, Output::ZoneEvent(ev) if ev.kind == ZoneEventKind::Consumed)
}
