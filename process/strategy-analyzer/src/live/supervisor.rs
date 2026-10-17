//! Supervisor (live spec §3, §3c, §5a).
//!
//! Spawns the single shared `allTickers` poller and one loop per configured symbol, then waits for a
//! shutdown signal (SIGINT/SIGTERM). Loops stop on shutdown and their state is retained. A task that
//! panics is logged and not respawned, so a bad data replay cannot crash-loop the process. Phase 1
//! has **no gRPC server** and publishes nothing.

use crate::live::clock::Clock;
use crate::live::config::{LiveConfig, LiveConfigError, TIMEFRAME_MS};
use crate::live::poller::{
    CandleCursor, CandlePoller, MarketData, SharedTickerPoller, TickerSnapshot, CANDLE_LOOKBACK,
};
use crate::live::state_store::{StateStore, StateStoreError};
use crate::live::sym_loop::{LiveEvent, LiveInput, SymbolLoop};
use crate::strategy::config::Config;
use crate::strategy::STATE_SCHEMA_VERSION;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinSet;

/// Errors from the live adapter.
#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error(transparent)]
    Config(#[from] LiveConfigError),
    #[error(transparent)]
    Store(#[from] StateStoreError),
    #[error("recorded-stream io error on {path}: {source}")]
    Stream {
        path: String,
        source: std::io::Error,
    },
    #[error("recorded-stream serialize error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("warm-from-history failed: {0}")]
    Warm(String),
    #[error("strategy core error: {0}")]
    Strategy(String),
}

/// How often the exchanged-time offset is refreshed (spec §3a "refreshed periodically").
const CLOCK_REFRESH: Duration = Duration::from_secs(300);

/// Owns the loops and the state store for one analyzer instance.
pub struct Supervisor {
    cfg: LiveConfig,
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    store: Arc<StateStore>,
    strategy_cfg: Config,
}

impl Supervisor {
    pub fn new(
        cfg: LiveConfig,
        market: Arc<dyn MarketData>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, LiveError> {
        cfg.validate()?;
        let store = Arc::new(StateStore::open(
            cfg.state_dir_path(),
            STATE_SCHEMA_VERSION,
        )?);
        let refs: Vec<&str> = cfg.symbols.iter().map(String::as_str).collect();
        let strategy_cfg = Config::default_for(&refs);
        Ok(Supervisor {
            cfg,
            market,
            clock,
            store,
            strategy_cfg,
        })
    }

    /// Run until `shutdown` resolves or every symbol loop has stopped. Loops stop on shutdown and
    /// their state files are retained.
    pub async fn run(self, shutdown: impl Future<Output = ()> + Send) -> Result<(), LiveError> {
        let Supervisor {
            cfg,
            market,
            clock,
            store,
            strategy_cfg,
        } = self;

        // Phase 1 has no `Analyze` call: prune state for symbols no longer in the explicit list.
        let removed = store.prune(&cfg.symbols)?;
        if !removed.is_empty() {
            log::info!("pruned inactive symbol state: {removed:?}");
        }

        // Measure the exchange offset once, then refresh on a heartbeat.
        sync_clock(market.clone(), clock.clone()).await;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (ticker_tx, ticker_handle) = SharedTickerPoller::spawn(
            market.clone(),
            clock.clone(),
            cfg.poll_interval(),
            shutdown_rx.clone(),
        );
        let heartbeat = {
            let market = market.clone();
            let clock = clock.clone();
            let mut sd = shutdown_rx.clone();
            tokio::spawn(async move {
                let mut t = tokio::time::interval(CLOCK_REFRESH);
                t.tick().await; // consume the immediate first tick
                loop {
                    tokio::select! {
                        r = sd.changed() => { if r.is_err() || *sd.borrow() { break; } }
                        _ = t.tick() => sync_clock(market.clone(), clock.clone()).await,
                    }
                }
            })
        };

        let mut set: JoinSet<()> = JoinSet::new();
        for symbol in cfg.symbols.iter().cloned() {
            let deps = SymbolDeps {
                symbol,
                strategy_cfg: strategy_cfg.clone(),
                live_cfg: cfg.clone(),
                store: store.clone(),
                market: market.clone(),
                clock: clock.clone(),
                rx: ticker_tx.subscribe(),
                shutdown: shutdown_rx.clone(),
            };
            set.spawn(run_symbol(deps));
        }

        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    log::info!("shutdown signal received; stopping loops");
                    break;
                }
                joined = set.join_next() => match joined {
                    Some(Ok(())) => {}
                    Some(Err(e)) => log::warn!("symbol task ended abnormally (not respawned): {e}"),
                    None => break,
                },
            }
        }

        // Stop the pollers and any remaining loops; state is already persisted.
        let _ = shutdown_tx.send(true);
        heartbeat.abort();
        ticker_handle.abort();
        set.abort_all();
        while set.join_next().await.is_some() {}
        Ok(())
    }
}

/// Everything one symbol task needs, grouped to keep the spawn site readable.
struct SymbolDeps {
    symbol: String,
    strategy_cfg: Config,
    live_cfg: LiveConfig,
    store: Arc<StateStore>,
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    rx: broadcast::Receiver<TickerSnapshot>,
    shutdown: watch::Receiver<bool>,
}

async fn run_symbol(deps: SymbolDeps) {
    let SymbolDeps {
        symbol,
        strategy_cfg,
        live_cfg,
        store,
        market,
        clock,
        mut rx,
        mut shutdown,
    } = deps;

    let stream_dir = live_cfg.stream_dir_path();
    let poller = CandlePoller::new(
        live_cfg.candle_interval.clone(),
        live_cfg.timeframe_ms(),
        live_cfg.candle_settle_ms,
        CANDLE_LOOKBACK,
    );

    let mut lp = match SymbolLoop::new(
        symbol.clone(),
        strategy_cfg,
        store,
        &stream_dir,
        live_cfg.price_staleness_ms,
        clock.exchange_now_ms(),
    ) {
        Ok(lp) => lp,
        Err(e) => {
            log::warn!("symbol={symbol} could not start: {e}");
            return;
        }
    };

    if lp.is_cold() {
        warm_from_history(&symbol, &market, &clock, &poller, &mut lp).await;
        if lp.is_quarantined() {
            return;
        }
    }

    // Check for a newly-closed 4H candle every poll tick, but only hit the network once the next
    // boundary's candle has settled — at most one candle fetch per 4H per symbol (spec §8 budget).
    let mut check = tokio::time::interval(live_cfg.poll_interval());
    let mut next_open = next_open_after(clock.exchange_now_ms(), lp.state().last_candle_time_ms);

    loop {
        tokio::select! {
            r = shutdown.changed() => {
                if r.is_err() || *shutdown.borrow() { break; }
            }
            recv = rx.recv() => match recv {
                Ok(snap) => {
                    if let Some(price) = snap.prices.get(&symbol).copied() {
                        let now = clock.exchange_now_ms();
                        match lp.process(LiveInput::Price { price, at_ms: snap.at_ms }, now) {
                            Ok(events) => if stop_requested(&events) {
                                log::warn!("symbol={symbol} stopping (quarantined)");
                                break;
                            },
                            Err(e) => log::warn!("symbol={symbol} price processing failed: {e}"),
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("symbol={symbol} ticker receiver lagged by {n}; recovery via Resume");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            _ = check.tick() => {
                let now = clock.exchange_now_ms();
                if now >= next_open + TIMEFRAME_MS + live_cfg.candle_settle_ms {
                    if poll_candles(&symbol, &market, &poller, &mut lp, now).await {
                        log::warn!("symbol={symbol} stopping (quarantined)");
                        break;
                    }
                    next_open = next_open_after(now, lp.state().last_candle_time_ms);
                }
            }
        }
    }
    log::info!("symbol={symbol} loop stopped (state retained)");
}

/// Fetch history for a cold symbol and warm the loop. A failure is logged and the loop starts
/// degraded (it will catch up on the next boundary).
async fn warm_from_history(
    symbol: &str,
    market: &Arc<dyn MarketData>,
    clock: &Arc<dyn Clock>,
    poller: &CandlePoller,
    lp: &mut SymbolLoop,
) {
    let now = clock.exchange_now_ms();
    let m = market.clone();
    let p = poller.clone();
    let sym = symbol.to_string();
    match tokio::task::spawn_blocking(move || p.poll(&*m, &sym, CandleCursor::default(), now)).await
    {
        Ok(Ok(candles)) => match lp.warm_from_candles(&candles) {
            Ok(n) => log::info!("symbol={symbol} warmed {n} candles from history"),
            Err(e) => log::warn!("symbol={symbol} warm feed failed: {e}"),
        },
        Ok(Err(e)) => log::warn!("symbol={symbol} warm fetch failed: {e}"),
        Err(e) => log::warn!("symbol={symbol} warm fetch task failed: {e}"),
    }
}

/// Poll and feed the symbol's closed candles. Returns `true` if the loop should stop (quarantined).
async fn poll_candles(
    symbol: &str,
    market: &Arc<dyn MarketData>,
    poller: &CandlePoller,
    lp: &mut SymbolLoop,
    now_ms: i64,
) -> bool {
    let cursor = lp.candle_cursor();
    let m = market.clone();
    let p = poller.clone();
    let sym = symbol.to_string();
    let candles = match tokio::task::spawn_blocking(move || p.poll(&*m, &sym, cursor, now_ms)).await
    {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            log::warn!("symbol={symbol} candle poll failed: {e}");
            return false;
        }
        Err(e) => {
            log::warn!("symbol={symbol} candle poll task failed: {e}");
            return false;
        }
    };
    for c in candles {
        match lp.process(LiveInput::Candle(c), now_ms) {
            Ok(events) => {
                if stop_requested(&events) {
                    return true;
                }
            }
            Err(e) => {
                log::warn!("symbol={symbol} candle processing failed: {e}");
                return false;
            }
        }
    }
    false
}

fn stop_requested(events: &[LiveEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, LiveEvent::Quarantine { .. }))
}

async fn sync_clock(market: Arc<dyn MarketData>, clock: Arc<dyn Clock>) {
    let m = market.clone();
    match tokio::task::spawn_blocking(move || m.exchange_time_ms()).await {
        Ok(Ok(t)) => clock.sync(t),
        Ok(Err(e)) => log::warn!("exchange-time sync failed: {e}"),
        Err(e) => log::warn!("exchange-time sync task failed: {e}"),
    }
}

/// The next 4H open time to wait for: the candle after the last processed one, or the next grid
/// boundary when starting fresh.
fn next_open_after(now_ms: i64, last_candle_ms: Option<i64>) -> i64 {
    match last_candle_ms {
        Some(l) => l + TIMEFRAME_MS,
        None => now_ms.div_euclid(TIMEFRAME_MS) * TIMEFRAME_MS + TIMEFRAME_MS,
    }
}

/// Resolves on SIGINT or (on Unix) SIGTERM.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => log::info!("SIGINT received"),
                    _ = term.recv() => log::info!("SIGTERM received"),
                }
            }
            Err(e) => {
                log::warn!("could not install SIGTERM handler ({e}); waiting for SIGINT only");
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_open_uses_the_last_candle_or_the_grid() {
        // Known last candle: the next open is exactly one timeframe later.
        assert_eq!(next_open_after(0, Some(0)), TIMEFRAME_MS);
        // Unknown: the next grid boundary strictly after now.
        assert_eq!(next_open_after(1, None), TIMEFRAME_MS);
        assert_eq!(next_open_after(TIMEFRAME_MS, None), 2 * TIMEFRAME_MS);
    }

    #[test]
    fn stop_requested_detects_quarantine() {
        assert!(!stop_requested(&[LiveEvent::Resume]));
        assert!(stop_requested(&[LiveEvent::Quarantine {
            reason: "x".into()
        }]));
    }
}
