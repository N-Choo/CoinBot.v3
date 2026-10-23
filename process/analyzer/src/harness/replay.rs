use crate::driver::{CandleDriver, DriverEvent};
use crate::harness::dataset::{Dataset, Provenance};
use crate::identity::{config_version, signal_id};
use crate::strategy::config::Config;
use crate::strategy::error::{Error as StrategyError, Result as StrategyResult};
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::types::{Candle, Signal};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayEvent {
    Warmup {
        candle_index: usize,
    },
    Signal {
        candle_index: usize,
        signal_index: usize,
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
    pub events: Vec<ReplayEvent>,
    pub signals: Vec<ReplaySignal>,
    pub provenance: Provenance,
}

pub fn build_replay(dataset: &Dataset, cfg: &Config) -> StrategyResult<Replay> {
    cfg.validate()?;
    let need = cfg.window.saturating_add(2);
    if dataset.candles.len() < need {
        return Err(StrategyError::ConfigInvalid(format!(
            "dataset shorter than window + 2: got {} need {}",
            dataset.candles.len(),
            need
        )));
    }

    let window = cfg.window;
    let mut driver = CandleDriver::new(&dataset.symbol, cfg.clone());
    let mut events = Vec::new();
    let mut signals = Vec::new();
    let version = config_version(cfg);

    for (index, candle) in dataset.candles.iter().enumerate() {
        for event in driver.push_closed(candle.clone())? {
            match event {
                DriverEvent::Warmed => events.push(ReplayEvent::Warmup {
                    candle_index: index,
                }),
                DriverEvent::Signal { signal } => {
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

    Ok(Replay {
        symbol: dataset.symbol.clone(),
        candles: dataset.candles.clone(),
        rsi: rsi_values,
        events,
        signals,
        provenance: dataset.provenance.clone(),
    })
}
