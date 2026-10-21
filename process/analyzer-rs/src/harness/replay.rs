use crate::driver::{CandleDriver, DriverEvent, SuppressReason};
use crate::harness::dataset::{Dataset, Provenance};
use crate::identity::{config_version, signal_id};
use crate::strategy::config::Config;
use crate::strategy::error::{Error as StrategyError, Result as StrategyResult};
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::indicators::sma::sma;
use crate::strategy::types::{Candle, Signal};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayEvent {
    Warmup {
        candle_index: usize,
        seeded_keys: usize,
    },
    Signal {
        candle_index: usize,
        signal_index: usize,
    },
    Suppressed {
        candle_index: usize,
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct ReplaySignal {
    pub candle_index: usize,
    pub signal: Signal,
    pub signal_id: String,
}

#[derive(Debug, Clone)]
pub struct Replay {
    pub symbol: String,
    pub candles: Vec<Candle>,
    pub rsi: Vec<Option<Decimal>>,
    pub sma: Vec<Option<Decimal>>,
    pub events: Vec<ReplayEvent>,
    pub signals: Vec<ReplaySignal>,
    pub provenance: Provenance,
}

fn suppress_reason(reason: SuppressReason) -> &'static str {
    match reason {
        SuppressReason::Duplicate => "duplicate",
        SuppressReason::BelowWatermark => "below_watermark",
        SuppressReason::TooOld => "too_old",
    }
}

pub fn build_replay(
    dataset: &Dataset,
    cfg: &Config,
    max_age_candles: Option<usize>,
) -> StrategyResult<Replay> {
    cfg.validate()?;
    if dataset.candles.len() < cfg.window + 2 {
        return Err(StrategyError::ConfigInvalid(
            "dataset shorter than window".into(),
        ));
    }

    let window = cfg.window;
    let mut driver = CandleDriver::new(&dataset.symbol, cfg.clone()).with_max_age(max_age_candles);
    let mut events = Vec::new();
    let mut signals = Vec::new();
    let version = config_version(cfg);

    for (index, candle) in dataset.candles.iter().enumerate() {
        for event in driver.push_closed(candle.clone())? {
            match event {
                DriverEvent::Warmed { seeded_keys } => events.push(ReplayEvent::Warmup {
                    candle_index: index,
                    seeded_keys,
                }),
                DriverEvent::Signal { signal, .. } => {
                    signals.push(ReplaySignal {
                        candle_index: index,
                        signal_id: signal_id(&signal, &version),
                        signal,
                    });
                    events.push(ReplayEvent::Signal {
                        candle_index: index,
                        signal_index: signals.len() - 1,
                    });
                }
                DriverEvent::Suppressed { reason, .. } => events.push(ReplayEvent::Suppressed {
                    candle_index: index,
                    reason: suppress_reason(reason).to_string(),
                }),
            }
        }
    }

    let closes: Vec<Option<Decimal>> = dataset.candles.iter().map(|c| Some(c.close)).collect();
    let rsi_values = (0..closes.len())
        .map(|index| {
            if index + 1 < window {
                return Ok(None);
            }
            let window_closes = &closes[index + 1 - window..=index];
            let values = rsi(window_closes, cfg.rsi_period)?;
            Ok(values.last().copied().flatten())
        })
        .collect::<StrategyResult<Vec<Option<Decimal>>>>()?;
    // SMA is time-invariant once its period is covered, so the full-series computation
    // matches what any rolling window would show.
    let sma_values = sma(&closes, cfg.sma_period)?;

    Ok(Replay {
        symbol: dataset.symbol.clone(),
        candles: dataset.candles.clone(),
        rsi: rsi_values,
        sma: sma_values,
        events,
        signals,
        provenance: dataset.provenance.clone(),
    })
}
