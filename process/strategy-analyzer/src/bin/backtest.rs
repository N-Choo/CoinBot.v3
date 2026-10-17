use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    /// Fetch OHLCV and cache to a file. Never used by `backtest`.
    FetchData {
        #[arg(long)] symbol: String,
        #[arg(long, default_value = "4hour")] interval: String,
        #[arg(long)] start_s: i64,
        #[arg(long)] end_s: i64,
        #[arg(long)] out: String,
    },
    /// Run the backtest offline against a cached dataset.
    Backtest {
        #[arg(long)] data: String,
        #[arg(long, default_value = "both")] policy: String,
        #[arg(long, default_value = "both")] mode: String,
        #[arg(long)] lock: Option<String>,
        #[arg(long, default_value_t = false)] oos: bool,
        #[arg(long, default_value_t = false)] oos_consumed: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::FetchData { .. } => eprintln!("fetch-data: network call added in Task 24; paging planner is tested"),
        Cmd::Backtest { data, .. } => {
            let candles = strategy_analyzer::harness::dataset::load_csv(&data)?;
            println!("loaded {} candles from {}", candles.len(), data);
        }
    }
    Ok(())
}
