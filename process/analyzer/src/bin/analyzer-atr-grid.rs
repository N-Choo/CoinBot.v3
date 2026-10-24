use analyzer::config_env::{env_string, load_dotenv};
use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::atr_experiment::{evaluate_atr_slice, AtrVariant};
use analyzer::harness::dataset::{load_csv, Dataset};
use analyzer::harness::sweep::costs::CostScenario;
use analyzer::harness::sweep::data::{fetch_range, write_cache};
use analyzer::harness::sweep::evaluate::PartitionMetrics;
use analyzer::harness::sweep::partition::Slice;
use analyzer::identity;
use analyzer::strategy::config::Config;
use analyzer::strategy::types::Candle;
use clap::Parser;
use rust_decimal::Decimal;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://api.kucoin.com";
const DEFAULT_SETTLE_MS: i64 = 5_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MIN_CANDLES: usize = 1000;
const ATR_PERIOD: usize = 14;
const ATR_PCT_HIGH: Decimal = Decimal::from_parts(5, 0, 0, false, 0);
const ATR_HIGH_ADJUST: Decimal = Decimal::from_parts(15, 0, 0, false, 1);
const MAX_DRAWDOWN_PCT: Decimal = Decimal::from_parts(20, 0, 0, false, 0);
const SL_MULTIPLIERS: [Decimal; 8] = [
    Decimal::from_parts(15, 0, 0, false, 1),
    Decimal::from_parts(20, 0, 0, false, 1),
    Decimal::from_parts(25, 0, 0, false, 1),
    Decimal::from_parts(30, 0, 0, false, 1),
    Decimal::from_parts(35, 0, 0, false, 1),
    Decimal::from_parts(40, 0, 0, false, 1),
    Decimal::from_parts(45, 0, 0, false, 1),
    Decimal::from_parts(50, 0, 0, false, 1),
];
const TP_MULTIPLIERS: [Decimal; 8] = SL_MULTIPLIERS;

#[derive(Parser, Debug)]
#[command(
    name = "analyzer-atr-grid",
    about = "Focused ATR SL/TP grid with a final-month out-of-sample test"
)]
struct Cli {
    /// Comma-separated KuCoin symbols.
    #[arg(long, default_value = "BTC-USDT,BNB-USDT,SOL-USDT,XRP-USDT")]
    symbols: String,

    /// Timeframe.
    #[arg(long, default_value = "4hour")]
    timeframe: String,

    /// History window in days.
    #[arg(long, default_value_t = 365)]
    days: u32,

    /// Reference time in milliseconds; defaults to now.
    #[arg(long, default_value_t = identity::now_ms())]
    as_of: i64,

    /// Results timestamp in RFC 3339 form; defaults to now.
    #[arg(long)]
    generated_at: Option<String>,

    /// Artifact output directory.
    #[arg(long, default_value = "process/analyzer/experiments/rsi-atr-grid-4h")]
    out_dir: PathBuf,

    /// Candle cache directory.
    #[arg(long, default_value = "data/atr-grid-4h")]
    data_dir: PathBuf,

    /// Refetch every symbol instead of reusing the cache.
    #[arg(long)]
    refresh: bool,

    /// Initial account fund per symbol.
    #[arg(long, default_value_t = 1000)]
    init_fund: i64,

    /// Base fee in basis points per side.
    #[arg(long, default_value_t = 10)]
    fee_bps: u32,

    /// Base slippage in basis points per side.
    #[arg(long, default_value_t = 5)]
    slip_bps: u32,

    /// Stress fee in basis points per side.
    #[arg(long, default_value_t = 20)]
    stress_fee_bps: u32,

    /// Stress slippage in basis points per side.
    #[arg(long, default_value_t = 10)]
    stress_slip_bps: u32,

    /// RSI lookback period.
    #[arg(long, default_value_t = 16)]
    rsi_period: usize,

    /// Long entry threshold.
    #[arg(long = "long", default_value_t = 60)]
    long_threshold: u32,

    /// Short entry threshold.
    #[arg(long = "short", default_value_t = 40)]
    short_threshold: u32,
}

fn parse_list(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in value.split(',') {
        let item = item.trim();
        if item.is_empty() || out.iter().any(|existing| existing == item) {
            continue;
        }
        out.push(item.to_string());
    }
    out
}

fn valid_symbol(symbol: &str) -> bool {
    let mut parts = symbol.split('-');
    let valid_part = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    };
    match (parts.next(), parts.next(), parts.next()) {
        (Some(base), Some(quote), None) => valid_part(base) && valid_part(quote),
        _ => false,
    }
}

fn cache_path(data_dir: &Path, symbol: &str, timeframe: Timeframe) -> PathBuf {
    data_dir.join(format!("{symbol}_{}.csv", timeframe.as_kucoin()))
}

fn dataset_from_cache(
    symbol: &str,
    path: &Path,
    timeframe: Timeframe,
    as_of: i64,
    settle_ms: i64,
    window: usize,
) -> Result<Dataset, String> {
    load_csv(symbol, path, timeframe, as_of, settle_ms, 0, window, 0)
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn load_symbol(
    symbol: &str,
    timeframe: Timeframe,
    start_ms: i64,
    as_of: i64,
    settle_ms: i64,
    window: usize,
    data_dir: &Path,
    base_url: &str,
    refresh: bool,
) -> Result<Dataset, String> {
    let path = cache_path(data_dir, symbol, timeframe);
    if !refresh && path.is_file() {
        match dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms, window) {
            Ok(dataset) if dataset.candles.len() >= MIN_CANDLES => {
                println!(
                    "{}: reusing cache, {} closed candles",
                    symbol,
                    dataset.candles.len()
                );
                return Ok(dataset);
            }
            Ok(dataset) => {
                println!(
                    "{}: cache has only {} closed candles; refetching",
                    symbol,
                    dataset.candles.len()
                );
            }
            Err(error) => {
                println!("{}: cache unusable ({error}); refetching", symbol);
            }
        }
    }

    let (candles, pages, duplicates) = fetch_range(
        base_url, symbol, timeframe, start_ms, as_of, settle_ms, as_of,
    )?;
    println!(
        "{}: fetched {} closed candles in {} page(s), {} duplicates dropped",
        symbol,
        candles.len(),
        pages,
        duplicates
    );
    write_cache(&path, &candles)?;
    dataset_from_cache(symbol, &path, timeframe, as_of, settle_ms, window)
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ms.to_string())
}

fn candle_time(candles: &[Candle], index: usize) -> String {
    candles
        .get(index)
        .map(|candle| iso(candle.open_time_ms))
        .unwrap_or_else(|| "—".to_string())
}

fn decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn profit_factor(value: Option<Decimal>, trades: usize) -> String {
    match value {
        Some(value) => decimal(value),
        None if trades == 0 => "—".to_string(),
        None => "∞".to_string(),
    }
}

fn off_variant() -> AtrVariant {
    AtrVariant {
        label: "ATR off".into(),
        enabled: false,
        period: ATR_PERIOD,
        sl_multiplier: Decimal::ZERO,
        tp_multiplier: Decimal::ZERO,
        pct_high: Decimal::ZERO,
        high_adjust: Decimal::ONE,
        sl_max_pct: Decimal::ZERO,
    }
}

fn grid_variants() -> Vec<AtrVariant> {
    let mut variants = Vec::with_capacity(SL_MULTIPLIERS.len() * TP_MULTIPLIERS.len());
    for sl in SL_MULTIPLIERS {
        for tp in TP_MULTIPLIERS {
            variants.push(AtrVariant {
                label: format!("SL{sl}/TP{tp}"),
                enabled: true,
                period: ATR_PERIOD,
                sl_multiplier: sl,
                tp_multiplier: tp,
                pct_high: ATR_PCT_HIGH,
                high_adjust: ATR_HIGH_ADJUST,
                sl_max_pct: Decimal::ZERO,
            });
        }
    }
    variants
}

#[derive(Debug, Clone, Serialize)]
struct MetricsView {
    trades: usize,
    net_expectancy: Decimal,
    net_profit_factor: Option<Decimal>,
    max_drawdown_pct: Decimal,
    expectancy_stderr: Decimal,
    closed_net_pnl: Decimal,
    open_net_pnl: Decimal,
    net_pnl: Decimal,
    win_rate: Decimal,
}

impl MetricsView {
    fn from_metrics(metrics: &PartitionMetrics) -> Self {
        Self {
            trades: metrics.trades,
            net_expectancy: metrics.net_expectancy,
            net_profit_factor: metrics.net_profit_factor,
            max_drawdown_pct: metrics.max_drawdown_pct,
            expectancy_stderr: metrics.expectancy_stderr,
            closed_net_pnl: metrics.closed_net_pnl,
            open_net_pnl: metrics.open_net_pnl,
            net_pnl: metrics.net_pnl,
            win_rate: metrics.win_rate,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DatasetView {
    first_open_time: String,
    last_open_time: String,
    candles: usize,
    content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct RangeView {
    start_index: usize,
    end_index: usize,
    candles: usize,
    start_time: String,
    end_time: String,
}

#[derive(Debug, Clone, Serialize)]
struct SplitView {
    train: RangeView,
    month12: RangeView,
}

#[derive(Debug, Clone, Serialize)]
struct StrategyView {
    rsi_period: usize,
    rsi_long_threshold: Decimal,
    rsi_short_threshold: Decimal,
    window: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CostView {
    label: String,
    fee_bps: Decimal,
    slip_bps: Decimal,
}

impl CostView {
    fn from_scenario(scenario: &CostScenario) -> Self {
        Self {
            label: scenario.label.clone(),
            fee_bps: scenario.fee_bps,
            slip_bps: scenario.slip_bps,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PeriodView {
    base: MetricsView,
    stress: MetricsView,
}

#[derive(Debug, Clone, Serialize)]
struct GridView {
    atr_period: usize,
    sl_multipliers: Vec<Decimal>,
    tp_multipliers: Vec<Decimal>,
    pct_high: Decimal,
    high_adjust: Decimal,
    sl_max_pct: Decimal,
    variants: usize,
}

#[derive(Debug, Clone, Serialize)]
struct GateView {
    max_drawdown_pct: Decimal,
    qualified: usize,
    total: usize,
    applied: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct VariantResult {
    variant: AtrVariant,
    train: MetricsView,
    top_rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct TopResult {
    rank: usize,
    variant: AtrVariant,
    train: MetricsView,
    month12: PeriodView,
}

#[derive(Debug, Clone, Serialize)]
struct SymbolResults {
    symbol: String,
    timeframe: String,
    generated_at: String,
    as_of_ms: i64,
    days: u32,
    dataset: DatasetView,
    split: SplitView,
    strategy: StrategyView,
    base_costs: CostView,
    stress_costs: CostView,
    grid: GridView,
    gate: GateView,
    baseline_train: MetricsView,
    baseline_month12: PeriodView,
    leaderboard: Vec<VariantResult>,
    top: Vec<TopResult>,
    conclusion: String,
}

fn range_view(candles: &[Candle], slice: Slice) -> RangeView {
    RangeView {
        start_index: slice.start,
        end_index: slice.end,
        candles: slice.end.saturating_sub(slice.start),
        start_time: candle_time(candles, slice.start),
        end_time: candle_time(candles, slice.end.saturating_sub(1)),
    }
}

fn profit_factor_order(left: Option<Decimal>, right: Option<Decimal>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => right.cmp(&left),
    }
}

fn rank_variants<'a>(variants: &[&'a VariantResult]) -> Vec<(&'a VariantResult, usize, usize)> {
    let mut by_pf = variants.to_vec();
    by_pf.sort_by(|a, b| {
        profit_factor_order(a.train.net_profit_factor, b.train.net_profit_factor)
            .then_with(|| b.train.net_expectancy.cmp(&a.train.net_expectancy))
            .then_with(|| a.variant.label.cmp(&b.variant.label))
    });
    let pf_rank: BTreeMap<String, usize> = by_pf
        .iter()
        .enumerate()
        .map(|(rank, variant)| (variant.variant.label.clone(), rank))
        .collect();

    let mut by_expectancy = variants.to_vec();
    by_expectancy.sort_by(|a, b| {
        b.train
            .net_expectancy
            .cmp(&a.train.net_expectancy)
            .then_with(|| profit_factor_order(a.train.net_profit_factor, b.train.net_profit_factor))
            .then_with(|| a.variant.label.cmp(&b.variant.label))
    });
    let expectancy_rank: BTreeMap<String, usize> = by_expectancy
        .iter()
        .enumerate()
        .map(|(rank, variant)| (variant.variant.label.clone(), rank))
        .collect();

    let mut ranked: Vec<(&VariantResult, usize, usize)> = variants
        .iter()
        .map(|variant| {
            let label = &variant.variant.label;
            (*variant, pf_rank[label], expectancy_rank[label])
        })
        .collect();
    ranked.sort_by(|a, b| {
        (a.1 + a.2)
            .cmp(&(b.1 + b.2))
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.0.variant.label.cmp(&b.0.variant.label))
    });
    ranked
}

fn select_top(
    variants: &[VariantResult],
    max_drawdown_pct: Decimal,
) -> (Vec<&VariantResult>, GateView) {
    let total = variants.len();
    let qualified: Vec<&VariantResult> = variants
        .iter()
        .filter(|variant| variant.train.max_drawdown_pct <= max_drawdown_pct)
        .collect();
    let apply_gate = qualified.len() >= 3;
    let candidates: Vec<&VariantResult> = if apply_gate {
        qualified.clone()
    } else {
        variants.iter().collect()
    };
    let note = if apply_gate {
        format!(
            "Drawdown gate applied: {} of {} variants have max DD <= {}%.",
            qualified.len(),
            total,
            decimal(max_drawdown_pct)
        )
    } else {
        format!(
            "Drawdown gate failed: only {} of {} variants have max DD <= {}%; top 3 selected from all {} variants without the gate.",
            qualified.len(),
            total,
            decimal(max_drawdown_pct),
            total
        )
    };
    let ranked = rank_variants(&candidates);
    let top = ranked
        .into_iter()
        .take(3)
        .map(|(variant, _, _)| variant)
        .collect();
    (
        top,
        GateView {
            max_drawdown_pct,
            qualified: qualified.len(),
            total,
            applied: apply_gate,
            note,
        },
    )
}

fn pf_beats(candidate: Option<Decimal>, baseline: Option<Decimal>) -> bool {
    profit_factor_order(candidate, baseline) == Ordering::Less
}

fn conclusion(results: &SymbolResults) -> String {
    if results.top.is_empty() {
        return "No ATR variant was available for the month-12 comparison.".into();
    }
    let baseline = &results.baseline_month12.base;
    let mut expectancy_wins = 0usize;
    let mut pf_wins = 0usize;
    let mut both_wins = 0usize;
    let mut total_wins = 0usize;
    let mut lines = Vec::new();
    for top in &results.top {
        let month12 = &top.month12.base;
        let exp_beats = month12.net_expectancy > baseline.net_expectancy;
        let pf_win = pf_beats(month12.net_profit_factor, baseline.net_profit_factor);
        let total_win = month12.net_pnl > baseline.net_pnl;
        if exp_beats {
            expectancy_wins += 1;
        }
        if pf_win {
            pf_wins += 1;
        }
        if exp_beats && pf_win {
            both_wins += 1;
        }
        if total_win {
            total_wins += 1;
        }
        lines.push(format!(
            "top-{} {}: month-12 base net expectancy {} vs ATR-off {}, PF {} vs {}, closed net PnL {} + open mark {} = total net PnL {} vs ATR-off total {}; stress net expectancy {} vs ATR-off {}.",
            top.rank,
            top.variant.label,
            decimal(month12.net_expectancy),
            decimal(baseline.net_expectancy),
            profit_factor(month12.net_profit_factor, month12.trades),
            profit_factor(baseline.net_profit_factor, baseline.trades),
            decimal(month12.closed_net_pnl),
            decimal(month12.open_net_pnl),
            decimal(month12.net_pnl),
            decimal(baseline.net_pnl),
            decimal(top.month12.stress.net_expectancy),
            decimal(results.baseline_month12.stress.net_expectancy)
        ));
    }
    let expectancy_verdict = if expectancy_wins > 0 { "Yes" } else { "No" };
    let total_verdict = if total_wins > 0 { "Yes" } else { "No" };
    format!(
        "{}\n\nOn month-12 base costs, {} of 3 top-3 ATR configs beat ATR-off on closed-trade net expectancy, {} of 3 beat it on PF, {} beat it on both, and {} beat it on total net PnL including open marks. Did any top-3 ATR config beat ATR-off on month 12? By net expectancy: {}. By total net PnL: {}.",
        lines.join("\n"),
        expectancy_wins,
        pf_wins,
        both_wins,
        total_wins,
        expectancy_verdict,
        total_verdict
    )
}

fn best_top(result: &SymbolResults) -> Option<&TopResult> {
    let mut best: Option<&TopResult> = None;
    for candidate in &result.top {
        let replace = match best {
            None => true,
            Some(current) => {
                let candidate_metrics = &candidate.month12.base;
                let current_metrics = &current.month12.base;
                candidate_metrics.net_expectancy > current_metrics.net_expectancy
                    || (candidate_metrics.net_expectancy == current_metrics.net_expectancy
                        && profit_factor_order(
                            candidate_metrics.net_profit_factor,
                            current_metrics.net_profit_factor,
                        ) == Ordering::Less)
                    || (candidate_metrics.net_expectancy == current_metrics.net_expectancy
                        && candidate_metrics.net_profit_factor == current_metrics.net_profit_factor
                        && candidate.variant.label < current.variant.label)
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn evaluate_symbol(
    dataset: &Dataset,
    cfg: &Config,
    generated_at: &str,
    as_of: i64,
    days: u32,
    init_fund: i64,
    base: &CostScenario,
    stress: &CostScenario,
) -> Result<SymbolResults, String> {
    let n = dataset.candles.len();
    let train = Slice {
        start: 0,
        end: n * 11 / 12,
    };
    let month12 = Slice {
        start: n * 11 / 12,
        end: n,
    };
    if train.end == 0 || month12.start >= month12.end {
        return Err(format!(
            "{}: {} candles are too few for an 11/12 + 1/12 split",
            dataset.symbol, n
        ));
    }

    let baseline = off_variant();
    let baseline_train =
        evaluate_atr_slice(dataset, cfg, &baseline, train, cfg.window, init_fund, base)?.metrics;
    let baseline_month12_base = evaluate_atr_slice(
        dataset, cfg, &baseline, month12, cfg.window, init_fund, base,
    )?
    .metrics;
    let baseline_month12_stress = evaluate_atr_slice(
        dataset, cfg, &baseline, month12, cfg.window, init_fund, stress,
    )?
    .metrics;

    let mut variants = Vec::with_capacity(SL_MULTIPLIERS.len() * TP_MULTIPLIERS.len());
    for variant in grid_variants() {
        let train_metrics =
            evaluate_atr_slice(dataset, cfg, &variant, train, cfg.window, init_fund, base)?.metrics;
        variants.push(VariantResult {
            variant,
            train: MetricsView::from_metrics(&train_metrics),
            top_rank: None,
        });
    }

    let (selected, gate) = select_top(&variants, MAX_DRAWDOWN_PCT);
    let top_inputs: Vec<(usize, AtrVariant, MetricsView)> = selected
        .iter()
        .enumerate()
        .map(|(index, variant)| (index + 1, variant.variant.clone(), variant.train.clone()))
        .collect();
    for variant in &mut variants {
        variant.top_rank = top_inputs
            .iter()
            .find(|(_, top, _)| top.label == variant.variant.label)
            .map(|(rank, _, _)| *rank);
    }

    let mut top = Vec::with_capacity(top_inputs.len());
    for (rank, variant, train_metrics) in top_inputs {
        let base_metrics =
            evaluate_atr_slice(dataset, cfg, &variant, month12, cfg.window, init_fund, base)?
                .metrics;
        let stress_metrics = evaluate_atr_slice(
            dataset, cfg, &variant, month12, cfg.window, init_fund, stress,
        )?
        .metrics;
        top.push(TopResult {
            rank,
            variant,
            train: train_metrics,
            month12: PeriodView {
                base: MetricsView::from_metrics(&base_metrics),
                stress: MetricsView::from_metrics(&stress_metrics),
            },
        });
    }

    let mut results = SymbolResults {
        symbol: dataset.symbol.clone(),
        timeframe: dataset.provenance.requested.timeframe.clone(),
        generated_at: generated_at.to_string(),
        as_of_ms: as_of,
        days,
        dataset: DatasetView {
            first_open_time: candle_time(&dataset.candles, 0),
            last_open_time: candle_time(&dataset.candles, n.saturating_sub(1)),
            candles: n,
            content_sha256: dataset.provenance.content_sha256.clone(),
        },
        split: SplitView {
            train: range_view(&dataset.candles, train),
            month12: range_view(&dataset.candles, month12),
        },
        strategy: StrategyView {
            rsi_period: cfg.rsi_period,
            rsi_long_threshold: cfg.rsi_long_threshold,
            rsi_short_threshold: cfg.rsi_short_threshold,
            window: cfg.window,
        },
        base_costs: CostView::from_scenario(base),
        stress_costs: CostView::from_scenario(stress),
        grid: GridView {
            atr_period: ATR_PERIOD,
            sl_multipliers: SL_MULTIPLIERS.to_vec(),
            tp_multipliers: TP_MULTIPLIERS.to_vec(),
            pct_high: ATR_PCT_HIGH,
            high_adjust: ATR_HIGH_ADJUST,
            sl_max_pct: Decimal::ZERO,
            variants: variants.len(),
        },
        gate,
        baseline_train: MetricsView::from_metrics(&baseline_train),
        baseline_month12: PeriodView {
            base: MetricsView::from_metrics(&baseline_month12_base),
            stress: MetricsView::from_metrics(&baseline_month12_stress),
        },
        leaderboard: variants,
        top,
        conclusion: String::new(),
    };
    results.conclusion = conclusion(&results);
    Ok(results)
}

fn write_train_top_rows(out: &mut String, results: &SymbolResults) {
    writeln!(
        out,
        "| Rank | Variant | Trades | Net expectancy | PF | Max DD % | Stderr |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
    for top in &results.top {
        writeln!(
            out,
            "| top-{} | {} | {} | {} | {} | {} | {} |",
            top.rank,
            top.variant.label,
            top.train.trades,
            decimal(top.train.net_expectancy),
            profit_factor(top.train.net_profit_factor, top.train.trades),
            decimal(top.train.max_drawdown_pct),
            decimal(top.train.expectancy_stderr)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_baseline_train_row(out: &mut String, metrics: &MetricsView) {
    writeln!(out, "| Trades | Net expectancy | PF | Max DD % | Stderr |")
        .expect("writing to String cannot fail");
    writeln!(out, "|---:|---:|---:|---:|---:|").expect("writing to String cannot fail");
    writeln!(
        out,
        "| {} | {} | {} | {} | {} |",
        metrics.trades,
        decimal(metrics.net_expectancy),
        profit_factor(metrics.net_profit_factor, metrics.trades),
        decimal(metrics.max_drawdown_pct),
        decimal(metrics.expectancy_stderr)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
}

fn write_period_rows(out: &mut String, period: &PeriodView) {
    writeln!(
        out,
        "| Costs | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|")
        .expect("writing to String cannot fail");
    for (label, metrics) in [("Base", &period.base), ("Stress", &period.stress)] {
        writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} | {} |",
            metrics.trades,
            decimal(metrics.net_expectancy),
            profit_factor(metrics.net_profit_factor, metrics.trades),
            decimal(metrics.max_drawdown_pct),
            decimal(metrics.expectancy_stderr),
            decimal(metrics.closed_net_pnl),
            decimal(metrics.open_net_pnl)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
}

fn write_symbol_report(dir: &Path, results: &SymbolResults) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let mut out = String::new();
    writeln!(
        out,
        "# ATR SL/TP Grid — {} ({}, {} days, month-12 OOS)",
        results.symbol, results.timeframe, results.days
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Focused experiment using fixed RSI signals and an ATR SL/TP grid. This is a research artifact, not a profitability claim. Generated at `{}`.",
        results.generated_at
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 1. Dataset").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Symbol | {} |", results.symbol).expect("writing to String cannot fail");
    writeln!(out, "| Timeframe | {} |", results.timeframe).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Date range | {} .. {} |",
        results.dataset.first_open_time, results.dataset.last_open_time
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Candles | {} |", results.dataset.candles)
        .expect("writing to String cannot fail");
    writeln!(out, "| SHA-256 | {} |", results.dataset.content_sha256)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| As-of | {} ({} ms) |",
        iso(results.as_of_ms),
        results.as_of_ms
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Requested window | {} days |", results.days)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 2. Split").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Partition | Candle range | Candles | First open | Last open |"
    )
    .expect("writing to String cannot fail");
    writeln!(out, "|---|---|---:|---|---|").expect("writing to String cannot fail");
    for (label, range) in [
        ("Train (first 11/12)", &results.split.train),
        ("Month 12 OOS (final 1/12)", &results.split.month12),
    ] {
        writeln!(
            out,
            "| {label} | {}..{} | {} | {} | {} |",
            range.start_index, range.end_index, range.candles, range.start_time, range.end_time
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Both partitions use the existing {}-candle warm-up; only trades opened inside the partition are counted.",
        results.strategy.window
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 3. Experiment").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "| Field | Value |").expect("writing to String cannot fail");
    writeln!(out, "|---|---|").expect("writing to String cannot fail");
    writeln!(out, "| Strategy version | rsi-mom-v1 |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| RSI | period {} / long {} / short {} |",
        results.strategy.rsi_period,
        decimal(results.strategy.rsi_long_threshold),
        decimal(results.strategy.rsi_short_threshold)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| ATR period | {} |", results.grid.atr_period)
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| ATR grid | SL {} × TP {} = {} variants |",
        results
            .grid
            .sl_multipliers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        results
            .grid
            .tp_multipliers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        results.grid.variants
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| High-vol adjustment | both exits × {} when ATR% > {} |",
        decimal(results.grid.high_adjust),
        decimal(results.grid.pct_high)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Stop cap | {} |",
        if results.grid.sl_max_pct.is_zero() {
            "none".to_string()
        } else {
            format!("{}%", decimal(results.grid.sl_max_pct))
        }
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Baseline | ATR off, same RSI signals |")
        .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Base costs | {} bps fee + {} bps slippage per side |",
        decimal(results.base_costs.fee_bps),
        decimal(results.base_costs.slip_bps)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Stress costs | {} bps fee + {} bps slippage per side |",
        decimal(results.stress_costs.fee_bps),
        decimal(results.stress_costs.slip_bps)
    )
    .expect("writing to String cannot fail");
    writeln!(out, "| Initial fund | 1000 |").expect("writing to String cannot fail");
    writeln!(
        out,
        "| Warm-up window | {} candles |",
        results.strategy.window
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Train ranking | Combined PF rank + net expectancy rank; PF None/infinite ranks first; no trade-count gate |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "| Drawdown gate | max DD <= {}% when at least 3 variants qualify |",
        decimal(results.gate.max_drawdown_pct)
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 4. Train results — first 11/12").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "{}", results.gate.note).expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "### Top 3 ATR variants (base costs)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_train_top_rows(&mut out, results);
    writeln!(out, "### ATR-off baseline (base costs)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_baseline_train_row(&mut out, &results.baseline_train);

    writeln!(out, "## 5. Month 12 OOS").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    for top in &results.top {
        writeln!(out, "### Top-{}: {}", top.rank, top.variant.label)
            .expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        write_period_rows(&mut out, &top.month12);
    }
    writeln!(out, "### ATR-off baseline (month 12)").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    write_period_rows(&mut out, &results.baseline_month12);

    writeln!(out, "## 6. Conclusion").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(out, "{}", results.conclusion).expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 7. Execution assumptions").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- A signal produced at a candle close fills at the next candle open."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- One position per symbol; opposite signals reverse at the fill open."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- ATR is computed on the signal candle; SL/TP levels use the signal close."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- SL/TP are checked on the entry candle and later candles; gap exits use the open, and SL wins when both levels are touched."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The high-vol adjustment multiplies both exits by {} when ATR% > {} and has no stop cap.",
        decimal(results.grid.high_adjust),
        decimal(results.grid.pct_high)
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Budget is `init_fund / 2`; costs are applied post-hoc to fills and marks."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## 8. Notes").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Train ranking uses base costs only. Stress costs are reported on the final month for the frozen top 3 and the ATR-off baseline."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- The month-12 result is not used to select the top 3."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- This report is not a profitability claim.")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`."
    )
    .expect("writing to String cannot fail");

    std::fs::write(dir.join("report.md"), out).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("results.json"), json).map_err(|error| error.to_string())
}

fn write_leaderboard(path: &Path, results: &SymbolResults) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| error.to_string())?;
    writer
        .write_record([
            "label",
            "sl_multiplier",
            "tp_multiplier",
            "atr_period",
            "pct_high",
            "high_adjust",
            "sl_max_pct",
            "train_trades",
            "train_net_expectancy",
            "train_net_profit_factor",
            "train_max_drawdown_pct",
            "train_expectancy_stderr",
            "train_closed_net_pnl",
            "train_open_net_pnl",
            "train_net_pnl",
            "train_win_rate",
            "pf_rank",
            "expectancy_rank",
            "combined_rank",
            "dd_gate_pass",
            "top_rank",
        ])
        .map_err(|error| error.to_string())?;
    let variants: Vec<&VariantResult> = results.leaderboard.iter().collect();
    for (index, (variant, pf_rank, expectancy_rank)) in
        rank_variants(&variants).into_iter().enumerate()
    {
        let record = [
            variant.variant.label.clone(),
            variant.variant.sl_multiplier.to_string(),
            variant.variant.tp_multiplier.to_string(),
            variant.variant.period.to_string(),
            variant.variant.pct_high.to_string(),
            variant.variant.high_adjust.to_string(),
            variant.variant.sl_max_pct.to_string(),
            variant.train.trades.to_string(),
            variant.train.net_expectancy.to_string(),
            variant
                .train
                .net_profit_factor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            variant.train.max_drawdown_pct.to_string(),
            variant.train.expectancy_stderr.to_string(),
            variant.train.closed_net_pnl.to_string(),
            variant.train.open_net_pnl.to_string(),
            variant.train.net_pnl.to_string(),
            variant.train.win_rate.to_string(),
            (pf_rank + 1).to_string(),
            (expectancy_rank + 1).to_string(),
            (index + 1).to_string(),
            (variant.train.max_drawdown_pct <= results.gate.max_drawdown_pct).to_string(),
            variant
                .top_rank
                .map(|rank| rank.to_string())
                .unwrap_or_default(),
        ];
        writer
            .write_record(record)
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_combined_index(
    dir: &Path,
    results: &[SymbolResults],
    errors: &[String],
    generated_at: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let timeframe = results
        .first()
        .map(|result| result.timeframe.as_str())
        .unwrap_or("—");
    let days = results.first().map(|result| result.days).unwrap_or(0);
    let mut out = String::new();
    writeln!(out, "# ATR SL/TP Grid — {timeframe}, {days} Days")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    if let Some(first) = results.first() {
        writeln!(
            out,
            "Focused experiment across {}. Fixed RSI {} / long {} / short {}; ATR({}) SL/TP grid with {} variants; train = first 11/12, month-12 OOS = final 1/12. Base costs {} bps fee + {} bps slippage per side; stress costs {} + {}. Generated at `{generated_at}`.",
            results
                .iter()
                .map(|result| result.symbol.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            first.strategy.rsi_period,
            decimal(first.strategy.rsi_long_threshold),
            decimal(first.strategy.rsi_short_threshold),
            first.grid.atr_period,
            first.grid.variants,
            decimal(first.base_costs.fee_bps),
            decimal(first.base_costs.slip_bps),
            decimal(first.stress_costs.fee_bps),
            decimal(first.stress_costs.slip_bps)
        )
        .expect("writing to String cannot fail");
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Per-symbol top-3 month-12 OOS").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Rank | Variant | Trades B | Net expectancy B | PF B | Max DD % B | Stderr B | Trades S | Net expectancy S | PF S | Max DD % S | Stderr S | Closed net PnL B | Open mark B |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    .expect("writing to String cannot fail");
    for result in results {
        for top in &result.top {
            let base = &top.month12.base;
            let stress = &top.month12.stress;
            writeln!(
                out,
                "| [{}]({}/report.md) | top-{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                result.symbol,
                result.symbol,
                top.rank,
                top.variant.label,
                base.trades,
                decimal(base.net_expectancy),
                profit_factor(base.net_profit_factor, base.trades),
                decimal(base.max_drawdown_pct),
                decimal(base.expectancy_stderr),
                stress.trades,
                decimal(stress.net_expectancy),
                profit_factor(stress.net_profit_factor, stress.trades),
                decimal(stress.max_drawdown_pct),
                decimal(stress.expectancy_stderr),
                decimal(base.closed_net_pnl),
                decimal(base.open_net_pnl)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");

    writeln!(out, "## Best top-3 variant per symbol (month-12 base)")
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "| Symbol | Best variant | Trades | Net expectancy | PF | Max DD % | Total net PnL | ATR-off net expectancy | ATR-off PF | ATR-off total net PnL | Beat ATR-off (exp+PF)? | Beat ATR-off (total PnL)? |"
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|"
    )
    .expect("writing to String cannot fail");
    for result in results {
        let baseline = &result.baseline_month12.base;
        if let Some(best) = best_top(result) {
            let metrics = &best.month12.base;
            let beat = metrics.net_expectancy > baseline.net_expectancy
                && pf_beats(metrics.net_profit_factor, baseline.net_profit_factor);
            let beat_total = metrics.net_pnl > baseline.net_pnl;
            writeln!(
                out,
                "| [{}]({}/report.md) | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                result.symbol,
                result.symbol,
                best.variant.label,
                metrics.trades,
                decimal(metrics.net_expectancy),
                profit_factor(metrics.net_profit_factor, metrics.trades),
                decimal(metrics.max_drawdown_pct),
                decimal(metrics.net_pnl),
                decimal(baseline.net_expectancy),
                profit_factor(baseline.net_profit_factor, baseline.trades),
                decimal(baseline.net_pnl),
                if beat { "yes" } else { "no" },
                if beat_total { "yes" } else { "no" }
            )
            .expect("writing to String cannot fail");
        } else {
            writeln!(
                out,
                "| [{}]({}/report.md) | — | — | — | — | — | — | {} | {} | {} | no | no |",
                result.symbol,
                result.symbol,
                decimal(baseline.net_expectancy),
                profit_factor(baseline.net_profit_factor, baseline.trades),
                decimal(baseline.net_pnl)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "Best is the top-3 config with the highest month-12 base net expectancy (PF then label break ties). This is descriptive only; selection was frozen on the first 11/12. Total net PnL is closed net PnL plus the open mark."
    )
    .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    if !errors.is_empty() {
        writeln!(out, "## Data issues").expect("writing to String cannot fail");
        writeln!(out).expect("writing to String cannot fail");
        for error in errors {
            writeln!(out, "- {error}").expect("writing to String cannot fail");
        }
        writeln!(out).expect("writing to String cannot fail");
    }

    writeln!(out, "## Notes").expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");
    writeln!(
        out,
        "- Ranking uses train base costs only. The drawdown gate is applied when it leaves at least 3 variants; otherwise the gate failure is reported and ranking falls back to all variants."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Execution: signal at close, fill at next open, one position, reversal exits, ATR SL/TP with gap/SL-first ordering."
    )
    .expect("writing to String cannot fail");
    writeln!(
        out,
        "- Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`."
    )
    .expect("writing to String cannot fail");
    writeln!(out, "- This report is not a profitability claim.")
        .expect("writing to String cannot fail");

    std::fs::write(dir.join("README.md"), out).map_err(|error| error.to_string())
}

fn print_summary(results: &[SymbolResults], errors: &[String], generated_at: &str) {
    println!();
    println!(
        "ATR grid completed for {} symbol(s); generated_at={generated_at}",
        results.len()
    );
    for result in results {
        println!(
            "{}: candles={} train_variants={} gate_applied={} top={}",
            result.symbol,
            result.dataset.candles,
            result.leaderboard.len(),
            result.gate.applied,
            result
                .top
                .iter()
                .map(|top| format!("{}:{}", top.rank, top.variant.label))
                .collect::<Vec<_>>()
                .join(",")
        );
        for top in &result.top {
            println!(
                "  top-{} {}: month12_base_expectancy={} PF={} total_net_pnl={} stress_expectancy={}",
                top.rank,
                top.variant.label,
                decimal(top.month12.base.net_expectancy),
                profit_factor(top.month12.base.net_profit_factor, top.month12.base.trades),
                decimal(top.month12.base.net_pnl),
                decimal(top.month12.stress.net_expectancy)
            );
        }
        println!(
            "  ATR-off month12_base_expectancy={} PF={} total_net_pnl={}",
            decimal(result.baseline_month12.base.net_expectancy),
            profit_factor(
                result.baseline_month12.base.net_profit_factor,
                result.baseline_month12.base.trades
            ),
            decimal(result.baseline_month12.base.net_pnl)
        );
    }
    if !errors.is_empty() {
        println!("{} data issue(s):", errors.len());
        for error in errors {
            println!("  - {error}");
        }
    }
}

fn main() -> Result<(), String> {
    load_dotenv();
    share::logger::init_logger();
    let cli = Cli::parse();

    let symbols = parse_list(&cli.symbols);
    if symbols.is_empty() {
        return Err("--symbols resolved to an empty list".into());
    }
    for symbol in &symbols {
        if !valid_symbol(symbol) {
            return Err(format!(
                "invalid symbol {symbol:?}; expected BASE-QUOTE matching ^[A-Z0-9]+-[A-Z0-9]+$"
            ));
        }
    }

    let timeframe = Timeframe::from_kucoin(&cli.timeframe)
        .ok_or_else(|| format!("unsupported timeframe {:?}", cli.timeframe))?;
    if cli.days == 0 {
        return Err("--days must be at least 1".into());
    }
    if cli.init_fund <= 0 {
        return Err("--init-fund must be positive".into());
    }
    for (name, value) in [
        ("--fee-bps", cli.fee_bps),
        ("--slip-bps", cli.slip_bps),
        ("--stress-fee-bps", cli.stress_fee_bps),
        ("--stress-slip-bps", cli.stress_slip_bps),
    ] {
        if value > 1000 {
            return Err(format!("{name} must be at most 1000"));
        }
    }

    let cfg = Config {
        rsi_period: cli.rsi_period,
        rsi_long_threshold: Decimal::from(cli.long_threshold),
        rsi_short_threshold: Decimal::from(cli.short_threshold),
        ..Config::default()
    };
    cfg.validate().map_err(|error| error.to_string())?;

    let generated_at = match &cli.generated_at {
        Some(value) => {
            chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
                format!("invalid --generated-at {value:?}; expected an RFC 3339 timestamp")
            })?;
            value.clone()
        }
        None => chrono::DateTime::<chrono::Utc>::from_timestamp_millis(identity::now_ms())
            .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default(),
    };

    let base_url = env_string("KUCOIN_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let settle_ms = match env_string("ANALYZER_CANDLE_SETTLE_MS") {
        Some(value) => value.parse::<i64>().map_err(|_| {
            format!("invalid ANALYZER_CANDLE_SETTLE_MS={value:?}: expected milliseconds")
        })?,
        None => DEFAULT_SETTLE_MS,
    };
    if settle_ms < 0 {
        return Err("ANALYZER_CANDLE_SETTLE_MS must be non-negative".into());
    }

    let base = CostScenario {
        label: "base".into(),
        fee_bps: Decimal::from(cli.fee_bps),
        slip_bps: Decimal::from(cli.slip_bps),
    };
    let stress = CostScenario {
        label: "stress".into(),
        fee_bps: Decimal::from(cli.stress_fee_bps),
        slip_bps: Decimal::from(cli.stress_slip_bps),
    };
    let start_ms = cli
        .as_of
        .saturating_sub(i64::from(cli.days).saturating_mul(DAY_MS));

    let mut errors = Vec::new();
    let mut results = Vec::new();
    for symbol in &symbols {
        match load_symbol(
            symbol,
            timeframe,
            start_ms,
            cli.as_of,
            settle_ms,
            cfg.window,
            &cli.data_dir,
            &base_url,
            cli.refresh,
        ) {
            Ok(dataset) => {
                if dataset.candles.len() < MIN_CANDLES {
                    errors.push(format!(
                        "{}: only {} closed candles (need >= {MIN_CANDLES})",
                        symbol,
                        dataset.candles.len()
                    ));
                    continue;
                }
                match evaluate_symbol(
                    &dataset,
                    &cfg,
                    &generated_at,
                    cli.as_of,
                    cli.days,
                    cli.init_fund,
                    &base,
                    &stress,
                ) {
                    Ok(result) => results.push(result),
                    Err(error) => errors.push(format!("{symbol}: evaluation failed: {error}")),
                }
            }
            Err(error) => errors.push(format!("{symbol}: fetch failed: {error}")),
        }
    }

    if results.is_empty() {
        print_summary(&results, &errors, &generated_at);
        return Err("no symbol produced a backtest result".into());
    }

    std::fs::create_dir_all(&cli.out_dir).map_err(|error| error.to_string())?;
    for result in &results {
        let dir = cli.out_dir.join(&result.symbol);
        write_symbol_report(&dir, result)?;
        write_leaderboard(&dir.join("leaderboard.csv"), result)?;
    }
    write_combined_index(&cli.out_dir, &results, &errors, &generated_at)?;
    print_summary(&results, &errors, &generated_at);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(
        net_expectancy: i64,
        net_profit_factor: Option<i64>,
        max_drawdown_pct: i64,
        trades: usize,
    ) -> MetricsView {
        MetricsView {
            trades,
            net_expectancy: Decimal::from(net_expectancy),
            net_profit_factor: net_profit_factor.map(Decimal::from),
            max_drawdown_pct: Decimal::from(max_drawdown_pct),
            expectancy_stderr: Decimal::ZERO,
            closed_net_pnl: Decimal::ZERO,
            open_net_pnl: Decimal::ZERO,
            net_pnl: Decimal::ZERO,
            win_rate: Decimal::ZERO,
        }
    }

    fn variant(
        label: &str,
        net_expectancy: i64,
        net_profit_factor: Option<i64>,
        max_drawdown_pct: i64,
        trades: usize,
    ) -> VariantResult {
        VariantResult {
            variant: AtrVariant {
                label: label.to_string(),
                enabled: true,
                period: ATR_PERIOD,
                sl_multiplier: Decimal::ONE,
                tp_multiplier: Decimal::ONE,
                pct_high: ATR_PCT_HIGH,
                high_adjust: ATR_HIGH_ADJUST,
                sl_max_pct: Decimal::ZERO,
            },
            train: metrics(net_expectancy, net_profit_factor, max_drawdown_pct, trades),
            top_rank: None,
        }
    }

    fn labels<'a>(selected: &[&'a VariantResult]) -> Vec<&'a str> {
        selected
            .iter()
            .map(|variant| variant.variant.label.as_str())
            .collect()
    }

    #[test]
    fn selection_uses_the_combined_pf_and_expectancy_rank() {
        let variants = vec![
            variant("pf-leader", 0, Some(10), 5, 20),
            variant("expectancy-leader", 20, Some(2), 5, 20),
            variant("balanced", 12, Some(4), 5, 20),
            variant("mid", 8, Some(3), 5, 20),
            variant("weak", -5, Some(1), 5, 20),
            variant("too-much-dd", 50, Some(20), 25, 20),
        ];
        let (selected, gate) = select_top(&variants, MAX_DRAWDOWN_PCT);
        assert!(gate.applied);
        assert_eq!(
            labels(&selected),
            vec!["balanced", "pf-leader", "expectancy-leader"]
        );
    }

    #[test]
    fn selection_keeps_an_infinite_pf_candidate_with_few_trades() {
        let variants = vec![
            variant("few-trades", 8, None, 5, 1),
            variant("many-trades", 2, Some(2), 5, 40),
            variant("moderate", 3, Some(3), 5, 10),
        ];
        let (selected, _) = select_top(&variants, MAX_DRAWDOWN_PCT);
        assert_eq!(selected[0].variant.label, "few-trades");
    }

    #[test]
    fn selection_applies_the_drawdown_gate_when_three_variants_qualify() {
        let variants = vec![
            variant("inside-a", 1, Some(2), 10, 10),
            variant("inside-b", 2, Some(3), 15, 10),
            variant("inside-c", 3, Some(4), 20, 10),
            variant("outside", 100, Some(100), 25, 10),
        ];
        let (selected, gate) = select_top(&variants, MAX_DRAWDOWN_PCT);
        assert!(gate.applied);
        assert_eq!(gate.qualified, 3);
        assert!(!labels(&selected).contains(&"outside"));
    }

    #[test]
    fn selection_falls_back_when_the_drawdown_gate_leaves_fewer_than_three() {
        let variants = vec![
            variant("inside", 1, Some(2), 10, 10),
            variant("outside-a", 2, Some(3), 25, 10),
            variant("outside-b", 3, Some(4), 30, 10),
            variant("outside-c", 4, Some(5), 35, 10),
        ];
        let (selected, gate) = select_top(&variants, MAX_DRAWDOWN_PCT);
        assert!(!gate.applied);
        assert_eq!(gate.qualified, 1);
        assert_eq!(selected.len(), 3);
        assert!(gate.note.contains("failed"));
    }
}
