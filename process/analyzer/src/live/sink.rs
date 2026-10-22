use crate::strategy::types::Signal;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum Sink {
    Log,
    Channel(tokio::sync::mpsc::Sender<Signal>),
    Collect(Arc<Mutex<Vec<Signal>>>),
}

impl Sink {
    pub fn publish(&self, signal: Signal) {
        match self {
            Sink::Log => println!(
                "SIGNAL symbol={} action={:?} entry={} sl={} tp={} reason={} age={}c",
                signal.ticker,
                signal.action,
                signal.entry_price,
                signal.sl_price,
                signal.tp_price,
                signal.reason,
                signal.divergence_age_candles
            ),
            Sink::Channel(tx) => {
                if tx.try_send(signal).is_err() {
                    eprintln!("signal queue full/closed; dropped");
                }
            }
            Sink::Collect(buffer) => buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(signal),
        }
    }
}
