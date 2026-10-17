//! Phase 1 (shadow) live runner (live spec §3, §5a).
//!
//! Two modes:
//! - `live` — the real adapter: the shared `allTickers` poller and per-symbol loops over `ureq`.
//!   Logs only; no gRPC server and nothing published.
//! - `replay` — an offline smoke: a cached 4H CSV is expanded to synthetic ticks and driven through
//!   the same [`SymbolLoop`], so the loop can be exercised without the network.
//!
//! `harness` is required for the `replay` CSV loader/tick synthesis, hence the binary's
//! `required-features = ["harness", "live"]`.

use clap::{Parser, Subcommand};
use std::sync::Arc;
use strategy_analyzer::harness::dataset::load_csv;
use strategy_analyzer::harness::tick_source::{Ohlcv4hSynthesis, PathPolicy, TickSource};
use strategy_analyzer::live::{
    Clock, ExchangeClock, LiveConfig, LiveEvent, LiveInput, MarketData, StateStore, Supervisor,
    SymbolLoop, UreqMarketData,
};
use strategy_analyzer::strategy::config::Config;
use strategy_analyzer::strategy::STATE_SCHEMA_VERSION;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the live shadow over KuCoin (logs only; no gRPC).
    Live {
        /// Optional `LiveConfig` JSON file (else `LIVE_CONFIG` / `LIVE_*` env / defaults).
        #[arg(long)]
        config: Option<String>,
    },
    /// Offline smoke: replay a cached 4H CSV through the loop (no network).
    Replay {
        /// Cached OHLCV CSV (`open_time_ms,open,high,low,close`), like the backtest fixtures.
        #[arg(long)]
        data: String,
        /// The symbol the CSV represents.
        #[arg(long)]
        symbol: String,
        /// State directory (defaults to the crate's live default).
        #[arg(long)]
        state_dir: Option<String>,
        /// Recorded-input-stream directory (defaults to the crate's live default).
        #[arg(long)]
        stream_dir: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Res<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    match Cli::parse().cmd {
        Cmd::Live { config } => run_live(config).await,
        Cmd::Replay {
            data,
            symbol,
            state_dir,
            stream_dir,
        } => run_replay(&data, &symbol, state_dir, stream_dir),
    }
}

async fn run_live(config: Option<String>) -> Res<()> {
    let cfg = LiveConfig::load_with_file(config.as_deref())?;
    let market: Arc<dyn MarketData> = Arc::new(UreqMarketData::new(cfg.exchange_base_url.clone()));
    let clock: Arc<dyn Clock> = Arc::new(ExchangeClock::new());
    let supervisor = Supervisor::new(cfg, market, clock)?;
    log::info!("strategy-analyzer live shadow starting (Phase 1: log only, no gRPC)");
    supervisor
        .run(strategy_analyzer::live::shutdown_signal())
        .await?;
    log::info!("strategy-analyzer live shadow stopped");
    Ok(())
}

fn run_replay(
    data: &str,
    symbol: &str,
    state_dir: Option<String>,
    stream_dir: Option<String>,
) -> Res<()> {
    let candles = load_csv(data)?;
    let cfg = LiveConfig {
        symbols: vec![symbol.to_string()],
        state_dir: state_dir.unwrap_or_else(|| LiveConfig::default().state_dir),
        stream_dir: stream_dir.unwrap_or_else(|| LiveConfig::default().stream_dir),
        ..LiveConfig::default()
    };

    let store = Arc::new(StateStore::open(
        cfg.state_dir_path(),
        STATE_SCHEMA_VERSION,
    )?);
    let strategy_cfg = Config {
        candle_interval_ms: cfg.timeframe_ms(),
        ..Config::default_for(&[symbol])
    };
    let mut lp = SymbolLoop::new(
        symbol,
        strategy_cfg,
        store,
        &cfg.stream_dir_path(),
        cfg.price_staleness_ms,
        0,
    )?;

    // Drive the fixture exactly like the offline harness: four synthetic ticks, then the candle.
    let mut src = Ohlcv4hSynthesis::new(candles, PathPolicy::LowFirst);
    let mut signals = 0usize;
    while let Some((ticks, candle)) = src.next_candle() {
        for t in ticks {
            for ev in lp.process(
                LiveInput::Price {
                    price: t.price,
                    at_ms: t.time_ms,
                },
                t.time_ms + 1,
            )? {
                if matches!(ev, LiveEvent::Signal(_)) {
                    signals += 1;
                }
            }
        }
        let closed_now = candle.open_time_ms + cfg.timeframe_ms() + cfg.candle_settle_ms + 1;
        for ev in lp.process(LiveInput::Candle(candle), closed_now)? {
            if matches!(ev, LiveEvent::Signal(_)) {
                signals += 1;
            }
        }
    }
    println!(
        "# offline replay smoke: symbol={symbol} signals={signals} stream={}",
        lp.stream_path().display()
    );
    Ok(())
}
