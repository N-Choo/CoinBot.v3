use crate::driver::{CandleDriver, DriverEvent};
use crate::exchange::kucoin::{fetch_klines, ExchangeError};
use crate::live::config::LiveConfig;
use crate::live::sink::Sink;
use crate::strategy::config::Config;
use crate::strategy::error::Error;
use crate::strategy::types::Candle;
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error(transparent)]
    Strategy(#[from] Error),
    #[error(transparent)]
    Exchange(#[from] ExchangeError),
    #[error("not enough closed candles for {symbol}: {got} < {need}")]
    InsufficientHistory {
        symbol: String,
        got: usize,
        need: usize,
    },
    #[error("fetch task failed: {0}")]
    Join(String),
}

pub fn process_batch(
    driver: &mut CandleDriver,
    candles: &[Candle],
    sink: &Sink,
) -> Result<usize, LiveError> {
    let mut emitted = 0usize;
    for candle in candles {
        for event in driver.push_closed(candle.clone())? {
            if let DriverEvent::Signal { signal, .. } = event {
                sink.publish(signal);
                emitted += 1;
            }
        }
    }
    Ok(emitted)
}

fn fetch_blocking(cfg: &LiveConfig, symbol: &str, limit: usize) -> Result<Vec<Candle>, LiveError> {
    Ok(fetch_klines(
        &cfg.base_url,
        symbol,
        cfg.timeframe,
        limit,
        None,
    )?)
}

pub async fn warm_driver(
    cfg: &LiveConfig,
    strategy: &Config,
    symbol: &str,
) -> Result<(CandleDriver, i64), LiveError> {
    let cfg_owned = cfg.clone();
    let symbol_owned = symbol.to_string();
    let limit = cfg.window + 2;
    let candles =
        tokio::task::spawn_blocking(move || fetch_blocking(&cfg_owned, &symbol_owned, limit))
            .await
            .map_err(|e| LiveError::Join(e.to_string()))??;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut closed: Vec<Candle> = candles
        .into_iter()
        .filter(|c| {
            cfg.timeframe
                .is_closed(c.open_time_ms, now_ms, cfg.settle_ms)
        })
        .collect();
    if closed.len() < cfg.window {
        return Err(LiveError::InsufficientHistory {
            symbol: symbol.to_string(),
            got: closed.len(),
            need: cfg.window,
        });
    }
    closed.sort_by_key(|c| c.open_time_ms);
    let cutoff = closed.len() - cfg.window;
    let mut driver =
        CandleDriver::new(symbol, strategy.clone()).with_max_age(cfg.max_divergence_age_candles);
    let warm_sink = Sink::Collect(Default::default());
    process_batch(&mut driver, &closed[cutoff..], &warm_sink)?;
    let cursor = closed.last().map(|c| c.open_time_ms).unwrap_or(0);
    Ok((driver, cursor))
}

pub async fn run_symbol(
    cfg: LiveConfig,
    strategy: Config,
    symbol: String,
    sink: Sink,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), LiveError> {
    let (mut driver, mut cursor) = warm_driver(&cfg, &strategy, &symbol).await?;
    log::info!(
        "warmed symbol={symbol} candles={} window={}",
        driver.window().len(),
        cfg.window
    );
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { break; }
            }
            _ = tokio::time::sleep(Duration::from_millis(cfg.poll_ms)) => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                if now_ms - cfg.settle_ms < cursor + 2 * cfg.timeframe.millis() { continue; }
                let cfg_owned = cfg.clone();
                let symbol_owned = symbol.clone();
                let limit = cfg.window + 5;
                let result = tokio::task::spawn_blocking(move || fetch_blocking(&cfg_owned, &symbol_owned, limit))
                    .await
                    .map_err(|e| LiveError::Join(e.to_string()))?;
                let candles = match result {
                    Ok(candles) => candles,
                    Err(error) => { log::warn!("fetch failed symbol={symbol}: {error}"); continue; }
                };
                let now_ms = chrono::Utc::now().timestamp_millis();
                let mut new_candles: Vec<Candle> = candles.into_iter()
                    .filter(|c| c.open_time_ms > cursor && cfg.timeframe.is_closed(c.open_time_ms, now_ms, cfg.settle_ms))
                    .collect();
                new_candles.sort_by_key(|c| c.open_time_ms);
                let emitted = process_batch(&mut driver, &new_candles, &sink)?;
                // Advance only after the batch commits, so a failed batch is retried from the
                // same cursor on the next poll.
                if let Some(last) = new_candles.last() {
                    cursor = last.open_time_ms;
                }
                if emitted > 0 { log::info!("emitted {emitted} signal(s) symbol={symbol}"); }
            }
        }
    }
    log::info!("stopped symbol={symbol}");
    Ok(())
}

pub async fn run_symbol_once(
    cfg: &LiveConfig,
    strategy: &Config,
    symbol: &str,
    sink: &Sink,
) -> Result<(usize, usize), LiveError> {
    let cfg_owned = cfg.clone();
    let symbol_owned = symbol.to_string();
    let limit = cfg.window + 3;
    let candles =
        tokio::task::spawn_blocking(move || fetch_blocking(&cfg_owned, &symbol_owned, limit))
            .await
            .map_err(|e| LiveError::Join(e.to_string()))??;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut closed: Vec<Candle> = candles
        .into_iter()
        .filter(|c| {
            cfg.timeframe
                .is_closed(c.open_time_ms, now_ms, cfg.settle_ms)
        })
        .collect();
    if closed.len() < cfg.window + 1 {
        return Err(LiveError::InsufficientHistory {
            symbol: symbol.to_string(),
            got: closed.len(),
            need: cfg.window + 1,
        });
    }
    closed.sort_by_key(|c| c.open_time_ms);
    let mut driver =
        CandleDriver::new(symbol, strategy.clone()).with_max_age(cfg.max_divergence_age_candles);
    process_batch(
        &mut driver,
        &closed[..cfg.window],
        &Sink::Collect(Default::default()),
    )?;
    let emitted = process_batch(&mut driver, &closed[cfg.window..], sink)?;
    Ok((driver.window().len(), emitted))
}
