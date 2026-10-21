use crate::exchange::timeframes::Timeframe;
use crate::strategy::config::Config;
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub enum LiveConfigError {
    #[error("unsupported KUCOIN_TIMEFRAME {0:?} (use 15min, 4hour, or 1day)")]
    UnsupportedTimeframe(String),
    #[error("invalid {0}={1}")]
    Invalid(&'static str, String),
    #[error("window must be >= 8")]
    WindowTooSmall,
    #[error("window {window} is below the strategy minimum {need}")]
    WindowBelowStrategy { window: usize, need: usize },
    #[error("poll interval must be > 0")]
    PollIntervalZero,
    #[error("activation poll interval must be > 0")]
    ActivationPollIntervalZero,
    #[error("candle settle must be >= 0")]
    NegativeSettle,
}

#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub symbols: Vec<String>,
    pub timeframe: Timeframe,
    pub window: usize,
    pub settle_ms: i64,
    pub poll_ms: u64,
    pub activation_poll_ms: u64,
    pub endpoint: String,
    pub base_url: String,
}

impl LiveConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbols: Vec<String>,
        timeframe: &str,
        window: usize,
        settle_ms: i64,
        poll_ms: u64,
        activation_poll_ms: u64,
        endpoint: String,
        base_url: String,
    ) -> Result<Self, LiveConfigError> {
        let timeframe = Timeframe::from_kucoin(timeframe)
            .ok_or_else(|| LiveConfigError::UnsupportedTimeframe(timeframe.to_string()))?;
        if window < 8 {
            return Err(LiveConfigError::WindowTooSmall);
        }
        if settle_ms < 0 {
            return Err(LiveConfigError::NegativeSettle);
        }
        if poll_ms == 0 {
            return Err(LiveConfigError::PollIntervalZero);
        }
        if activation_poll_ms == 0 {
            return Err(LiveConfigError::ActivationPollIntervalZero);
        }

        let mut symbols: Vec<String> = symbols
            .into_iter()
            .map(|symbol| symbol.trim().replace('/', "-"))
            .filter(|symbol| !symbol.is_empty())
            .collect();
        let mut seen = BTreeSet::new();
        symbols.retain(|symbol| seen.insert(symbol.clone()));

        Ok(LiveConfig {
            symbols,
            timeframe,
            window,
            settle_ms,
            poll_ms,
            activation_poll_ms,
            endpoint,
            base_url,
        })
    }

    pub fn from_env() -> Result<Self, LiveConfigError> {
        let symbols = std::env::var("LIVE_SYMBOLS")
            .unwrap_or_default()
            .split(',')
            .map(str::to_string)
            .collect();
        let timeframe = std::env::var("KUCOIN_TIMEFRAME").unwrap_or_else(|_| "15min".into());
        let window = parse("ANALYZER_WINDOW_CANDLES", 100)?;
        let settle_ms = parse("ANALYZER_CANDLE_SETTLE_MS", 5_000)?;
        let poll_ms = parse("ANALYZER_POLL_INTERVAL_MS", 1_000)?;
        let activation_poll_ms = parse("ANALYZER_ACTIVATION_POLL_MS", 30_000)?;
        let endpoint = std::env::var("GRPC_TRADE_ENGINE_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".into());
        let base_url =
            std::env::var("KUCOIN_BASE_URL").unwrap_or_else(|_| "https://api.kucoin.com".into());
        Self::new(
            symbols,
            &timeframe,
            window,
            settle_ms,
            poll_ms,
            activation_poll_ms,
            endpoint,
            base_url,
        )
    }

    /// Reject a live window that cannot satisfy the strategy's indicator warm-up.
    pub fn validate_strategy(&self, strategy: &Config) -> Result<(), LiveConfigError> {
        let need = strategy.min_window();
        if self.window < need {
            return Err(LiveConfigError::WindowBelowStrategy {
                window: self.window,
                need,
            });
        }
        Ok(())
    }
}

fn parse<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, LiveConfigError> {
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|_| LiveConfigError::Invalid(key, value)),
        Err(_) => Ok(default),
    }
}
