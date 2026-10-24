use crate::harness::sweep::data::DatasetProvenance;
use crate::harness::ts_momentum_backtest::{
    EquityPoint, Metrics, Mode, Split, SplitName, TradeEpisode, Variant,
};
use crate::strategy::ts_momentum::TsMomentumConfig;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const STRATEGY_NOTE: &str =
    "BTC time-series momentum. Signals use only closed daily candles and execute at the next open.";
fn limitations(config: &TsMomentumConfig) -> Vec<String> {
    let required = config
        .momentum_lookback_days
        .max(config.volatility_lookback_days)
        .saturating_add(1);
    vec![
        "Spot shorting is not supported by the current trade engine. Short-mode results are a backtest-only simulation.".to_string(),
        format!("Signals require {required} closed daily candles (max of momentum and volatility lookback + 1); earlier days are flat."),
        "The 200-day BTC regime label needs 200 closes and is reported as unknown before that.".to_string(),
        "Splits are chronological reporting partitions. No parameter tuning or model selection is performed on them.".to_string(),
        "Fees and slippage are modelled as a single per-side rate on traded notional; market impact is not modelled.".to_string(),
        "Open episodes at the end of a split are marked at the final close and counted as closed for win-rate reporting without an extra liquidation cost.".to_string(),
        "The drawdown variant evaluates the 20%/10% hysteresis on the uncapped variant-B reference equity path so the control can recover while the live portfolio is flat.".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetInfo {
    pub symbol: String,
    pub timeframe: String,
    pub requested_first_ms: i64,
    pub requested_last_ms: i64,
    pub first_open_time_ms: i64,
    pub last_open_time_ms: i64,
    pub candles: usize,
    pub requested_candles: usize,
    pub expected_candles: usize,
    pub missing_candles: usize,
    pub duplicates_dropped: usize,
    pub pages: usize,
    pub fetched_at_ms: i64,
    pub settle_ms: i64,
    pub content_sha256: String,
}

impl DatasetInfo {
    pub fn from_provenance(provenance: &DatasetProvenance) -> Self {
        Self {
            symbol: provenance.symbol.clone(),
            timeframe: provenance.timeframe.clone(),
            requested_first_ms: provenance.requested_first_ms,
            requested_last_ms: provenance.requested_last_ms,
            first_open_time_ms: provenance.first_open_time_ms,
            last_open_time_ms: provenance.last_open_time_ms,
            candles: provenance.candles,
            requested_candles: provenance.requested_candles,
            expected_candles: provenance.expected_candles,
            missing_candles: provenance.missing_candles,
            duplicates_dropped: provenance.duplicates_dropped,
            pages: provenance.pages,
            fetched_at_ms: provenance.fetched_at_ms,
            settle_ms: provenance.settle_ms,
            content_sha256: provenance.content_sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitBoundary {
    pub name: SplitName,
    pub start_index: usize,
    pub end_index: usize,
    pub candles: usize,
    pub start_time: String,
    pub end_time: String,
}

impl SplitBoundary {
    pub fn from_split(candles: &[Candle], split: Split) -> Self {
        Self {
            name: split.name,
            start_index: split.start,
            end_index: split.end,
            candles: split.len(),
            start_time: date_at(candles.get(split.start).map(|candle| candle.open_time_ms)),
            end_time: date_at(
                candles
                    .get(split.end.saturating_sub(1))
                    .map(|candle| candle.open_time_ms),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReportContext {
    pub symbol: String,
    pub timeframe: String,
    pub as_of_ms: i64,
    pub generated_at: String,
    pub days: u32,
    pub init_fund: Decimal,
    pub config: TsMomentumConfig,
    pub borrow_bps_per_day: Decimal,
    pub max_drawdown: Decimal,
    pub drawdown_resume: Decimal,
    pub dataset: DatasetInfo,
    pub splits: Vec<SplitBoundary>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioInfo {
    pub label: String,
    pub fee_bps: Decimal,
    pub slip_bps: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub scenario: ScenarioInfo,
    pub variant: Variant,
    pub mode: Mode,
    pub split: Split,
    pub split_start_time: String,
    pub split_end_time: String,
    pub candles: usize,
    pub init_fund: Decimal,
    pub borrow_bps_per_day: Decimal,
    pub max_drawdown: Decimal,
    pub drawdown_resume: Decimal,
    pub metrics: Metrics,
    pub benchmark: Metrics,
    pub trades: Vec<TradeEpisode>,
    pub trading_costs: Decimal,
    pub borrow_costs: Decimal,
    pub traded_notional: Decimal,
    pub benchmark_trading_costs: Decimal,
    pub benchmark_traded_notional: Decimal,
    #[serde(skip)]
    pub points: Vec<EquityPoint>,
}

impl RunReport {
    pub fn directory(&self) -> PathBuf {
        PathBuf::from(&self.scenario.label)
            .join(format!("{}-{}", self.variant.slug(), self.mode.slug()))
            .join(self.split.name.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioRuns {
    pub info: ScenarioInfo,
    pub runs: Vec<RunReport>,
}

#[derive(Debug, Clone, Serialize)]
struct RunDocument {
    symbol: String,
    timeframe: String,
    generated_at: String,
    dataset: DatasetInfo,
    config: TsMomentumConfig,
    run: RunReport,
    assumptions: Vec<&'static str>,
    limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ResultsDocument {
    symbol: String,
    timeframe: String,
    generated_at: String,
    as_of_ms: i64,
    days: u32,
    init_fund: Decimal,
    config: TsMomentumConfig,
    borrow_bps_per_day: Decimal,
    max_drawdown: Decimal,
    drawdown_resume: Decimal,
    dataset: DatasetInfo,
    splits: Vec<SplitBoundary>,
    scenarios: Vec<ScenarioInfo>,
    runs: Vec<RunReport>,
    errors: Vec<String>,
    assumptions: Vec<&'static str>,
    limitations: Vec<String>,
}

/// Writes the combined report plus one `report.md`, `results.json`, and
/// `equity.csv` per scenario / variant / mode / split.
pub fn write_reports(
    out_dir: &Path,
    context: &ReportContext,
    scenarios: &[ScenarioRuns],
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;
    write_top_report(&out_dir.join("report.md"), context, scenarios)?;
    write_top_json(&out_dir.join("results.json"), context, scenarios)?;

    for scenario in scenarios {
        for run in &scenario.runs {
            let dir = out_dir.join(run.directory());
            std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
            write_equity_csv(&dir.join("equity.csv"), &run.points)?;
            write_run_report(&dir.join("report.md"), context, run)?;
            write_run_json(&dir.join("results.json"), context, run)?;
        }
    }
    Ok(())
}

fn write_equity_csv(path: &Path, points: &[EquityPoint]) -> Result<(), String> {
    let mut out = String::from("date,equity,exposure,regime\n");
    for point in points {
        writeln!(
            out,
            "{},{},{},{}",
            date_at(Some(point.open_time_ms)),
            csv_decimal(point.equity),
            csv_decimal(point.exposure),
            regime_label(point.regime)
        )
        .expect("writing to String cannot fail");
    }
    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn write_top_report(
    path: &Path,
    context: &ReportContext,
    scenarios: &[ScenarioRuns],
) -> Result<(), String> {
    let mut out = String::new();
    writeln!(out, "# BTC 1-day time-series momentum backtest\n")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "> Generated at {}. This is a cost-aware historical simulation, not evidence of future profitability. The test split was not used for tuning.\n",
        context.generated_at
    )
    .expect("writing to String cannot fail");

    write_dataset_section(&mut out, context);
    let scenario_infos: Vec<ScenarioInfo> = scenarios
        .iter()
        .map(|scenario| scenario.info.clone())
        .collect();
    write_config_section(&mut out, context, &scenario_infos);
    write_splits_section(&mut out, context);
    for scenario in scenarios {
        write_scenario_section(&mut out, scenario);
    }
    write_regime_section(&mut out, scenarios);
    write_assumptions_section(&mut out, context);
    write_limitations_section(&mut out, &context.config);
    write_errors_section(&mut out, context);

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn write_run_report(path: &Path, context: &ReportContext, run: &RunReport) -> Result<(), String> {
    let mut out = String::new();
    writeln!(
        out,
        "# BTC time-series momentum — {} / {} / {} / {}\n",
        run.variant.as_str(),
        run.mode.as_str(),
        run.split.name.as_str(),
        run.scenario.label
    )
    .expect("writing to String cannot fail");
    writeln!(out, "> {STRATEGY_NOTE}\n").expect("writing to String cannot fail");

    write_dataset_section(&mut out, context);
    write_config_section(&mut out, context, std::slice::from_ref(&run.scenario));
    write_run_split_section(&mut out, run);
    write_run_results_section(&mut out, run);
    write_run_regime_section(&mut out, run);
    write_run_trades_section(&mut out, run);
    write_assumptions_section(&mut out, context);
    write_limitations_section(&mut out, &context.config);
    write_errors_section(&mut out, context);

    std::fs::write(path, out).map_err(|error| error.to_string())
}

fn write_top_json(
    path: &Path,
    context: &ReportContext,
    scenarios: &[ScenarioRuns],
) -> Result<(), String> {
    let runs: Vec<RunReport> = scenarios
        .iter()
        .flat_map(|scenario| scenario.runs.iter().cloned())
        .collect();
    let document = ResultsDocument {
        symbol: context.symbol.clone(),
        timeframe: context.timeframe.clone(),
        generated_at: context.generated_at.clone(),
        as_of_ms: context.as_of_ms,
        days: context.days,
        init_fund: context.init_fund,
        config: context.config.clone(),
        borrow_bps_per_day: context.borrow_bps_per_day,
        max_drawdown: context.max_drawdown,
        drawdown_resume: context.drawdown_resume,
        dataset: context.dataset.clone(),
        splits: context.splits.clone(),
        scenarios: scenarios
            .iter()
            .map(|scenario| scenario.info.clone())
            .collect(),
        runs,
        errors: context.errors.clone(),
        assumptions: execution_assumptions(),
        limitations: limitations(&context.config),
    };
    let json = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn write_run_json(path: &Path, context: &ReportContext, run: &RunReport) -> Result<(), String> {
    let document = RunDocument {
        symbol: context.symbol.clone(),
        timeframe: context.timeframe.clone(),
        generated_at: context.generated_at.clone(),
        dataset: context.dataset.clone(),
        config: context.config.clone(),
        run: run.clone(),
        assumptions: execution_assumptions(),
        limitations: limitations(&context.config),
    };
    let json = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn write_dataset_section(out: &mut String, context: &ReportContext) {
    let dataset = &context.dataset;
    writeln!(out, "## Dataset\n").expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", context.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", context.timeframe).expect("writing to String cannot fail");
    writeln!(out, "| As of | {} |", datetime_at(context.as_of_ms))
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Requested range | {} .. {} |",
        date_at(Some(dataset.requested_first_ms)),
        date_at(Some(dataset.requested_last_ms))
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Effective range | {} .. {} |",
        date_at(Some(dataset.first_open_time_ms)),
        date_at(Some(dataset.last_open_time_ms))
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Closed candles | {} |", dataset.candles)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Requested / expected candles | {} / {} |",
        dataset.requested_candles, dataset.expected_candles
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Missing candles | {} |", dataset.missing_candles)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Duplicates dropped | {} |",
        dataset.duplicates_dropped
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Pages fetched | {} |", dataset.pages).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Fetched at | {} |",
        datetime_at(dataset.fetched_at_ms)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Settle | {} ms |", dataset.settle_ms).expect("writing to String cannot fail");
    writeln!(out, "| SHA-256 | `{}` |", dataset.content_sha256)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_config_section(out: &mut String, context: &ReportContext, scenarios: &[ScenarioInfo]) {
    let config = &context.config;
    writeln!(out, "## Strategy and configuration\n").expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Strategy | {} |",
        crate::strategy::ts_momentum::STRATEGY_ID
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Momentum lookback | {} days |",
        config.momentum_lookback_days
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Volatility lookback | {} days |",
        config.volatility_lookback_days
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Target annualized volatility | {} |",
        decimal_plain(config.target_annualized_volatility)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Annualization days | {} |",
        config.annualization_days
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Exposure bounds | [{}, {}] |",
        decimal_plain(config.min_exposure),
        decimal_plain(config.max_exposure)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Negative momentum | Flat and Short run separately |")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Initial fund | {} |",
        decimal_plain(context.init_fund)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Borrow cost | {} bps/day on short exposure |",
        decimal_plain(context.borrow_bps_per_day)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Drawdown control | halt at {}%, resume below {}% |",
        decimal_plain(context.max_drawdown * Decimal::from(100)),
        decimal_plain(context.drawdown_resume * Decimal::from(100))
    )
    .expect("writing to String cannot fail");
    for scenario in scenarios {
        writeln!(
            out,
            "| Costs ({}) | {} bps fee + {} bps slippage per side |",
            scenario.label,
            decimal_plain(scenario.fee_bps),
            decimal_plain(scenario.slip_bps)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "Variants:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- **A. Fixed exposure** — long 1.0 when momentum is positive, flat otherwise; short -1.0 in Short mode."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- **B. Volatility targeting** — exposure is `clamp(target_vol / realized_vol, min, max)` with the momentum direction."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- **C. Volatility targeting + drawdown control** — B plus a configurable 20%/10% drawdown hysteresis overlay. The hysteresis is evaluated on the uncapped variant-B reference equity path so it can recover while the live portfolio is flat."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_splits_section(out: &mut String, context: &ReportContext) {
    writeln!(out, "## Chronological splits\n").expect("writing to String cannot fail");
    writeln!(
        out,
        "Splits are 60% training, 20% validation, and 20% test by candle count. Each split is simulated independently with its own initial fund and uses earlier candles only for signal warm-up. No parameters are tuned.\n"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Split | Candles | Start index | End index | First date | Last date |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---|---|").expect("writing to String cannot fail");
    for split in &context.splits {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            split.name.as_str(),
            split.candles,
            split.start_index,
            split.end_index,
            split.start_time,
            split.end_time
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_scenario_section(out: &mut String, scenario: &ScenarioRuns) {
    writeln!(
        out,
        "## {} costs ({} bps fee + {} bps slippage)\n",
        scenario.info.label, scenario.info.fee_bps, scenario.info.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Variant | Mode | Split | Net return | CAGR | Max DD | Sharpe | Sortino | Win rate | Trades | Avg hold (d) | Turnover | Trading costs | Borrow costs | Avg abs exposure |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    .expect("writing to String cannot fail");
    for run in &scenario.runs {
        let metrics = &run.metrics;
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            run.variant.as_str(),
            run.mode.as_str(),
            run.split.name.as_str(),
            percent(metrics.net_cumulative_return),
            percent(metrics.cagr),
            percent(metrics.max_drawdown),
            ratio(metrics.sharpe),
            ratio(metrics.sortino),
            percent(metrics.win_rate),
            metrics.trades,
            ratio(metrics.avg_holding_days),
            ratio(Some(metrics.turnover)),
            money(metrics.total_trading_costs),
            money(metrics.total_borrow_costs),
            percent(metrics.avg_abs_exposure)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Benchmark: BTC buy-and-hold for the same split, with one entry at the first open, one exit at the last close, and the same per-side rate.\n"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Split | Net return | CAGR | Max DD | Sharpe | Sortino | Trades | Trading costs | Turnover |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for run in scenario
        .runs
        .iter()
        .filter(|run| run.variant == Variant::Fixed && run.mode == Mode::Flat)
    {
        let metrics = &run.benchmark;
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            run.split.name.as_str(),
            percent(metrics.net_cumulative_return),
            percent(metrics.cagr),
            percent(metrics.max_drawdown),
            ratio(metrics.sharpe),
            ratio(metrics.sortino),
            metrics.trades,
            money(metrics.total_trading_costs),
            ratio(Some(metrics.turnover))
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_run_split_section(out: &mut String, run: &RunReport) {
    writeln!(out, "## Split\n").expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Split | {} |", run.split.name.as_str())
        .expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", run.candles).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Range | {} .. {} |",
        run.split_start_time, run.split_end_time
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Variant | {} |", run.variant.as_str()).expect("writing to String cannot fail");
    writeln!(out, "| Mode | {} |", run.mode.as_str()).expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_run_results_section(out: &mut String, run: &RunReport) {
    writeln!(out, "## Results\n").expect("writing to String cannot fail");
    writeln!(out, "| Metric | Strategy | BTC buy-and-hold |")
        .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|").expect("writing to String cannot fail");
    write_metric_row(
        out,
        "Net cumulative return",
        percent(run.metrics.net_cumulative_return),
        percent(run.benchmark.net_cumulative_return),
    );
    write_metric_row(
        out,
        "CAGR",
        percent(run.metrics.cagr),
        percent(run.benchmark.cagr),
    );
    write_metric_row(
        out,
        "Max drawdown",
        percent(run.metrics.max_drawdown),
        percent(run.benchmark.max_drawdown),
    );
    write_metric_row(
        out,
        "Sharpe",
        ratio(run.metrics.sharpe),
        ratio(run.benchmark.sharpe),
    );
    write_metric_row(
        out,
        "Sortino",
        ratio(run.metrics.sortino),
        ratio(run.benchmark.sortino),
    );
    write_metric_row(
        out,
        "Win rate",
        percent(run.metrics.win_rate),
        percent(run.benchmark.win_rate),
    );
    write_metric_row(
        out,
        "Trades",
        run.metrics.trades.to_string(),
        run.benchmark.trades.to_string(),
    );
    write_metric_row(
        out,
        "Average holding days",
        ratio(run.metrics.avg_holding_days),
        ratio(run.benchmark.avg_holding_days),
    );
    write_metric_row(
        out,
        "Turnover (initial equity)",
        ratio(Some(run.metrics.turnover)),
        ratio(Some(run.benchmark.turnover)),
    );
    write_metric_row(
        out,
        "Trading costs",
        money(run.metrics.total_trading_costs),
        money(run.benchmark.total_trading_costs),
    );
    write_metric_row(
        out,
        "Borrow costs",
        money(run.metrics.total_borrow_costs),
        "—".to_string(),
    );
    write_metric_row(
        out,
        "Average exposure",
        percent(run.metrics.avg_exposure),
        "100.00%".to_string(),
    );
    write_metric_row(
        out,
        "Average absolute exposure",
        percent(run.metrics.avg_abs_exposure),
        "100.00%".to_string(),
    );
    write_metric_row(
        out,
        "Days with exposure",
        run.metrics.exposure_days.to_string(),
        run.candles.to_string(),
    );
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Equity, exposure, and regime for every day are in `equity.csv`.\n"
    )
    .expect("writing to String cannot fail");
}

fn write_metric_row(out: &mut String, label: &str, strategy: String, benchmark: String) {
    writeln!(out, "| {label} | {strategy} | {benchmark} |").expect("writing to String cannot fail");
}

fn write_run_regime_section(out: &mut String, run: &RunReport) {
    writeln!(out, "## Regime results\n").expect("writing to String cannot fail");
    writeln!(
        out,
        "Regime is `BTC close > 200-day simple moving average`. Early days without a 200-day history are unknown. Regime returns compound only the daily returns observed while that regime was active, so they are a contribution, not a calendar-period CAGR.\n"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Regime | Days | Net return | Sharpe | Sortino | Max DD | Avg exposure | Avg abs exposure |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    write_regime_rows(out, "Bull", &run.metrics.bull);
    write_regime_rows(out, "Bear", &run.metrics.bear);
    write_regime_rows(out, "Unknown", &run.metrics.unknown_regime);
    writeln!(out).expect("writing to String cannot fail");
}

fn write_regime_rows(
    out: &mut String,
    label: &str,
    metrics: &crate::harness::ts_momentum_backtest::RegimeMetrics,
) {
    writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} | {} |",
        label,
        metrics.days,
        percent(metrics.cumulative_return),
        ratio(metrics.sharpe),
        ratio(metrics.sortino),
        percent(metrics.max_drawdown),
        percent(metrics.avg_exposure),
        percent(metrics.avg_abs_exposure)
    )
    .expect("writing to String cannot fail");
}

fn write_run_trades_section(out: &mut String, run: &RunReport) {
    writeln!(out, "## Trades\n").expect("writing to String cannot fail");
    if run.trades.is_empty() {
        writeln!(out, "No non-zero exposure episodes in this split.\n")
            .expect("writing to String cannot fail");
        return;
    }
    writeln!(
        out,
        "A trade is a contiguous non-zero signed exposure episode. A negative PnL includes the costs paid inside the episode.\n"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Side | Entry | Exit | Holding days | PnL | Closed at end |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---:|---:|---|").expect("writing to String cannot fail");
    for trade in &run.trades {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            trade.side.as_str(),
            date_at(Some(trade.entry_time_ms)),
            date_at(Some(trade.exit_time_ms)),
            trade.holding_days,
            money(trade.pnl),
            if trade.closed_at_end { "yes" } else { "no" }
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_regime_section(out: &mut String, scenarios: &[ScenarioRuns]) {
    writeln!(out, "## Regime summary (full period)\n").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Scenario | Variant | Mode | Regime | Days | Net return | Sharpe | Max DD | Avg abs exposure |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for scenario in scenarios {
        for run in scenario
            .runs
            .iter()
            .filter(|run| run.split.name == SplitName::Full)
        {
            for (label, regime) in [
                ("Bull", &run.metrics.bull),
                ("Bear", &run.metrics.bear),
                ("Unknown", &run.metrics.unknown_regime),
            ] {
                writeln!(
                    out,
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    scenario.info.label,
                    run.variant.as_str(),
                    run.mode.as_str(),
                    label,
                    regime.days,
                    percent(regime.cumulative_return),
                    ratio(regime.sharpe),
                    percent(regime.max_drawdown),
                    percent(regime.avg_abs_exposure)
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_assumptions_section(out: &mut String, context: &ReportContext) {
    writeln!(out, "## Costs and execution assumptions\n").expect("writing to String cannot fail");
    writeln!(out, "- Signals are evaluated at the close of a fully closed candle and executed at the next candle open.").expect("writing to String cannot fail");
    writeln!(
        out,
        "- Overnight gap from the previous close to the next open is applied at the old exposure."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- Costs are `(fee_bps + slip_bps) / 10_000 × |target exposure - old exposure| × equity` at execution.").expect("writing to String cannot fail");
    writeln!(out, "- After execution the new exposure earns the intraday open-to-close return. No leverage above the configured maximum of 1.0 is used.").expect("writing to String cannot fail");
    writeln!(out, "- Borrow cost is `borrow_bps_per_day / 10_000 × |short exposure| × equity` each day; it defaults to zero.").expect("writing to String cannot fail");
    writeln!(out, "- Benchmark uses full equity, one entry at the split's first open, one exit at the split's last close, and the same per-side rate.").expect("writing to String cannot fail");
    writeln!(
        out,
        "- Turnover is total traded notional divided by the split's initial fund. Results are in {}, not percentage points of price.\n",
        context.symbol
    )
    .expect("writing to String cannot fail");
}

fn write_limitations_section(out: &mut String, config: &TsMomentumConfig) {
    writeln!(out, "## Limitations\n").expect("writing to String cannot fail");
    for limitation in limitations(config) {
        writeln!(out, "- {limitation}").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_errors_section(out: &mut String, context: &ReportContext) {
    writeln!(out, "## Data issues\n").expect("writing to String cannot fail");
    if context.errors.is_empty() {
        writeln!(out, "No data issues were recorded.\n").expect("writing to String cannot fail");
        return;
    }
    for error in &context.errors {
        writeln!(out, "- {error}").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn execution_assumptions() -> Vec<&'static str> {
    vec![
        "Signal at a closed candle close; execute at the next open.",
        "Overnight gap applied at the old exposure, then costs on |target - old| x equity.",
        "New exposure earns the intraday open-to-close return; maximum exposure is 1.0.",
        "Borrow cost is charged daily on |short exposure| x equity (default zero).",
        "Benchmark is full-equity BTC buy-and-hold with one entry and one exit at the same per-side cost.",
    ]
}

fn date_at(ms: Option<i64>) -> String {
    let Some(ms) = ms else {
        return "—".to_string();
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| ms.to_string())
}

fn datetime_at(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn regime_label(regime: Option<bool>) -> &'static str {
    match regime {
        Some(true) => "bull",
        Some(false) => "bear",
        None => "unknown",
    }
}

fn decimal_plain(value: Decimal) -> String {
    value.normalize().to_string()
}

fn csv_decimal(value: Decimal) -> String {
    value.round_dp(8).normalize().to_string()
}

fn percent(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(|| "—".to_string())
}

fn ratio(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

fn money(value: Decimal) -> String {
    value.round_dp(2).normalize().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::sweep::costs::CostScenario;
    use crate::harness::ts_momentum_backtest::{
        benchmark, run, splits, Mode, Split, TsBacktestParams, Variant,
    };
    use crate::strategy::types::Candle;
    use rust_decimal::prelude::FromPrimitive;

    fn candles(count: usize) -> Vec<Candle> {
        let mut price = 100.0_f64;
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                price *= 1.0 + if index % 2 == 0 { 0.002 } else { 0.001 };
            }
            let close = Decimal::from_f64(price).expect("finite price");
            let open = out
                .last()
                .map(|candle: &Candle| candle.close)
                .unwrap_or(close);
            out.push(Candle {
                open_time_ms: index as i64 * 86_400_000,
                open,
                high: open.max(close),
                low: open.min(close),
                close,
                volume: Decimal::ONE,
            });
        }
        out
    }

    #[test]
    fn writes_combined_and_per_run_artifacts() {
        let candles = candles(64);
        let split = Split::full(candles.len());
        let split_set = splits(candles.len());
        let all_splits = [split, split_set.train, split_set.validation, split_set.test];
        let boundaries: Vec<SplitBoundary> = all_splits
            .iter()
            .map(|split| SplitBoundary::from_split(&candles, *split))
            .collect();
        let cfg = TsMomentumConfig::default();
        let params = TsBacktestParams::default();
        let outcome = run(&candles, &cfg, &params).unwrap();
        let scenario = CostScenario::base();
        let benchmark_outcome = benchmark(&candles, params.init_fund, &scenario, split).unwrap();
        let metrics = outcome.metrics(params.init_fund);
        let benchmark_metrics = benchmark_outcome.metrics(params.init_fund);
        let dataset = DatasetInfo {
            symbol: "BTC-USDT".into(),
            timeframe: "1day".into(),
            requested_first_ms: candles[0].open_time_ms,
            requested_last_ms: candles[candles.len() - 1].open_time_ms,
            first_open_time_ms: candles[0].open_time_ms,
            last_open_time_ms: candles[candles.len() - 1].open_time_ms,
            candles: candles.len(),
            requested_candles: candles.len(),
            expected_candles: candles.len(),
            missing_candles: 0,
            duplicates_dropped: 0,
            pages: 1,
            fetched_at_ms: 0,
            settle_ms: 5_000,
            content_sha256: "test".into(),
        };
        let context = ReportContext {
            symbol: "BTC-USDT".into(),
            timeframe: "1day".into(),
            as_of_ms: candles[candles.len() - 1].open_time_ms,
            generated_at: "2026-10-23T00:00:00Z".into(),
            days: 64,
            init_fund: params.init_fund,
            config: cfg.clone(),
            borrow_bps_per_day: Decimal::ZERO,
            max_drawdown: params.max_drawdown,
            drawdown_resume: params.drawdown_resume,
            dataset,
            splits: boundaries,
            errors: Vec::new(),
        };
        let boundary = &context.splits[0];
        let run_report = RunReport {
            scenario: ScenarioInfo {
                label: "base".into(),
                fee_bps: scenario.fee_bps,
                slip_bps: scenario.slip_bps,
            },
            variant: Variant::Fixed,
            mode: Mode::Flat,
            split,
            split_start_time: boundary.start_time.clone(),
            split_end_time: boundary.end_time.clone(),
            candles: split.len(),
            init_fund: params.init_fund,
            borrow_bps_per_day: Decimal::ZERO,
            max_drawdown: params.max_drawdown,
            drawdown_resume: params.drawdown_resume,
            metrics,
            benchmark: benchmark_metrics,
            trades: outcome.trades.clone(),
            trading_costs: outcome.trading_costs,
            borrow_costs: outcome.borrow_costs,
            traded_notional: outcome.traded_notional,
            benchmark_trading_costs: benchmark_outcome.trading_costs,
            benchmark_traded_notional: benchmark_outcome.traded_notional,
            points: outcome.points,
        };
        let scenarios = vec![ScenarioRuns {
            info: run_report.scenario.clone(),
            runs: vec![run_report],
        }];
        let out_dir = std::env::temp_dir().join(format!(
            "analyzer-ts-momentum-report-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&out_dir);
        write_reports(&out_dir, &context, &scenarios).unwrap();

        let top = std::fs::read_to_string(out_dir.join("report.md")).unwrap();
        assert!(top.contains("## Dataset"));
        assert!(top.contains("## Strategy and configuration"));
        assert!(top.contains("## base costs"));
        assert!(top.contains("## Regime summary (full period)"));
        assert!(top.contains("## Limitations"));
        assert!(out_dir.join("results.json").is_file());

        let run_dir = out_dir.join("base/a-fixed-flat/full");
        assert!(run_dir.join("report.md").is_file());
        assert!(run_dir.join("results.json").is_file());
        let equity = std::fs::read_to_string(run_dir.join("equity.csv")).unwrap();
        assert!(equity.starts_with("date,equity,exposure,regime\n"));
        assert_eq!(equity.lines().count(), candles.len() + 1);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(run_dir.join("results.json")).unwrap())
                .unwrap();
        assert_eq!(json["run"]["variant"], "Fixed");
        assert_eq!(json["run"]["mode"], "Flat");

        std::fs::remove_dir_all(&out_dir).unwrap();
    }
}
