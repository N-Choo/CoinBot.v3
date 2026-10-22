use crate::strategy::config::Config;
use crate::strategy::error::Result;
use crate::strategy::generator::{candidates, signals};
use crate::strategy::types::{Candle, Direction, DivergenceKind, Signal};
use crate::strategy::window::Window;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DivergenceKey {
    pub symbol: String,
    pub action: Direction,
    pub kind: DivergenceKind,
    pub divergence_open_time_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressReason {
    Duplicate,
    BelowWatermark,
    TooOld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    Warmed {
        seeded_keys: usize,
    },
    Signal {
        signal: Signal,
        key: DivergenceKey,
    },
    Suppressed {
        key: DivergenceKey,
        reason: SuppressReason,
    },
}

pub struct CandleDriver {
    symbol: String,
    window: Window,
    config: Config,
    emitted: BTreeSet<DivergenceKey>,
    watermark_ms: Option<i64>,
    warmed: bool,
    max_age_candles: Option<usize>,
}

impl CandleDriver {
    pub fn new(symbol: impl Into<String>, config: Config) -> Self {
        let capacity = config.window;
        CandleDriver {
            symbol: symbol.into(),
            window: Window::new(capacity),
            config,
            emitted: BTreeSet::new(),
            watermark_ms: None,
            warmed: false,
            max_age_candles: None,
        }
    }

    pub fn with_max_age(mut self, max_age_candles: Option<usize>) -> Self {
        self.max_age_candles = max_age_candles;
        self
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }
    pub fn window(&self) -> &[Candle] {
        self.window.as_slice()
    }
    pub fn is_warmed(&self) -> bool {
        self.warmed
    }

    pub fn push_closed(&mut self, candle: Candle) -> Result<Vec<DriverEvent>> {
        self.window.push_closed(candle)?;

        if !self.warmed {
            if !self.window.is_full() {
                return Ok(Vec::new());
            }
            let seeded = candidates(self.window.as_slice(), &self.config)?;
            for candidate in &seeded {
                let key = self.key(candidate.kind, candidate.index);
                self.watermark_ms = Some(match self.watermark_ms {
                    Some(current) => current.max(key.divergence_open_time_ms),
                    None => key.divergence_open_time_ms,
                });
                self.emitted.insert(key);
            }
            self.warmed = true;
            return Ok(vec![DriverEvent::Warmed {
                seeded_keys: seeded.len(),
            }]);
        }

        let Some(signal) = signals(self.window.as_slice(), &self.symbol, &self.config)? else {
            return Ok(Vec::new());
        };
        let key = self.key(signal.divergence_kind, signal.divergence_index);

        if self.emitted.contains(&key) {
            return Ok(vec![DriverEvent::Suppressed {
                key,
                reason: SuppressReason::Duplicate,
            }]);
        }
        if self
            .watermark_ms
            .is_some_and(|watermark| key.divergence_open_time_ms <= watermark)
        {
            return Ok(vec![DriverEvent::Suppressed {
                key,
                reason: SuppressReason::BelowWatermark,
            }]);
        }
        if self
            .max_age_candles
            .is_some_and(|max| signal.divergence_age_candles > max)
        {
            return Ok(vec![DriverEvent::Suppressed {
                key,
                reason: SuppressReason::TooOld,
            }]);
        }

        self.emitted.insert(key.clone());
        self.watermark_ms = Some(key.divergence_open_time_ms);
        Ok(vec![DriverEvent::Signal { signal, key }])
    }

    fn key(&self, kind: DivergenceKind, index: usize) -> DivergenceKey {
        DivergenceKey {
            symbol: self.symbol.clone(),
            action: kind.action(),
            kind,
            divergence_open_time_ms: self.window.as_slice()[index].open_time_ms,
        }
    }
}
