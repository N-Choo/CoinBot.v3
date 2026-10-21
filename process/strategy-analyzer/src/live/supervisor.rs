//! Supervisor (live spec §3, §3c, §5a).
//!
//! Spawns the single shared `allTickers` poller and one loop per configured symbol, then waits for a
//! shutdown signal (SIGINT/SIGTERM). Loops stop on shutdown and their state is retained. A task that
//! panics is logged and not respawned, so a bad data replay cannot crash-loop the process. Phase 1
//! has **no gRPC server** and publishes nothing.

use crate::live::clock::Clock;
use crate::live::config::{LiveConfig, LiveConfigError};
use crate::live::poller::{
    with_retry, CandleCursor, CandlePoller, MarketData, MarketError, RateLimiter, RetryPolicy,
    SharedTickerPoller, TickerSnapshot, CANDLE_LOOKBACK,
};
use crate::live::state_store::{StateStore, StateStoreError};
use crate::live::sym_loop::{LiveEvent, LiveInput, SymbolLoop};
use crate::live::{to_kucoin, SymbolState};
use crate::strategy::config::Config;
use crate::strategy::types::Signal;
use crate::strategy::STATE_SCHEMA_VERSION;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::{Id, JoinError, JoinSet};

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

/// How often the supervisor re-reads the engine's active set to add/stop per-symbol loops.
const ACTIVATION_TICK: Duration = Duration::from_secs(5);

/// Owns the loops and the state store for one analyzer instance.
pub struct Supervisor {
    cfg: LiveConfig,
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    limiter: Arc<RateLimiter>,
    retry: RetryPolicy,
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
        // The loops run the core with the configured candle interval (default 4h), so a live run on a
        // non-4H stream is accepted instead of being rejected as gapped.
        let strategy_cfg = Config {
            candle_interval_ms: cfg.timeframe_ms(),
            ..Config::default_for(&refs)
        };
        let limiter = Arc::new(RateLimiter::with_jitter(
            cfg.rate_limit_interval(),
            cfg.jitter_pct,
        ));
        let retry = RetryPolicy::new(cfg.max_retries, cfg.retry_base_ms, cfg.retry_max_ms);
        Ok(Supervisor {
            cfg,
            market,
            clock,
            limiter,
            retry,
            store,
            strategy_cfg,
        })
    }

    /// Run until `shutdown` resolves or every symbol loop has stopped. Loops stop on shutdown and
    /// their state files are retained. `signal_tx`, when present, receives every emitted entry
    /// signal for delivery to the engine (the sink is absent in log-only Phase 1 runs).
    pub async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send,
        signal_tx: Option<mpsc::Sender<Signal>>,
        active_symbols: Arc<SymbolState>,
    ) -> Result<(), LiveError> {
        let Supervisor {
            cfg,
            market,
            clock,
            limiter,
            retry,
            store,
            strategy_cfg,
        } = self;

        // Measure the exchange offset once, then refresh on a heartbeat.
        sync_clock(market.clone(), clock.clone(), limiter.clone(), retry).await;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (ticker_tx, ticker_handle) = SharedTickerPoller::spawn(
            market.clone(),
            clock.clone(),
            limiter.clone(),
            retry,
            cfg.poll_interval(),
            shutdown_rx.clone(),
        );
        let heartbeat = spawn_heartbeat(
            market.clone(),
            clock.clone(),
            limiter.clone(),
            retry,
            shutdown_rx,
        );

        // Shared inputs for starting loops on demand, plus the live worker set.
        let make = LoopFactory {
            strategy_cfg,
            live_cfg: cfg.clone(),
            store: store.clone(),
            market,
            clock,
            limiter,
            retry,
            ticker_tx,
            signal_tx,
        };
        let fallback: BTreeSet<String> = cfg.symbols.iter().map(|s| to_kucoin(s)).collect();
        let mut set: JoinSet<()> = JoinSet::new();
        let mut stops: BTreeMap<String, watch::Sender<bool>> = BTreeMap::new(); // symbol -> stop
        let mut sym_of: BTreeMap<Id, String> = BTreeMap::new(); // task id -> symbol
                                                                // The interval's first tick fires immediately, seeding the loops on entry.
        let mut activation = tokio::time::interval(ACTIVATION_TICK);

        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    log::info!("shutdown signal received; stopping loops");
                    break;
                }
                _ = activation.tick() => {
                    let snap: BTreeSet<String> = active_symbols.snapshot().into_iter().collect();
                    let desired = if snap.is_empty() { fallback.clone() } else { snap };
                    reconcile(&desired, &make, &mut set, &mut stops, &mut sym_of);
                    let removed = store.prune(&desired.into_iter().collect::<Vec<_>>())?;
                    if !removed.is_empty() {
                        log::info!("pruned inactive symbol state: {removed:?}");
                    }
                }
                joined = set.join_next_with_id() => reap(joined, &mut stops, &mut sym_of),
            }
        }

        // Stop the pollers and remaining loops, letting them persist before force-aborting.
        let _ = shutdown_tx.send(true);
        for stop in stops.values() {
            let _ = stop.send(true);
        }
        shutdown_workers(&mut set).await;
        heartbeat.abort();
        ticker_handle.abort();
        Ok(())
    }
}

/// Shared inputs for starting a per-symbol loop (everything re-cloned per spawn).
struct LoopFactory {
    strategy_cfg: Config,
    live_cfg: LiveConfig,
    store: Arc<StateStore>,
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    limiter: Arc<RateLimiter>,
    retry: RetryPolicy,
    ticker_tx: broadcast::Sender<TickerSnapshot>,
    signal_tx: Option<mpsc::Sender<Signal>>,
}

impl LoopFactory {
    /// Spawn one loop with its own stop channel; returns the task id and the stop handle.
    fn spawn(&self, symbol: String, set: &mut JoinSet<()>) -> (Id, watch::Sender<bool>) {
        let (stop_tx, stop_rx) = watch::channel(false);
        let deps = SymbolDeps {
            symbol,
            strategy_cfg: self.strategy_cfg.clone(),
            live_cfg: self.live_cfg.clone(),
            store: self.store.clone(),
            market: self.market.clone(),
            clock: self.clock.clone(),
            limiter: self.limiter.clone(),
            retry: self.retry,
            rx: self.ticker_tx.subscribe(),
            shutdown: stop_rx,
            signal_tx: self.signal_tx.clone(),
        };
        let id = set.spawn(run_symbol(deps)).id();
        (id, stop_tx)
    }
}

/// Add loops for newcomers and gracefully stop loops whose symbols are no longer desired.
fn reconcile(
    desired: &BTreeSet<String>,
    make: &LoopFactory,
    set: &mut JoinSet<()>,
    stops: &mut BTreeMap<String, watch::Sender<bool>>,
    sym_of: &mut BTreeMap<Id, String>,
) {
    let newcomers: Vec<String> = desired
        .iter()
        .filter(|s| !stops.contains_key(*s))
        .cloned()
        .collect();
    for symbol in newcomers {
        let (id, stop) = make.spawn(symbol.clone(), set);

        sym_of.insert(id, symbol.clone());
        stops.insert(symbol.clone(), stop);
        log::info!("symbol={symbol} watch activated");
    }

    let gone: Vec<String> = stops
        .keys()
        .filter(|s| !desired.contains(*s))
        .cloned()
        .collect();
    for symbol in gone {
        if let Some(stop) = stops.remove(&symbol) {
            let _ = stop.send(true);
            sym_of.retain(|_, s| s.as_str() != symbol.as_str());
            log::info!("symbol={symbol} deactivated");
        }
    }
}

/// Forget a finished or panicked loop so activation can respawn it later.
///
/// - `joined`: the result of `JoinSet::join_next_with_id` — the `Id` of the task that ended and
///   whether it finished normally (`Ok`) or panicked (`Err`).
/// - `stops`: per-symbol stop senders; the reaped symbol's entry is removed.
/// - `sym_of`: task-id → symbol map, used to recover the symbol for `joined`'s `Id`.
fn reap(
    joined: Option<Result<(Id, ()), JoinError>>,
    stops: &mut BTreeMap<String, watch::Sender<bool>>,
    sym_of: &mut BTreeMap<Id, String>,
) {
    match joined {
        Some(Ok((id, ()))) => {
            if let Some(sym) = sym_of.remove(&id) {
                stops.remove(&sym);
            }
        }
        Some(Err(e)) => {
            let id = e.id();
            match sym_of.remove(&id) {
                Some(sym) => log::warn!("symbol={sym} task ended abnormally (not respawned): {e}"),
                None => log::warn!("symbol task ended abnormally (not respawned): {e}"),
            }
        }
        None => {}
    }
}

/// Drain the loop set, giving loops a moment to persist before force-aborting stragglers.
async fn shutdown_workers(set: &mut JoinSet<()>) {
    let drained = tokio::time::timeout(Duration::from_secs(5), async {
        while set.join_next().await.is_some() {}
    })
    .await;
    if drained.is_err() {
        set.abort_all();
        while set.join_next().await.is_some() {}
    }
}

/// Spawn the exchange-clock heartbeat (re-syncs the offset every `CLOCK_REFRESH`).
fn spawn_heartbeat(
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    limiter: Arc<RateLimiter>,
    retry: RetryPolicy,
    shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    let mut sd = shutdown;
    tokio::spawn(async move {
        let mut t = tokio::time::interval(CLOCK_REFRESH);
        t.tick().await; // consume the immediate first tick
        loop {
            tokio::select! {
                r = sd.changed() => { if r.is_err() || *sd.borrow() { break; } }
                _ = t.tick() => sync_clock(market.clone(), clock.clone(), limiter.clone(), retry).await,
            }
        }
    })
}

/// Everything one symbol task needs, grouped to keep the spawn site readable.
struct SymbolDeps {
    symbol: String,
    strategy_cfg: Config,
    live_cfg: LiveConfig,
    store: Arc<StateStore>,
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    limiter: Arc<RateLimiter>,
    retry: RetryPolicy,
    rx: broadcast::Receiver<TickerSnapshot>,
    shutdown: watch::Receiver<bool>,
    /// Sink for emitted entry signals (None = log-only; no delivery).
    signal_tx: Option<mpsc::Sender<Signal>>,
}

async fn run_symbol(deps: SymbolDeps) {
    let SymbolDeps {
        symbol,
        strategy_cfg,
        live_cfg,
        store,
        market,
        clock,
        limiter,
        retry,
        mut rx,
        mut shutdown,
        signal_tx,
    } = deps;

    let stream_dir = live_cfg.stream_dir_path();
    let interval_ms = live_cfg.timeframe_ms();
    let poller = CandlePoller::new(
        live_cfg.candle_interval.clone(),
        interval_ms,
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
        warm_from_history(&symbol, &market, &clock, &limiter, retry, &poller, &mut lp).await;
        if lp.is_quarantined() {
            return;
        }
    }

    // Check for a newly-closed candle every poll tick, but only hit the network once the next
    // boundary's candle has settled — at most one candle fetch per interval per symbol (spec §8).
    let mut check = tokio::time::interval(live_cfg.poll_interval());
    let mut next_open = next_open_after(
        clock.exchange_now_ms(),
        lp.state().last_candle_time_ms,
        interval_ms,
    );

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
                            Ok(events) => {
                                forward_signals(&events, &signal_tx);
                                if stop_requested(&events) {
                                    log::warn!("symbol={symbol} stopping (quarantined)");
                                    break;
                                }
                            }
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
                if now >= next_open + interval_ms + live_cfg.candle_settle_ms {
                    if poll_candles(&symbol, &market, &limiter, retry, &poller, &mut lp, now, &signal_tx).await {
                        log::warn!("symbol={symbol} stopping (quarantined)");
                        break;
                    }
                    next_open = next_open_after(now, lp.state().last_candle_time_ms, interval_ms);
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
    limiter: &Arc<RateLimiter>,
    retry: RetryPolicy,
    poller: &CandlePoller,
    lp: &mut SymbolLoop,
) {
    let now = clock.exchange_now_ms();
    limiter.acquire().await;
    let m = market.clone();
    let p = poller.clone();
    let sym = symbol.to_string();
    let fetched = with_retry(retry, move || {
        let m = m.clone();
        let p = p.clone();
        let sym = sym.clone();
        async move {
            tokio::task::spawn_blocking(move || p.poll(&*m, &sym, CandleCursor::default(), now))
                .await
                .map_err(|e| MarketError::Fatal(format!("warm fetch join error: {e}")))?
        }
    })
    .await;
    match fetched {
        Ok(candles) => match lp.warm_from_candles(&candles) {
            Ok(n) => log::info!("symbol={symbol} warmed {n} candles from history"),
            Err(e) => log::warn!("symbol={symbol} warm feed failed: {e}"),
        },
        Err(e) => log::warn!("symbol={symbol} warm fetch failed: {e}"),
    }
}

/// Poll and feed the symbol's closed candles. Returns `true` if the loop should stop (quarantined).
#[allow(clippy::too_many_arguments)]
async fn poll_candles(
    symbol: &str,
    market: &Arc<dyn MarketData>,
    limiter: &Arc<RateLimiter>,
    retry: RetryPolicy,
    poller: &CandlePoller,
    lp: &mut SymbolLoop,
    now_ms: i64,
    signal_tx: &Option<mpsc::Sender<Signal>>,
) -> bool {
    let cursor = lp.candle_cursor();
    limiter.acquire().await;
    let m = market.clone();
    let p = poller.clone();
    let sym = symbol.to_string();
    let fetched = with_retry(retry, move || {
        let m = m.clone();
        let p = p.clone();
        let sym = sym.clone();
        async move {
            tokio::task::spawn_blocking(move || p.poll(&*m, &sym, cursor, now_ms))
                .await
                .map_err(|e| MarketError::Fatal(format!("candle poll join error: {e}")))?
        }
    })
    .await;
    let candles = match fetched {
        Ok(c) => c,
        Err(e) => {
            log::warn!("symbol={symbol} candle poll failed: {e}");
            return false;
        }
    };
    for c in candles {
        match lp.process(LiveInput::Candle(c), now_ms) {
            Ok(events) => {
                forward_signals(&events, signal_tx);
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

/// Forward any emitted entry signals into the (optional) sink for engine delivery.
fn forward_signals(events: &[LiveEvent], tx: &Option<mpsc::Sender<Signal>>) {
    let Some(tx) = tx else { return };
    for ev in events {
        if let LiveEvent::Signal(sig) = ev {
            if tx.try_send(sig.clone()).is_err() {
                log::warn!(
                    "signal sink full or closed; dropping signal for {}",
                    sig.symbol
                );
            }
        }
    }
}

async fn sync_clock(
    market: Arc<dyn MarketData>,
    clock: Arc<dyn Clock>,
    limiter: Arc<RateLimiter>,
    retry: RetryPolicy,
) {
    limiter.acquire().await;
    let fetched = with_retry(retry, move || {
        let m = market.clone();
        async move {
            tokio::task::spawn_blocking(move || m.exchange_time_ms())
                .await
                .map_err(|e| MarketError::Fatal(format!("exchange-time join error: {e}")))?
        }
    })
    .await;
    match fetched {
        Ok(t) => clock.sync(t),
        Err(e) => log::warn!("exchange-time sync failed: {e}"),
    }
}

/// The next candle open time to wait for: the candle after the last processed one, or the next
/// `interval_ms`-aligned grid boundary when starting fresh.
fn next_open_after(now_ms: i64, last_candle_ms: Option<i64>, interval_ms: i64) -> i64 {
    match last_candle_ms {
        Some(l) => l + interval_ms,
        None => now_ms.div_euclid(interval_ms) * interval_ms + interval_ms,
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
        let h4 = crate::strategy::config::interval_to_ms("4hour").unwrap();
        // Known last candle: the next open is exactly one timeframe later.
        assert_eq!(next_open_after(0, Some(0), h4), h4);
        // Unknown: the next grid boundary strictly after now.
        assert_eq!(next_open_after(1, None, h4), h4);
        assert_eq!(next_open_after(h4, None, h4), 2 * h4);
    }

    #[test]
    fn stop_requested_detects_quarantine() {
        assert!(!stop_requested(&[LiveEvent::Resume]));
        assert!(stop_requested(&[LiveEvent::Quarantine {
            reason: "x".into()
        }]));
    }
}
