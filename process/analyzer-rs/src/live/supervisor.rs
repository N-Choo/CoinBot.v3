use crate::live::config::LiveConfig;
use crate::live::grpc::EngineClient;
use crate::live::sink::Sink;
use crate::live::symbol_task::run_symbol;
use crate::strategy::config::Config;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

pub async fn run_supervisor(
    cfg: LiveConfig,
    strategy: Config,
    sink: Sink,
    client: EngineClient,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut tasks: BTreeMap<String, watch::Sender<bool>> = BTreeMap::new();
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
                    Ok(symbols) if !symbols.is_empty() => symbols.into_iter().map(|s| s.replace('/', "-")).collect(),
                    Ok(_) => fallback.clone(),
                    Err(error) => { eprintln!("activation poll failed: {error}"); continue; }
                };
                reconcile(&desired, &cfg, &strategy, &sink, &mut tasks, &mut set);
            }
            _ = set.join_next() => {}
        }
    }

    for stop in tasks.values() {
        let _ = stop.send(true);
    }
    while set.join_next().await.is_some() {}
    println!("supervisor stopped");
}

fn reconcile(
    desired: &BTreeSet<String>,
    cfg: &LiveConfig,
    strategy: &Config,
    sink: &Sink,
    tasks: &mut BTreeMap<String, watch::Sender<bool>>,
    set: &mut JoinSet<()>,
) {
    let newcomers: Vec<String> = desired
        .iter()
        .filter(|s| !tasks.contains_key(*s))
        .cloned()
        .collect();
    for symbol in newcomers {
        let (stop_tx, stop_rx) = watch::channel(false);
        let cfg = cfg.clone();
        let strategy = strategy.clone();
        let sink = sink.clone();
        let spawned = symbol.clone();
        set.spawn(async move {
            if let Err(error) = run_symbol(cfg, strategy, spawned.clone(), sink, stop_rx).await {
                eprintln!("symbol={spawned} stopped: {error}");
            }
        });
        tasks.insert(symbol.clone(), stop_tx);
        println!("symbol={symbol} activated");
    }
    let gone: Vec<String> = tasks
        .keys()
        .filter(|s| !desired.contains(*s))
        .cloned()
        .collect();
    for symbol in gone {
        if let Some(stop) = tasks.remove(&symbol) {
            let _ = stop.send(true);
        }
        println!("symbol={symbol} deactivated");
    }
}
