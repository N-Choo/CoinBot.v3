use crate::strategy::config::Config;
use crate::strategy::error::Result;
use crate::strategy::generator::signals;
use crate::strategy::types::{Candle, Signal};
use crate::strategy::window::Window;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    Warmed,
    Signal { signal: Signal },
}

pub struct CandleDriver {
    symbol: String,
    window: Window,
    config: Config,
    warmed: bool,
}

impl CandleDriver {
    pub fn new(symbol: impl Into<String>, config: Config) -> Self {
        let capacity = config.window;
        CandleDriver {
            symbol: symbol.into(),
            window: Window::new(capacity),
            config,
            warmed: false,
        }
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
            self.warmed = true;
            return Ok(vec![DriverEvent::Warmed]);
        }

        Ok(
            match signals(self.window.as_slice(), &self.symbol, &self.config)? {
                Some(signal) => vec![DriverEvent::Signal { signal }],
                None => Vec::new(),
            },
        )
    }
}
