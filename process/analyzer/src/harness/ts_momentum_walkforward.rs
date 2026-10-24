use crate::harness::sweep::costs::CostScenario;
use crate::harness::ts_momentum_backtest::{
    benchmark, run_range, Metrics, Mode, Split, SplitName, TsBacktestParams, Variant,
};
use crate::harness::ts_momentum_report::DatasetInfo;
use crate::strategy::ts_momentum::{NegativeMomentumMode, TsMomentumConfig};
use crate::strategy::types::Candle;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Write as _;
use std::path::Path;

pub const VOLATILITY_LOOKBACK_DAYS: usize = 30;
pub const OOS_MONTHS: usize = 6;
pub const TOTAL_MONTHS: usize = 60;
pub const TRAIN_MONTHS: [usize; 8] = [12, 18, 24, 30, 36, 42, 48, 54];
pub const BASE_LOOKBACKS: [usize; 3] = [30, 60, 90];
pub const TARGET_VOLS: [Decimal; 3] = [
    Decimal::from_parts(15, 0, 0, false, 2),
    Decimal::from_parts(20, 0, 0, false, 2),
    Decimal::from_parts(25, 0, 0, false, 2),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WfConfig {
    pub lookback: usize,
    pub regime_filter: bool,
    pub target_vol: Decimal,
}

impl WfConfig {
    pub fn baseline() -> Self {
        Self {
            lookback: 30,
            regime_filter: false,
            target_vol: Decimal::new(20, 2),
        }
    }

    pub fn grid() -> Vec<Self> {
        let mut configs = Vec::with_capacity(18);
        for lookback in BASE_LOOKBACKS {
            for regime_filter in [false, true] {
                for target_vol in TARGET_VOLS {
                    configs.push(Self {
                        lookback,
                        regime_filter,
                        target_vol,
                    });
                }
            }
        }
        configs
    }

    pub fn regime_label(self) -> &'static str {
        if self.regime_filter {
            "on"
        } else {
            "off"
        }
    }

    pub fn target_vol_percent(self) -> String {
        (self.target_vol * Decimal::from(100))
            .normalize()
            .to_string()
    }

    pub fn label(self) -> String {
        format!(
            "{}/{}/{}",
            self.lookback,
            self.regime_label(),
            self.target_vol_percent()
        )
    }

    pub fn strategy_config(self) -> TsMomentumConfig {
        TsMomentumConfig {
            momentum_lookback_days: self.lookback,
            volatility_lookback_days: VOLATILITY_LOOKBACK_DAYS,
            target_annualized_volatility: self.target_vol,
            negative_momentum_mode: NegativeMomentumMode::Flat,
            ..TsMomentumConfig::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WfWindowSplit {
    pub months: usize,
    pub train_start: usize,
    pub train_end: usize,
    pub oos_start: usize,
    pub oos_end: usize,
}

impl WfWindowSplit {
    pub fn train(self) -> (usize, usize) {
        (self.train_start, self.train_end)
    }

    pub fn oos(self) -> (usize, usize) {
        (self.oos_start, self.oos_end)
    }

    fn oos_split(self) -> Split {
        Split {
            name: SplitName::Test,
            start: self.oos_start,
            end: self.oos_end,
        }
    }
}

pub fn windows(n: usize) -> Vec<WfWindowSplit> {
    TRAIN_MONTHS
        .iter()
        .map(|&months| {
            let train_end = n * months / TOTAL_MONTHS;
            let oos_end = n * (months + OOS_MONTHS) / TOTAL_MONTHS;
            WfWindowSplit {
                months,
                train_start: 0,
                train_end,
                oos_start: train_end,
                oos_end,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct WfRangeView {
    pub start_index: usize,
    pub end_index: usize,
    pub candles: usize,
    pub start_time: String,
    pub end_time: String,
}

fn range_view(candles: &[Candle], start: usize, end: usize) -> WfRangeView {
    WfRangeView {
        start_index: start,
        end_index: end,
        candles: end.saturating_sub(start),
        start_time: candles
            .get(start)
            .map(|candle| iso(candle.open_time_ms))
            .unwrap_or_else(|| "—".to_string()),
        end_time: candles
            .get(end.saturating_sub(1))
            .map(|candle| iso(candle.open_time_ms))
            .unwrap_or_else(|| "—".to_string()),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WfRank {
    pub rank: usize,
    pub config: WfConfig,
    pub sharpe: Option<f64>,
    pub train: Metrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct WfRun {
    pub config: WfConfig,
    pub rank: Option<usize>,
    pub train_sharpe: Option<f64>,
    pub train: Metrics,
    pub oos_base: Metrics,
    pub oos_stress: Metrics,
    pub successful: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WfWindow {
    pub split: WfWindowSplit,
    pub train: WfRangeView,
    pub oos: WfRangeView,
    pub ranking: Vec<WfRank>,
    pub top: Vec<WfRun>,
    pub baseline: WfRun,
    pub benchmark_stress: Metrics,
    pub successful: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WfSettings {
    pub generated_at: String,
    pub as_of_ms: i64,
    pub days: u32,
    pub init_fund: Decimal,
    pub base: CostScenario,
    pub stress: CostScenario,
    pub min_oos_trades: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WfVerdict {
    Pass,
    Fail,
}

impl WfVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            WfVerdict::Pass => "Pass",
            WfVerdict::Fail => "Fail",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WfDecision {
    pub verdict: WfVerdict,
    pub reasons: Vec<String>,
    pub windows_with_success: usize,
    pub total_windows: usize,
    pub successful_top_runs: usize,
    pub total_top_runs: usize,
    pub baseline_successes: usize,
    pub baseline_runs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WfResult {
    pub dataset: DatasetInfo,
    pub settings: WfSettings,
    pub grid: Vec<WfConfig>,
    pub baseline: WfConfig,
    pub windows: Vec<WfWindow>,
    pub decision: WfDecision,
}

fn backtest_params(
    settings: &WfSettings,
    scenario: &CostScenario,
    config: WfConfig,
) -> TsBacktestParams {
    TsBacktestParams {
        init_fund: settings.init_fund,
        fee_bps: scenario.fee_bps,
        slip_bps: scenario.slip_bps,
        variant: Variant::VolTarget,
        mode: Mode::Flat,
        regime_filter: config.regime_filter,
        ..TsBacktestParams::default()
    }
}

fn rank_sharpe(sharpe: Option<f64>) -> f64 {
    sharpe
        .filter(|value| value.is_finite())
        .unwrap_or(f64::NEG_INFINITY)
}

fn compare_sharpe(left: Option<f64>, right: Option<f64>) -> Ordering {
    rank_sharpe(right)
        .partial_cmp(&rank_sharpe(left))
        .unwrap_or(Ordering::Equal)
}

fn success(
    oos_stress: &Metrics,
    benchmark_stress: &Metrics,
    min_oos_trades: usize,
) -> (bool, Vec<String>) {
    let mut failures = Vec::new();
    if oos_stress.trades < min_oos_trades {
        failures.push(format!(
            "OOS trades {} < {}",
            oos_stress.trades, min_oos_trades
        ));
    }
    match (
        oos_stress.net_cumulative_return,
        benchmark_stress.net_cumulative_return,
    ) {
        (Some(run), Some(benchmark)) if run > benchmark => {}
        (Some(run), Some(benchmark)) => failures.push(format!(
            "stress net return {:.2}% <= benchmark {:.2}%",
            run * 100.0,
            benchmark * 100.0
        )),
        _ => failures.push("stress net return or benchmark unavailable".to_string()),
    }
    match oos_stress.sharpe {
        Some(sharpe) if sharpe > 0.0 => {}
        Some(sharpe) => failures.push(format!("stress Sharpe {sharpe:.3} <= 0")),
        None => failures.push("stress Sharpe unavailable".to_string()),
    }
    (failures.is_empty(), failures)
}

#[allow(clippy::too_many_arguments)]
fn run_oos(
    candles: &[Candle],
    split: WfWindowSplit,
    config: WfConfig,
    rank: Option<usize>,
    train: &Metrics,
    train_sharpe: Option<f64>,
    settings: &WfSettings,
    benchmark_stress: &Metrics,
) -> Result<WfRun, String> {
    let strategy = config.strategy_config();
    let base_params = backtest_params(settings, &settings.base, config);
    let base_outcome = run_range(
        candles,
        &strategy,
        &base_params,
        split.oos_start,
        split.oos_end,
    )?;
    let oos_base = base_outcome.metrics(settings.init_fund);

    let stress_params = backtest_params(settings, &settings.stress, config);
    let stress_outcome = run_range(
        candles,
        &strategy,
        &stress_params,
        split.oos_start,
        split.oos_end,
    )?;
    let oos_stress = stress_outcome.metrics(settings.init_fund);
    let (successful, failures) = success(&oos_stress, benchmark_stress, settings.min_oos_trades);

    Ok(WfRun {
        config,
        rank,
        train_sharpe,
        train: train.clone(),
        oos_base,
        oos_stress,
        successful,
        failures,
    })
}

pub fn evaluate(
    candles: &[Candle],
    dataset: DatasetInfo,
    settings: &WfSettings,
) -> Result<WfResult, String> {
    if candles.is_empty() {
        return Err("no candles to evaluate".into());
    }
    if settings.min_oos_trades == 0 {
        return Err("min_oos_trades must be at least 1".into());
    }
    let grid = WfConfig::grid();
    let baseline_config = WfConfig::baseline();
    let mut evaluated = Vec::with_capacity(TRAIN_MONTHS.len());

    for split in windows(candles.len()) {
        if split.train_end <= split.train_start || split.oos_end <= split.oos_start {
            return Err(format!(
                "empty train or OOS range for {}-month window",
                split.months
            ));
        }
        let mut ranking = Vec::with_capacity(grid.len());
        for config in &grid {
            let strategy = config.strategy_config();
            let params = backtest_params(settings, &settings.base, *config);
            let outcome = run_range(
                candles,
                &strategy,
                &params,
                split.train_start,
                split.train_end,
            )?;
            let train = outcome.metrics(settings.init_fund);
            ranking.push(WfRank {
                rank: 0,
                config: *config,
                sharpe: train.sharpe,
                train,
            });
        }
        ranking.sort_by(|left, right| {
            compare_sharpe(left.sharpe, right.sharpe)
                .then_with(|| left.config.label().cmp(&right.config.label()))
        });
        for (index, entry) in ranking.iter_mut().enumerate() {
            entry.rank = index + 1;
        }

        let benchmark_outcome = benchmark(
            candles,
            settings.init_fund,
            &settings.stress,
            split.oos_split(),
        )?;
        let benchmark_stress = benchmark_outcome.metrics(settings.init_fund);

        let mut top = Vec::with_capacity(3);
        for entry in ranking.iter().take(3) {
            top.push(run_oos(
                candles,
                split,
                entry.config,
                Some(entry.rank),
                &entry.train,
                entry.sharpe,
                settings,
                &benchmark_stress,
            )?);
        }

        let baseline_entry = ranking
            .iter()
            .find(|entry| entry.config == baseline_config)
            .ok_or_else(|| "baseline config missing from grid".to_string())?;
        let baseline = run_oos(
            candles,
            split,
            baseline_config,
            None,
            &baseline_entry.train,
            baseline_entry.sharpe,
            settings,
            &benchmark_stress,
        )?;

        let successful = top.iter().any(|run| run.successful);
        evaluated.push(WfWindow {
            split,
            train: range_view(candles, split.train_start, split.train_end),
            oos: range_view(candles, split.oos_start, split.oos_end),
            ranking,
            top,
            baseline,
            benchmark_stress,
            successful,
        });
    }

    let total_windows = evaluated.len();
    let windows_with_success = evaluated.iter().filter(|window| window.successful).count();
    let successful_top_runs = evaluated
        .iter()
        .flat_map(|window| &window.top)
        .filter(|run| run.successful)
        .count();
    let total_top_runs = total_windows * 3;
    let baseline_successes = evaluated
        .iter()
        .filter(|window| window.baseline.successful)
        .count();
    let baseline_runs = total_windows;
    let majority = total_windows.div_ceil(2).max(1);
    let verdict = if total_windows > 0 && windows_with_success >= majority {
        WfVerdict::Pass
    } else {
        WfVerdict::Fail
    };
    let reasons = vec![
        format!(
            "{} of {} OOS windows contained at least one successful frozen top-3 config; the majority rule requires {}.",
            windows_with_success, total_windows, majority
        ),
        format!(
            "{} of {} frozen top-3 OOS runs passed all gates (>= {} trades, stress net return > BTC buy-and-hold, stress Sharpe > 0).",
            successful_top_runs, total_top_runs, settings.min_oos_trades
        ),
        format!(
            "Baseline {} passed in {} of {} OOS windows.",
            baseline_config.label(),
            baseline_successes,
            baseline_runs
        ),
    ];
    let decision = WfDecision {
        verdict,
        reasons,
        windows_with_success,
        total_windows,
        successful_top_runs,
        total_top_runs,
        baseline_successes,
        baseline_runs,
    };

    Ok(WfResult {
        dataset,
        settings: settings.clone(),
        grid,
        baseline: baseline_config,
        windows: evaluated,
        decision,
    })
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn date_from_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| ms.to_string())
}

fn window_label(months: usize) -> String {
    format!("M1-M{months}")
}

fn oos_label(months: usize) -> String {
    format!("M{}-M{}", months + 1, months + OOS_MONTHS)
}

fn pct(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(|| "—".to_string())
}

fn number(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;
    mean.is_finite().then_some(mean)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Axis {
    Lookback(usize),
    Regime(bool),
    TargetVol(Decimal),
}

#[derive(Debug, Default)]
struct AxisStat {
    selections: usize,
    successes: usize,
    returns: Vec<f64>,
    sharpes: Vec<f64>,
    train_sharpes: Vec<f64>,
}

impl AxisStat {
    fn add(&mut self, run: &WfRun) {
        self.selections += 1;
        if run.successful {
            self.successes += 1;
        }
        if let Some(value) = run.oos_stress.net_cumulative_return {
            self.returns.push(value);
        }
        if let Some(value) = run.oos_stress.sharpe {
            self.sharpes.push(value);
        }
        if let Some(value) = run.train_sharpe {
            self.train_sharpes.push(value);
        }
    }

    fn mean_return(&self) -> Option<f64> {
        mean(&self.returns)
    }

    fn mean_sharpe(&self) -> Option<f64> {
        mean(&self.sharpes)
    }

    fn mean_train_sharpe(&self) -> Option<f64> {
        mean(&self.train_sharpes)
    }
}

fn axis_stats(result: &WfResult) -> Vec<(Axis, AxisStat)> {
    let mut stats = Vec::new();
    for lookback in BASE_LOOKBACKS {
        stats.push((Axis::Lookback(lookback), AxisStat::default()));
    }
    stats.push((Axis::Regime(false), AxisStat::default()));
    stats.push((Axis::Regime(true), AxisStat::default()));
    for target in TARGET_VOLS {
        stats.push((Axis::TargetVol(target), AxisStat::default()));
    }
    for window in &result.windows {
        for run in &window.top {
            for (axis, stat) in stats.iter_mut() {
                let matches = match *axis {
                    Axis::Lookback(value) => run.config.lookback == value,
                    Axis::Regime(value) => run.config.regime_filter == value,
                    Axis::TargetVol(value) => run.config.target_vol == value,
                };
                if matches {
                    stat.add(run);
                }
            }
        }
    }
    stats
}

fn config_stats(result: &WfResult) -> Vec<(WfConfig, AxisStat)> {
    let mut stats: Vec<(WfConfig, AxisStat)> = result
        .grid
        .iter()
        .copied()
        .map(|config| (config, AxisStat::default()))
        .collect();
    for window in &result.windows {
        for run in &window.top {
            if let Some((_, stat)) = stats.iter_mut().find(|(config, _)| *config == run.config) {
                stat.add(run);
            }
        }
    }
    stats
}

fn axis_label(axis: Axis) -> String {
    match axis {
        Axis::Lookback(value) => format!("lookback {value}"),
        Axis::Regime(false) => "regime off".to_string(),
        Axis::Regime(true) => "regime on".to_string(),
        Axis::TargetVol(value) => {
            format!("target vol {}%", (value * Decimal::from(100)).normalize())
        }
    }
}

fn write_dataset_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 1. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", result.dataset.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", result.dataset.timeframe)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Requested range | {} .. {} |",
        date_from_ms(result.dataset.requested_first_ms),
        date_from_ms(result.dataset.requested_last_ms)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Effective range | {} .. {} |",
        date_from_ms(result.dataset.first_open_time_ms),
        date_from_ms(result.dataset.last_open_time_ms)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", result.dataset.candles)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Missing candles | {} |",
        result.dataset.missing_candles
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Duplicates dropped | {} |",
        result.dataset.duplicates_dropped
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Pages | {} |", result.dataset.pages).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Fetched at | {} |",
        iso(result.dataset.fetched_at_ms)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| SHA-256 | {} |", result.dataset.content_sha256)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Train window | Train candles | OOS window | OOS candles | Train range | OOS range |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---|---:|---|---|").expect("writing to String cannot fail");
    for window in &result.windows {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} .. {} | {} .. {} |",
            window_label(window.split.months),
            window.train.candles,
            oos_label(window.split.months),
            window.oos.candles,
            window.train.start_time,
            window.train.end_time,
            window.oos.start_time,
            window.oos.end_time
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_experiment_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 2. Experiment and grid").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy | BTC time-series momentum |")
        .expect("writing to String cannot fail");
    writeln!(out, "| Variant | VolTarget |").expect("writing to String cannot fail");
    writeln!(out, "| Mode | Flat (long/flat only) |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Volatility lookback | {} days (fixed) |",
        VOLATILITY_LOOKBACK_DAYS
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Exposure clamp | 0..1 |").expect("writing to String cannot fail");
    writeln!(out, "| Grid size | {} configs |", result.grid.len())
        .expect("writing to String cannot fail");
    writeln!(out, "| Walk-forward windows | {} |", result.windows.len())
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Train / OOS | expanding train ending at months 12,18,...,54; next 6 months OOS |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Ranking metric | train Sharpe |").expect("writing to String cannot fail");
    writeln!(out, "| Frozen per window | top 3 |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Base costs | fee {} bps, slip {} bps |",
        result.settings.base.fee_bps, result.settings.base.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Stress costs | fee {} bps, slip {} bps |",
        result.settings.stress.fee_bps, result.settings.stress.slip_bps
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| OOS success gates | >= {} trades, stress net return > BTC buy-and-hold stress return, stress Sharpe > 0 |",
        result.settings.min_oos_trades
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Overall decision rule | at least half of the OOS windows contain a successful frozen top-3 config |"
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| # | Lookback | Regime filter | Target vol | Label | Role |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---:|---:|---|---:|---|---|").expect("writing to String cannot fail");
    for (index, config) in result.grid.iter().enumerate() {
        let role = if *config == result.baseline {
            "Baseline"
        } else {
            "Candidate"
        };
        writeln!(
            out,
            "| {} | {} | {} | {}% | {} | {} |",
            index + 1,
            config.lookback,
            config.regime_label(),
            config.target_vol_percent(),
            config.label(),
            role
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_window_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 3. Walk-forward results").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for window in &result.windows {
        writeln!(
            out,
            "### Window {} / OOS {}",
            window_label(window.split.months),
            oos_label(window.split.months)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "Train indices {}..{} ({} candles), OOS indices {}..{} ({} candles).",
            window.train.start_index,
            window.train.end_index,
            window.train.candles,
            window.oos.start_index,
            window.oos.end_index,
            window.oos.candles
        )
        .expect("writing to String cannot fail");
        writeln!(
            out,
            "BTC buy-and-hold stress benchmark: net {}, Sharpe {}, max drawdown {}.",
            pct(window.benchmark_stress.net_cumulative_return),
            number(window.benchmark_stress.sharpe),
            pct(window.benchmark_stress.max_drawdown)
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        writeln!(
            out,
            "| Role | Config | Train Sharpe | Train net | Train trades | OOS base net | OOS base Sharpe | OOS stress net | OOS stress Sharpe | OOS trades | Success | Failure |"
        )
        .expect("writing to String cannot fail");
        writeln!(
            out,
            "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|"
        )
        .expect("writing to String cannot fail");
        for run in &window.top {
            let role = run
                .rank
                .map(|rank| format!("Top {rank}"))
                .unwrap_or_else(|| "Top".to_string());
            let failure = if run.failures.is_empty() {
                "—".to_string()
            } else {
                run.failures.join("; ")
            };
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                role,
                run.config.label(),
                number(run.train_sharpe),
                pct(run.train.net_cumulative_return),
                run.train.trades,
                pct(run.oos_base.net_cumulative_return),
                number(run.oos_base.sharpe),
                pct(run.oos_stress.net_cumulative_return),
                number(run.oos_stress.sharpe),
                run.oos_stress.trades,
                if run.successful { "PASS" } else { "FAIL" },
                failure
            )
            .expect("writing to String cannot fail");
        }
        let baseline = &window.baseline;
        let baseline_failure = if baseline.failures.is_empty() {
            "—".to_string()
        } else {
            baseline.failures.join("; ")
        };
        writeln!(
            out,
            "| Baseline | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            baseline.config.label(),
            number(baseline.train_sharpe),
            pct(baseline.train.net_cumulative_return),
            baseline.train.trades,
            pct(baseline.oos_base.net_cumulative_return),
            number(baseline.oos_base.sharpe),
            pct(baseline.oos_stress.net_cumulative_return),
            number(baseline.oos_stress.sharpe),
            baseline.oos_stress.trades,
            if baseline.successful { "PASS" } else { "FAIL" },
            baseline_failure
        )
        .expect("writing to String cannot fail");
        writeln!(
            out,
            "| Benchmark | BTC buy-and-hold | — | — | — | — | — | {} | {} | {} | — | — |",
            pct(window.benchmark_stress.net_cumulative_return),
            number(window.benchmark_stress.sharpe),
            window.benchmark_stress.trades
        )
        .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
    }
}

fn write_overall_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 4. Overall results").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Metric | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---:|").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Windows with at least one successful top-3 run | {} / {} |",
        result.decision.windows_with_success, result.decision.total_windows
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Successful frozen top-3 runs | {} / {} |",
        result.decision.successful_top_runs, result.decision.total_top_runs
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Baseline {} windows passed | {} / {} |",
        result.baseline.label(),
        result.decision.baseline_successes,
        result.decision.baseline_runs
    )
    .expect("writing to String cannot fail");
    let selected_returns: Vec<f64> = result
        .windows
        .iter()
        .flat_map(|window| &window.top)
        .filter_map(|run| run.oos_stress.net_cumulative_return)
        .collect();
    let baseline_returns: Vec<f64> = result
        .windows
        .iter()
        .filter_map(|window| window.baseline.oos_stress.net_cumulative_return)
        .collect();
    writeln!(
        out,
        "| Mean selected-run OOS stress net return | {} |",
        pct(mean(&selected_returns))
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Mean baseline OOS stress net return | {} |",
        pct(mean(&baseline_returns))
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| Window | Successful top-3 configs | Baseline {} | Baseline OOS stress net | Baseline OOS stress Sharpe | Baseline trades | Benchmark stress net |"
    , result.baseline.label())
        .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for window in &result.windows {
        let successes: Vec<String> = window
            .top
            .iter()
            .filter(|run| run.successful)
            .map(|run| run.config.label())
            .collect();
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            window_label(window.split.months),
            if successes.is_empty() {
                "none".to_string()
            } else {
                successes.join(", ")
            },
            if window.baseline.successful {
                "PASS"
            } else {
                "FAIL"
            },
            pct(window.baseline.oos_stress.net_cumulative_return),
            number(window.baseline.oos_stress.sharpe),
            window.baseline.oos_stress.trades,
            pct(window.benchmark_stress.net_cumulative_return)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| Window | Frozen config | OOS stress net | OOS stress Sharpe | OOS trades | Benchmark stress net |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for window in &result.windows {
        for run in window.top.iter().filter(|run| run.successful) {
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                window_label(window.split.months),
                run.config.label(),
                pct(run.oos_stress.net_cumulative_return),
                number(run.oos_stress.sharpe),
                run.oos_stress.trades,
                pct(window.benchmark_stress.net_cumulative_return)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_stability_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 5. Stability and consistency").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "All statistics below cover the frozen top-3 runs only; the baseline is reported separately."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| Axis | Selections | Successful | Mean train Sharpe | Mean OOS stress net | Mean OOS stress Sharpe |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for (axis, stat) in axis_stats(result) {
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            axis_label(axis),
            stat.selections,
            stat.successes,
            number(stat.mean_train_sharpe()),
            pct(stat.mean_return()),
            number(stat.mean_sharpe())
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(
        out,
        "| Config | Windows selected | Successful runs | Mean OOS stress net | Mean OOS stress Sharpe |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for (config, stat) in config_stats(result) {
        if stat.selections == 0 {
            continue;
        }
        writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            config.label(),
            stat.selections,
            stat.successes,
            pct(stat.mean_return()),
            number(stat.mean_sharpe())
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "| Window | Top-3 frozen configs |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    for window in &result.windows {
        let labels: Vec<String> = window
            .top
            .iter()
            .map(|run| {
                format!(
                    "{} ({})",
                    run.config.label(),
                    if run.successful { "PASS" } else { "FAIL" }
                )
            })
            .collect();
        writeln!(
            out,
            "| {} | {} |",
            window_label(window.split.months),
            labels.join(", ")
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_decision_section(out: &mut String, result: &WfResult) {
    writeln!(out, "## 6. Decision").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "**Overall verdict: {}**",
        result.decision.verdict.as_str()
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "Reasons:").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for reason in &result.decision.reasons {
        writeln!(out, "- {reason}").expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    let stats = axis_stats(result);
    let find = |axis: Axis| -> &AxisStat {
        stats
            .iter()
            .find(|(candidate, _)| *candidate == axis)
            .map(|(_, stat)| stat)
            .expect("axis stat is initialized")
    };
    let regime_off = find(Axis::Regime(false));
    let regime_on = find(Axis::Regime(true));
    let lookback_30 = find(Axis::Lookback(30));
    let lookback_60 = find(Axis::Lookback(60));
    let lookback_90 = find(Axis::Lookback(90));
    let target_15 = find(Axis::TargetVol(TARGET_VOLS[0]));
    let target_20 = find(Axis::TargetVol(TARGET_VOLS[1]));
    let target_25 = find(Axis::TargetVol(TARGET_VOLS[2]));

    writeln!(out, "**Improvement checks against the 30/off/20 baseline**")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Regime filter: on selected {} runs ({} passed, mean stress net {}); off selected {} runs ({} passed, mean stress net {}).",
        regime_on.selections,
        regime_on.successes,
        pct(regime_on.mean_return()),
        regime_off.selections,
        regime_off.successes,
        pct(regime_off.mean_return())
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Longer lookback: 30 selected {} runs ({} passed, mean stress net {}); 60 selected {} runs ({} passed, mean stress net {}); 90 selected {} runs ({} passed, mean stress net {}).",
        lookback_30.selections,
        lookback_30.successes,
        pct(lookback_30.mean_return()),
        lookback_60.selections,
        lookback_60.successes,
        pct(lookback_60.mean_return()),
        lookback_90.selections,
        lookback_90.successes,
        pct(lookback_90.mean_return())
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Volatility target: 15% selected {} runs ({} passed, mean stress net {}); 20% selected {} runs ({} passed, mean stress net {}); 25% selected {} runs ({} passed, mean stress net {}).",
        target_15.selections,
        target_15.successes,
        pct(target_15.mean_return()),
        target_20.selections,
        target_20.successes,
        pct(target_20.mean_return()),
        target_25.selections,
        target_25.successes,
        pct(target_25.mean_return())
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Baseline 30/off/20 passed {} of {} windows (mean stress net {}, mean stress Sharpe {}).",
        result.decision.baseline_successes,
        result.decision.baseline_runs,
        pct(mean(
            &result
                .windows
                .iter()
                .filter_map(|window| window.baseline.oos_stress.net_cumulative_return)
                .collect::<Vec<_>>()
        )),
        number(mean(
            &result
                .windows
                .iter()
                .filter_map(|window| window.baseline.oos_stress.sharpe)
                .collect::<Vec<_>>()
        ))
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_report(path: &Path, result: &WfResult) -> Result<(), String> {
    let mut out = String::new();
    writeln!(
        out,
        "# BTC 1-day time-series momentum walk-forward improvement experiment\n"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "> Generated at {}. This is a cost-aware historical simulation, not evidence of future profitability. The OOS windows were not used to tune the frozen configs.\n",
        result.settings.generated_at
    )
    .expect("writing to String cannot fail");
    write_dataset_section(&mut out, result);
    write_experiment_section(&mut out, result);
    write_window_section(&mut out, result);
    write_overall_section(&mut out, result);
    write_stability_section(&mut out, result);
    write_decision_section(&mut out, result);
    std::fs::write(path, out).map_err(|error| error.to_string())
}

pub fn write_reports(out_dir: &Path, result: &WfResult) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;
    write_report(&out_dir.join("report.md"), result)?;
    let json = serde_json::to_string_pretty(result).map_err(|error| error.to_string())?;
    std::fs::write(out_dir.join("results.json"), json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_has_eighteen_configs_and_includes_the_baseline() {
        let grid = WfConfig::grid();
        assert_eq!(grid.len(), 18);
        assert!(grid.contains(&WfConfig::baseline()));
        assert_eq!(WfConfig::baseline().label(), "30/off/20");
        for lookback in BASE_LOOKBACKS {
            for regime_filter in [false, true] {
                for target_vol in TARGET_VOLS {
                    assert!(grid.contains(&WfConfig {
                        lookback,
                        regime_filter,
                        target_vol
                    }));
                }
            }
        }
    }

    #[test]
    fn windows_are_expanding_and_non_overlapping() {
        let splits = windows(1_800);
        assert_eq!(splits.len(), 8);
        assert_eq!(splits[0].months, 12);
        assert_eq!(splits[7].months, 54);
        for (index, split) in splits.iter().enumerate() {
            assert_eq!(split.train_start, 0);
            assert_eq!(split.train_end, 1_800 * split.months / 60);
            assert_eq!(split.oos_start, split.train_end);
            assert_eq!(split.oos_end, 1_800 * (split.months + 6) / 60);
            if index > 0 {
                assert_eq!(split.oos_start, splits[index - 1].oos_end);
            }
        }
        assert_eq!(splits.last().unwrap().oos_end, 1_800);
    }
}
