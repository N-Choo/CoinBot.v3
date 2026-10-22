use crate::live::config::LiveConfig;
use crate::live::grpc::EngineClient;
use crate::live::sink::Sink;
use crate::live::symbol_task::run_symbol;
use crate::strategy::config::Config;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::task::{Id, JoinError, JoinSet};

const REPLACEMENT_BACKOFF: Duration = Duration::from_secs(30);
const WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run_supervisor(
    cfg: LiveConfig,
    strategy: Config,
    sink: Sink,
    client: EngineClient,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut stops: BTreeMap<String, watch::Sender<bool>> = BTreeMap::new();
    let mut stopping: BTreeSet<String> = BTreeSet::new();
    let mut sym_of: BTreeMap<Id, String> = BTreeMap::new();
    let mut failed_until: BTreeMap<String, Instant> = BTreeMap::new();
    let mut set: JoinSet<()> = JoinSet::new();
    let mut activation = tokio::time::interval(Duration::from_millis(cfg.activation_poll_ms));
    let fallback: BTreeSet<String> = cfg.symbols.iter().cloned().collect();

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { break; }
            }
            _ = activation.tick() => {
                let mut client = client.clone();
                let desired: BTreeSet<String> = match client.active_symbols().await {
                    Ok(symbols) if !symbols.is_empty() => normalize_symbols(symbols),
                    Ok(_) => fallback.clone(),
                    Err(error) => {
                        log::warn!("activation poll failed: {error}");
                        if set.is_empty() && stops.is_empty() {
                            fallback.clone()
                        } else {
                            log::warn!("keeping the current symbol set");
                            continue;
                        }
                    }
                };
                reconcile(
                    &desired,
                    &cfg,
                    &strategy,
                    &sink,
                    &mut set,
                    &mut stops,
                    &mut stopping,
                    &mut sym_of,
                    &mut failed_until,
                );
            }
            joined = set.join_next_with_id(), if !set.is_empty() => {
                reap(joined, &mut stops, &mut stopping, &mut sym_of, &mut failed_until, Instant::now())
            }
        }
    }

    for stop in stops.values() {
        let _ = stop.send(true);
    }
    shutdown_workers(&mut set).await;
    log::info!("supervisor stopped");
}

fn normalize_symbols(symbols: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    symbols
        .into_iter()
        .map(|symbol| symbol.trim().replace('/', "-"))
        .filter(|symbol| !symbol.is_empty())
        .collect()
}

/// Add tasks for newcomers and ask removed symbols to stop.
///
/// A stopped symbol stays in `stops` until its task is reaped, which prevents a duplicate task
/// from being spawned while the old one is still winding down.
#[allow(clippy::too_many_arguments)]
fn reconcile(
    desired: &BTreeSet<String>,
    cfg: &LiveConfig,
    strategy: &Config,
    sink: &Sink,
    set: &mut JoinSet<()>,
    stops: &mut BTreeMap<String, watch::Sender<bool>>,
    stopping: &mut BTreeSet<String>,
    sym_of: &mut BTreeMap<Id, String>,
    failed_until: &mut BTreeMap<String, Instant>,
) {
    let now = Instant::now();
    let (newcomers, gone) = reconcile_plan(desired, stops, stopping, failed_until, now);

    for symbol in newcomers {
        let (stop_tx, stop_rx) = watch::channel(false);
        let cfg = cfg.clone();
        let strategy = strategy.clone();
        let sink = sink.clone();
        let spawned = symbol.clone();
        let id = set
            .spawn(async move {
                if let Err(error) = run_symbol(cfg, strategy, spawned.clone(), sink, stop_rx).await
                {
                    log::error!("symbol={spawned} stopped: {error}");
                }
            })
            .id();
        sym_of.insert(id, symbol.clone());
        stops.insert(symbol.clone(), stop_tx);
        log::info!("symbol={symbol} activated");
    }

    for symbol in gone {
        if let Some(stop) = stops.get(&symbol) {
            let _ = stop.send(true);
        }
        stopping.insert(symbol.clone());
        log::info!("symbol={symbol} stopping");
    }
}

/// Decide which symbols to start and stop without touching tasks, so the policy is testable.
fn reconcile_plan(
    desired: &BTreeSet<String>,
    stops: &BTreeMap<String, watch::Sender<bool>>,
    stopping: &BTreeSet<String>,
    failed_until: &BTreeMap<String, Instant>,
    now: Instant,
) -> (Vec<String>, Vec<String>) {
    let newcomers = desired
        .iter()
        .filter(|symbol| !stops.contains_key(*symbol))
        .filter(|symbol| !stopping.contains(*symbol))
        .filter(|symbol| failed_until.get(*symbol).is_none_or(|until| *until <= now))
        .cloned()
        .collect();
    let gone = stops
        .keys()
        .filter(|symbol| !desired.contains(*symbol))
        .filter(|symbol| !stopping.contains(*symbol))
        .cloned()
        .collect();
    (newcomers, gone)
}

/// Forget a finished or panicked task and back off before trying that symbol again.
fn reap(
    joined: Option<Result<(Id, ()), JoinError>>,
    stops: &mut BTreeMap<String, watch::Sender<bool>>,
    stopping: &mut BTreeSet<String>,
    sym_of: &mut BTreeMap<Id, String>,
    failed_until: &mut BTreeMap<String, Instant>,
    now: Instant,
) {
    let Some(joined) = joined else {
        return;
    };
    let (id, failure) = match joined {
        Ok((id, ())) => (id, None),
        Err(error) => (error.id(), Some(error)),
    };
    let Some(symbol) = sym_of.remove(&id) else {
        log::warn!("task {id:?} ended without a symbol mapping");
        return;
    };
    stops.remove(&symbol);
    stopping.remove(&symbol);
    failed_until.insert(symbol.clone(), now + REPLACEMENT_BACKOFF);
    match failure {
        Some(error) => log::error!(
            "symbol={symbol} task ended abnormally: {error}; retry in {}s",
            REPLACEMENT_BACKOFF.as_secs()
        ),
        None => log::warn!(
            "symbol={symbol} task ended; retry in {}s",
            REPLACEMENT_BACKOFF.as_secs()
        ),
    }
}

async fn shutdown_workers(set: &mut JoinSet<()>) {
    let drained = tokio::time::timeout(WORKER_SHUTDOWN_TIMEOUT, async {
        while set.join_next().await.is_some() {}
    })
    .await;
    if drained.is_err() {
        log::warn!("worker shutdown timed out; aborting stragglers");
        set.abort_all();
        while set.join_next().await.is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop_map(symbols: &[&str]) -> BTreeMap<String, watch::Sender<bool>> {
        symbols
            .iter()
            .map(|symbol| ((*symbol).to_string(), watch::channel(false).0))
            .collect()
    }

    #[test]
    fn reconcile_plan_skips_stopping_and_backoff_symbols() {
        let now = Instant::now();
        let desired: BTreeSet<String> = ["BTC-USDT", "ETH-USDT", "SOL-USDT"]
            .iter()
            .map(|symbol| symbol.to_string())
            .collect();
        let stops = stop_map(&["BTC-USDT"]);
        let stopping = BTreeSet::from(["BTC-USDT".to_string()]);
        let failed_until = BTreeMap::from([("ETH-USDT".to_string(), now + REPLACEMENT_BACKOFF)]);

        let (newcomers, gone) = reconcile_plan(&desired, &stops, &stopping, &failed_until, now);

        assert_eq!(newcomers, vec!["SOL-USDT".to_string()]);
        assert!(gone.is_empty());
    }

    #[test]
    fn reconcile_plan_does_not_stop_a_symbol_twice() {
        let now = Instant::now();
        let desired = BTreeSet::new();
        let stops = stop_map(&["BTC-USDT"]);

        let (_, gone) = reconcile_plan(&desired, &stops, &BTreeSet::new(), &BTreeMap::new(), now);
        assert_eq!(gone, vec!["BTC-USDT".to_string()]);

        let stopping = BTreeSet::from(["BTC-USDT".to_string()]);
        let (_, gone) = reconcile_plan(&desired, &stops, &stopping, &BTreeMap::new(), now);
        assert!(gone.is_empty());
    }

    #[test]
    fn reconcile_plan_allows_expired_backoff() {
        let now = Instant::now();
        let desired = BTreeSet::from(["ETH-USDT".to_string()]);
        let failed_until = BTreeMap::from([("ETH-USDT".to_string(), now + REPLACEMENT_BACKOFF)]);

        let (newcomers, _) = reconcile_plan(
            &desired,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &failed_until,
            now,
        );
        assert!(newcomers.is_empty());

        let (newcomers, _) = reconcile_plan(
            &desired,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &failed_until,
            now + REPLACEMENT_BACKOFF,
        );
        assert_eq!(newcomers, vec!["ETH-USDT".to_string()]);
    }

    #[tokio::test]
    async fn reap_clears_state_and_sets_backoff() {
        let mut set = JoinSet::new();
        let id = set.spawn(async {}).id();
        let joined = set.join_next_with_id().await;

        let mut stops = stop_map(&["BTC-USDT"]);
        let mut stopping = BTreeSet::from(["BTC-USDT".to_string()]);
        let mut sym_of = BTreeMap::from([(id, "BTC-USDT".to_string())]);
        let mut failed_until = BTreeMap::new();
        let now = Instant::now();

        reap(
            joined,
            &mut stops,
            &mut stopping,
            &mut sym_of,
            &mut failed_until,
            now,
        );

        assert!(stops.is_empty());
        assert!(stopping.is_empty());
        assert!(sym_of.is_empty());
        assert_eq!(
            failed_until.get("BTC-USDT"),
            Some(&(now + REPLACEMENT_BACKOFF))
        );
    }

    #[tokio::test]
    async fn reap_handles_a_panicked_task() {
        let mut set = JoinSet::new();
        let id = set.spawn(async { panic!("boom") }).id();
        let joined = set.join_next_with_id().await;

        let mut stops = stop_map(&["BTC-USDT"]);
        let mut stopping = BTreeSet::from(["BTC-USDT".to_string()]);
        let mut sym_of = BTreeMap::from([(id, "BTC-USDT".to_string())]);
        let mut failed_until = BTreeMap::new();
        let now = Instant::now();

        reap(
            joined,
            &mut stops,
            &mut stopping,
            &mut sym_of,
            &mut failed_until,
            now,
        );

        assert!(stops.is_empty());
        assert!(stopping.is_empty());
        assert!(sym_of.is_empty());
        assert!(failed_until.contains_key("BTC-USDT"));
    }
}
