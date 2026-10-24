use crate::harness::sweep::costs::CostScenario;
use crate::strategy::decimal::{add, div, mul, sub};
use crate::strategy::ts_momentum::{
    self, NegativeMomentumMode, TsDirection, TsMomentumConfig, TsSignalResult,
};
use crate::strategy::types::Candle;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const REGIME_SMA_DAYS: usize = 200;
pub const DAYS_PER_YEAR: f64 = 365.0;
const BPS: Decimal = Decimal::from_parts(10_000, 0, 0, false, 0);
pub const BACKTEST_SYMBOL: &str = "BTC-USDT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variant {
    Fixed,
    VolTarget,
    VolTargetDrawdown,
}

impl Variant {
    pub fn as_str(self) -> &'static str {
        match self {
            Variant::Fixed => "Fixed exposure",
            Variant::VolTarget => "Volatility targeting",
            Variant::VolTargetDrawdown => "Volatility targeting + drawdown control",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Variant::Fixed => "a-fixed",
            Variant::VolTarget => "b-vol-target",
            Variant::VolTargetDrawdown => "c-vol-target-drawdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Flat,
    Short,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Flat => "Flat",
            Mode::Short => "Short",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Mode::Flat => "flat",
            Mode::Short => "short",
        }
    }

    pub fn negative_momentum_mode(self) -> NegativeMomentumMode {
        match self {
            Mode::Flat => NegativeMomentumMode::Flat,
            Mode::Short => NegativeMomentumMode::Short,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsBacktestParams {
    pub init_fund: Decimal,
    pub fee_bps: Decimal,
    pub slip_bps: Decimal,
    pub borrow_bps_per_day: Decimal,
    pub max_drawdown: Decimal,
    pub drawdown_resume: Decimal,
    pub variant: Variant,
    pub mode: Mode,
    pub regime_filter: bool,
}

impl Default for TsBacktestParams {
    fn default() -> Self {
        Self {
            init_fund: Decimal::from(1_000),
            fee_bps: Decimal::from(10),
            slip_bps: Decimal::from(5),
            borrow_bps_per_day: Decimal::ZERO,
            max_drawdown: Decimal::new(20, 2),
            drawdown_resume: Decimal::new(10, 2),
            variant: Variant::Fixed,
            mode: Mode::Flat,
            regime_filter: false,
        }
    }
}

impl TsBacktestParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.init_fund <= Decimal::ZERO {
            return Err("init_fund must be positive".into());
        }
        if self.fee_bps < Decimal::ZERO || self.slip_bps < Decimal::ZERO {
            return Err("fee_bps and slip_bps must be non-negative".into());
        }
        if self.borrow_bps_per_day < Decimal::ZERO {
            return Err("borrow_bps_per_day must be non-negative".into());
        }
        if self.max_drawdown < Decimal::ZERO || self.max_drawdown > Decimal::ONE {
            return Err("max_drawdown must be in [0, 1]".into());
        }
        if self.drawdown_resume < Decimal::ZERO || self.drawdown_resume >= self.max_drawdown {
            return Err("drawdown_resume must be in [0, max_drawdown)".into());
        }
        Ok(())
    }

    fn per_side_rate(&self) -> Decimal {
        (self.fee_bps + self.slip_bps) / BPS
    }

    fn borrow_rate(&self) -> Decimal {
        self.borrow_bps_per_day / BPS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SplitName {
    Full,
    Train,
    Validation,
    Test,
}

impl SplitName {
    pub fn as_str(self) -> &'static str {
        match self {
            SplitName::Full => "full",
            SplitName::Train => "train",
            SplitName::Validation => "validation",
            SplitName::Test => "test",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Split {
    pub name: SplitName,
    pub start: usize,
    pub end: usize,
}

impl Split {
    pub fn full(n: usize) -> Self {
        Self {
            name: SplitName::Full,
            start: 0,
            end: n,
        }
    }

    pub fn candles<'a>(&self, candles: &'a [Candle]) -> &'a [Candle] {
        &candles[self.start.min(candles.len())..self.end.min(candles.len())]
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Splits {
    pub train: Split,
    pub validation: Split,
    pub test: Split,
}

pub fn splits(n: usize) -> Splits {
    let pct = |p: usize| n * p / 100;
    Splits {
        train: Split {
            name: SplitName::Train,
            start: 0,
            end: pct(60),
        },
        validation: Split {
            name: SplitName::Validation,
            start: pct(60),
            end: pct(80),
        },
        test: Split {
            name: SplitName::Test,
            start: pct(80),
            end: n,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TradeSide {
    Long,
    Short,
}

impl TradeSide {
    pub fn as_str(self) -> &'static str {
        match self {
            TradeSide::Long => "Long",
            TradeSide::Short => "Short",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TradeEpisode {
    pub side: TradeSide,
    pub entry_index: usize,
    pub exit_index: usize,
    pub entry_time_ms: i64,
    pub exit_time_ms: i64,
    pub pnl: Decimal,
    pub holding_days: usize,
    pub closed_at_end: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EquityPoint {
    pub open_time_ms: i64,
    pub equity: Decimal,
    pub exposure: Decimal,
    pub regime: Option<bool>,
    pub trading_cost: Decimal,
    pub borrow_cost: Decimal,
    pub traded_notional: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BacktestOutcome {
    pub points: Vec<EquityPoint>,
    pub trades: Vec<TradeEpisode>,
    pub trading_costs: Decimal,
    pub borrow_costs: Decimal,
    pub traded_notional: Decimal,
    pub start_index: usize,
    pub end_index: usize,
}

impl BacktestOutcome {
    pub fn equity_values(&self) -> Vec<Decimal> {
        self.points.iter().map(|point| point.equity).collect()
    }

    pub fn exposures(&self) -> Vec<Decimal> {
        self.points.iter().map(|point| point.exposure).collect()
    }

    pub fn regimes(&self) -> Vec<Option<bool>> {
        self.points.iter().map(|point| point.regime).collect()
    }

    pub fn turnover_ratio(&self, starting_equity: Decimal) -> f64 {
        if starting_equity <= Decimal::ZERO {
            return 0.0;
        }
        let value = self.traded_notional / starting_equity;
        value
            .to_f64()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn metrics(&self, starting_equity: Decimal) -> Metrics {
        metrics(
            starting_equity,
            &self.equity_values(),
            &self.exposures(),
            &self.regimes(),
            &self.trades,
            self.trading_costs,
            self.borrow_costs,
            self.turnover_ratio(starting_equity),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BenchmarkOutcome {
    pub points: Vec<EquityPoint>,
    pub trades: Vec<TradeEpisode>,
    pub trading_costs: Decimal,
    pub traded_notional: Decimal,
}

impl BenchmarkOutcome {
    pub fn equity_values(&self) -> Vec<Decimal> {
        self.points.iter().map(|point| point.equity).collect()
    }

    pub fn exposures(&self) -> Vec<Decimal> {
        self.points.iter().map(|point| point.exposure).collect()
    }

    pub fn regimes(&self) -> Vec<Option<bool>> {
        self.points.iter().map(|point| point.regime).collect()
    }

    pub fn turnover_ratio(&self, starting_equity: Decimal) -> f64 {
        if starting_equity <= Decimal::ZERO {
            return 0.0;
        }
        let value = self.traded_notional / starting_equity;
        value
            .to_f64()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn metrics(&self, starting_equity: Decimal) -> Metrics {
        metrics(
            starting_equity,
            &self.equity_values(),
            &self.exposures(),
            &self.regimes(),
            &self.trades,
            self.trading_costs,
            Decimal::ZERO,
            self.turnover_ratio(starting_equity),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegimeMetrics {
    pub days: usize,
    pub cumulative_return: Option<f64>,
    pub cagr: Option<f64>,
    pub sharpe: Option<f64>,
    pub sortino: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub avg_exposure: Option<f64>,
    pub avg_abs_exposure: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    pub days: usize,
    pub net_cumulative_return: Option<f64>,
    pub cagr: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub sharpe: Option<f64>,
    pub sortino: Option<f64>,
    pub win_rate: Option<f64>,
    pub trades: usize,
    pub avg_holding_days: Option<f64>,
    pub turnover: f64,
    pub total_trading_costs: Decimal,
    pub total_borrow_costs: Decimal,
    pub avg_exposure: Option<f64>,
    pub avg_abs_exposure: Option<f64>,
    pub exposure_days: usize,
    pub bull: RegimeMetrics,
    pub bear: RegimeMetrics,
    pub unknown_regime: RegimeMetrics,
}

pub fn regime_at(candles: &[Candle], index: usize, period: usize) -> Option<bool> {
    if period == 0 || index + 1 < period || index >= candles.len() {
        return None;
    }
    let start = index + 1 - period;
    let sum: Decimal = candles[start..=index]
        .iter()
        .map(|candle| candle.close)
        .sum();
    let sma = sum / Decimal::from(period);
    Some(candles[index].close > sma)
}

pub fn run(
    candles: &[Candle],
    cfg: &TsMomentumConfig,
    params: &TsBacktestParams,
) -> Result<BacktestOutcome, String> {
    run_range(candles, cfg, params, 0, candles.len())
}

pub fn run_range(
    candles: &[Candle],
    cfg: &TsMomentumConfig,
    params: &TsBacktestParams,
    start: usize,
    end: usize,
) -> Result<BacktestOutcome, String> {
    let mut cfg = cfg.clone();
    cfg.negative_momentum_mode = params.mode.negative_momentum_mode();
    cfg.validate().map_err(|error| error.to_string())?;
    params.validate()?;
    if candles.is_empty() {
        return Err("no candles to backtest".into());
    }
    if start >= end || end > candles.len() {
        return Err(format!(
            "invalid backtest range {start}..{end} for {} candles",
            candles.len()
        ));
    }

    let rate = params.per_side_rate();
    let borrow_rate = params.borrow_rate();
    let mut equity = params.init_fund;
    let mut exposure = Decimal::ZERO;
    let mut open_episode: Option<OpenEpisode> = None;
    let mut trades = Vec::new();
    let mut points = Vec::with_capacity(end - start);
    let mut trading_costs = Decimal::ZERO;
    let mut borrow_costs = Decimal::ZERO;
    let mut traded_notional = Decimal::ZERO;

    let mut shadow_enabled = params.variant == Variant::VolTargetDrawdown;
    let mut shadow_equity = params.init_fund;
    let mut shadow_exposure = Decimal::ZERO;
    let mut shadow_peak = params.init_fund;
    let mut guard = DrawdownGuard::new(params.max_drawdown, params.drawdown_resume);

    for index in start..end {
        let candle = &candles[index];
        let previous_close = if index > start {
            Some(candles[index - 1].close)
        } else {
            None
        };

        let gap_ratio = match previous_close {
            Some(previous_close) => {
                div(candle.open, previous_close).map_err(|error| error.to_string())?
            }
            None => Decimal::ONE,
        };
        let equity_after_gap = apply_return(equity, exposure, gap_ratio)?;

        let mut shadow_after_gap = shadow_equity;
        if shadow_enabled {
            shadow_after_gap = apply_return(shadow_equity, shadow_exposure, gap_ratio)?;
            guard.update(shadow_after_gap, shadow_peak);
        }

        let raw_target = signal_target(candles, index, &cfg, params)?;
        let target = if shadow_enabled && guard.halted() {
            Decimal::ZERO
        } else {
            raw_target
        };

        let mut day_cost = Decimal::ZERO;
        let mut day_notional = Decimal::ZERO;
        let mut day_borrow = Decimal::ZERO;

        let old_exposure = exposure;
        let old_sign = sign(old_exposure);
        let target_sign = sign(target);
        let crossed =
            old_exposure != Decimal::ZERO && (target == Decimal::ZERO || old_sign != target_sign);

        if crossed {
            let total_delta = sub(target, old_exposure)
                .map_err(|error| error.to_string())?
                .abs();
            let total_notional =
                mul(total_delta, equity_after_gap).map_err(|error| error.to_string())?;
            let total_cost = mul(rate, total_notional).map_err(|error| error.to_string())?;
            let old_cost = if total_delta == Decimal::ZERO {
                Decimal::ZERO
            } else {
                let old_share =
                    div(old_exposure.abs(), total_delta).map_err(|error| error.to_string())?;
                mul(total_cost, old_share).map_err(|error| error.to_string())?
            };
            let equity_after_old_exit =
                sub(equity_after_gap, old_cost).map_err(|error| error.to_string())?;
            if let Some(episode) = open_episode.take() {
                trades.push(episode.close(
                    equity_after_old_exit,
                    index,
                    candle.open_time_ms,
                    false,
                )?);
            }
            equity = sub(equity_after_gap, total_cost).map_err(|error| error.to_string())?;
            day_cost = add(day_cost, total_cost).map_err(|error| error.to_string())?;
            day_notional = add(day_notional, total_notional).map_err(|error| error.to_string())?;
            if target != Decimal::ZERO {
                open_episode = Some(OpenEpisode {
                    side: if target > Decimal::ZERO {
                        TradeSide::Long
                    } else {
                        TradeSide::Short
                    },
                    entry_index: index,
                    entry_time_ms: candle.open_time_ms,
                    baseline: equity_after_old_exit,
                });
            }
        } else if old_exposure == Decimal::ZERO && target != Decimal::ZERO {
            let entry_notional = mul(target.abs(), equity).map_err(|error| error.to_string())?;
            let entry_cost = mul(rate, entry_notional).map_err(|error| error.to_string())?;
            let baseline = equity;
            equity = sub(equity, entry_cost).map_err(|error| error.to_string())?;
            day_cost = add(day_cost, entry_cost).map_err(|error| error.to_string())?;
            day_notional = add(day_notional, entry_notional).map_err(|error| error.to_string())?;
            open_episode = Some(OpenEpisode {
                side: if target > Decimal::ZERO {
                    TradeSide::Long
                } else {
                    TradeSide::Short
                },
                entry_index: index,
                entry_time_ms: candle.open_time_ms,
                baseline,
            });
        } else if old_exposure != Decimal::ZERO && target != Decimal::ZERO {
            let delta = sub(target, old_exposure)
                .map_err(|error| error.to_string())?
                .abs();
            let delta_notional = mul(delta, equity_after_gap).map_err(|error| error.to_string())?;
            let delta_cost = mul(rate, delta_notional).map_err(|error| error.to_string())?;
            equity = sub(equity_after_gap, delta_cost).map_err(|error| error.to_string())?;
            day_cost = add(day_cost, delta_cost).map_err(|error| error.to_string())?;
            day_notional = add(day_notional, delta_notional).map_err(|error| error.to_string())?;
        } else {
            equity = equity_after_gap;
        }

        exposure = target;

        let intraday_ratio = div(candle.close, candle.open).map_err(|error| error.to_string())?;
        let mut equity_after_intraday = apply_return(equity, exposure, intraday_ratio)?;
        if exposure < Decimal::ZERO {
            let borrow_notional =
                mul(exposure.abs(), equity_after_intraday).map_err(|error| error.to_string())?;
            let borrow_cost =
                mul(borrow_rate, borrow_notional).map_err(|error| error.to_string())?;
            equity_after_intraday =
                sub(equity_after_intraday, borrow_cost).map_err(|error| error.to_string())?;
            day_borrow = borrow_cost;
        }
        equity = equity_after_intraday;

        if shadow_enabled {
            let shadow_delta = sub(raw_target, shadow_exposure)
                .map_err(|error| error.to_string())?
                .abs();
            let shadow_notional =
                mul(shadow_delta, shadow_after_gap).map_err(|error| error.to_string())?;
            let shadow_cost = mul(rate, shadow_notional).map_err(|error| error.to_string())?;
            shadow_equity =
                sub(shadow_after_gap, shadow_cost).map_err(|error| error.to_string())?;
            shadow_exposure = raw_target;
            let shadow_intraday =
                div(candle.close, candle.open).map_err(|error| error.to_string())?;
            shadow_equity = apply_return(shadow_equity, shadow_exposure, shadow_intraday)?;
            if shadow_exposure < Decimal::ZERO {
                let borrow_notional =
                    mul(shadow_exposure.abs(), shadow_equity).map_err(|error| error.to_string())?;
                let borrow_cost =
                    mul(borrow_rate, borrow_notional).map_err(|error| error.to_string())?;
                shadow_equity =
                    sub(shadow_equity, borrow_cost).map_err(|error| error.to_string())?;
            }
            if shadow_equity > shadow_peak {
                shadow_peak = shadow_equity;
            }
            if shadow_equity <= Decimal::ZERO {
                shadow_enabled = false;
            }
        }

        trading_costs = add(trading_costs, day_cost).map_err(|error| error.to_string())?;
        borrow_costs = add(borrow_costs, day_borrow).map_err(|error| error.to_string())?;
        traded_notional = add(traded_notional, day_notional).map_err(|error| error.to_string())?;

        points.push(EquityPoint {
            open_time_ms: candle.open_time_ms,
            equity,
            exposure,
            regime: regime_at(candles, index, REGIME_SMA_DAYS),
            trading_cost: day_cost,
            borrow_cost: day_borrow,
            traded_notional: day_notional,
        });
    }

    if let Some(episode) = open_episode.take() {
        trades.push(episode.close(equity, end, candles[end - 1].open_time_ms, true)?);
    }

    Ok(BacktestOutcome {
        points,
        trades,
        trading_costs,
        borrow_costs,
        traded_notional,
        start_index: start,
        end_index: end,
    })
}

struct OpenEpisode {
    side: TradeSide,
    entry_index: usize,
    entry_time_ms: i64,
    baseline: Decimal,
}

impl OpenEpisode {
    fn close(
        self,
        equity: Decimal,
        exit_index: usize,
        exit_time_ms: i64,
        closed_at_end: bool,
    ) -> Result<TradeEpisode, String> {
        let pnl = sub(equity, self.baseline).map_err(|error| error.to_string())?;
        Ok(TradeEpisode {
            side: self.side,
            entry_index: self.entry_index,
            exit_index,
            entry_time_ms: self.entry_time_ms,
            exit_time_ms,
            pnl,
            holding_days: exit_index.saturating_sub(self.entry_index),
            closed_at_end,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct DrawdownGuard {
    halted: bool,
    max_drawdown: Decimal,
    resume_drawdown: Decimal,
}

impl DrawdownGuard {
    fn new(max_drawdown: Decimal, resume_drawdown: Decimal) -> Self {
        Self {
            halted: false,
            max_drawdown,
            resume_drawdown,
        }
    }

    fn halted(&self) -> bool {
        self.halted
    }

    fn update(&mut self, equity: Decimal, peak: Decimal) {
        if peak <= Decimal::ZERO {
            return;
        }
        let drawdown = if equity >= peak {
            Decimal::ZERO
        } else {
            (peak - equity) / peak
        };
        if self.halted {
            if drawdown <= self.resume_drawdown {
                self.halted = false;
            }
        } else if drawdown >= self.max_drawdown {
            self.halted = true;
        }
    }
}

fn apply_return(
    equity: Decimal,
    exposure: Decimal,
    price_ratio: Decimal,
) -> Result<Decimal, String> {
    if exposure == Decimal::ZERO {
        return Ok(equity);
    }
    let change = sub(price_ratio, Decimal::ONE).map_err(|error| error.to_string())?;
    let pnl = mul(exposure, change).map_err(|error| error.to_string())?;
    let factor = add(Decimal::ONE, pnl).map_err(|error| error.to_string())?;
    mul(equity, factor).map_err(|error| error.to_string())
}

fn signal_target(
    candles: &[Candle],
    index: usize,
    cfg: &TsMomentumConfig,
    params: &TsBacktestParams,
) -> Result<Decimal, String> {
    if index == 0 {
        return Ok(Decimal::ZERO);
    }
    let signal = match ts_momentum::evaluate(&candles[..index], BACKTEST_SYMBOL, cfg)
        .map_err(|error| error.to_string())?
    {
        TsSignalResult::Signal(signal) => signal,
        TsSignalResult::NoSignal { .. } => return Ok(Decimal::ZERO),
    };
    let magnitude = match params.variant {
        Variant::Fixed => cfg.max_exposure.min(Decimal::ONE),
        Variant::VolTarget | Variant::VolTargetDrawdown => signal.desired_exposure,
    };
    let target = if params.regime_filter {
        match (signal.direction, regime_at(candles, index, REGIME_SMA_DAYS)) {
            (TsDirection::Long, Some(true)) => magnitude,
            (TsDirection::Short, Some(false)) => -magnitude,
            _ => Decimal::ZERO,
        }
    } else {
        match signal.direction {
            TsDirection::Long => magnitude,
            TsDirection::Short => -magnitude,
            TsDirection::Flat => Decimal::ZERO,
        }
    };
    let cap = cfg.max_exposure.max(Decimal::ZERO).min(Decimal::ONE);
    Ok(target.max(-cap).min(cap))
}

fn sign(value: Decimal) -> i8 {
    if value > Decimal::ZERO {
        1
    } else if value < Decimal::ZERO {
        -1
    } else {
        0
    }
}

#[allow(clippy::too_many_arguments)]
pub fn metrics(
    starting_equity: Decimal,
    equity: &[Decimal],
    exposures: &[Decimal],
    regime: &[Option<bool>],
    trades: &[TradeEpisode],
    trading_costs: Decimal,
    borrow_costs: Decimal,
    turnover: f64,
) -> Metrics {
    let start = positive_f64(starting_equity);
    let equity_values: Vec<f64> = equity
        .iter()
        .map(|value| decimal_f64(*value).unwrap_or(0.0))
        .collect();
    let returns = daily_returns(start, &equity_values);
    let net_cumulative_return = match (start, equity_values.last().copied()) {
        (Some(start), Some(last)) if start > 0.0 => finite(last / start - 1.0),
        _ => None,
    };
    let days = equity.len();
    let cagr = net_cumulative_return.and_then(|total| cagr(total, days));
    let max_drawdown = drawdown_from_equity(start, &equity_values);

    let exposure_values: Vec<Option<f64>> =
        exposures.iter().map(|value| decimal_f64(*value)).collect();
    let avg_exposure = mean_option(&exposure_values);
    let avg_abs_exposure = mean_option(
        &exposure_values
            .iter()
            .map(|value| value.map(f64::abs))
            .collect::<Vec<_>>(),
    );
    let exposure_days = exposures
        .iter()
        .filter(|value| **value != Decimal::ZERO)
        .count();

    let wins = trades
        .iter()
        .filter(|trade| trade.pnl > Decimal::ZERO)
        .count();
    let win_rate = if trades.is_empty() {
        None
    } else {
        finite(wins as f64 / trades.len() as f64)
    };
    let avg_holding_days = mean_option(
        &trades
            .iter()
            .map(|trade| Some(trade.holding_days as f64))
            .collect::<Vec<_>>(),
    );

    let bull = regime_metrics(&returns, regime, &exposure_values, Some(true));
    let bear = regime_metrics(&returns, regime, &exposure_values, Some(false));
    let unknown_regime = regime_metrics(&returns, regime, &exposure_values, None);

    Metrics {
        days,
        net_cumulative_return,
        cagr,
        max_drawdown,
        sharpe: sharpe(&returns),
        sortino: sortino(&returns),
        win_rate,
        trades: trades.len(),
        avg_holding_days,
        turnover: finite(turnover).unwrap_or(0.0),
        total_trading_costs: trading_costs,
        total_borrow_costs: borrow_costs,
        avg_exposure,
        avg_abs_exposure,
        exposure_days,
        bull,
        bear,
        unknown_regime,
    }
}

fn regime_metrics(
    returns: &[f64],
    regime: &[Option<bool>],
    exposures: &[Option<f64>],
    label: Option<bool>,
) -> RegimeMetrics {
    let mut selected = Vec::new();
    let mut selected_exposures = Vec::new();
    for index in 0..returns.len().min(regime.len()) {
        if regime[index] == label {
            selected.push(returns[index]);
            selected_exposures.push(exposures.get(index).copied().flatten());
        }
    }
    if selected.is_empty() {
        return RegimeMetrics {
            days: 0,
            cumulative_return: None,
            cagr: None,
            sharpe: None,
            sortino: None,
            max_drawdown: None,
            avg_exposure: None,
            avg_abs_exposure: None,
        };
    }
    let cumulative_return = cumulative_return(&selected);
    RegimeMetrics {
        days: selected.len(),
        cumulative_return,
        cagr: cumulative_return.and_then(|total| cagr(total, selected.len())),
        sharpe: sharpe(&selected),
        sortino: sortino(&selected),
        max_drawdown: drawdown_from_returns(&selected),
        avg_exposure: mean_option(&selected_exposures),
        avg_abs_exposure: mean_option(
            &selected_exposures
                .iter()
                .map(|value| value.map(f64::abs))
                .collect::<Vec<_>>(),
        ),
    }
}

pub fn benchmark(
    candles: &[Candle],
    init_fund: Decimal,
    scenario: &CostScenario,
    split: Split,
) -> Result<BenchmarkOutcome, String> {
    if init_fund <= Decimal::ZERO {
        return Err("benchmark init_fund must be positive".into());
    }
    if split.start >= split.end || split.end > candles.len() {
        return Err(format!(
            "invalid benchmark range {}..{} for {} candles",
            split.start,
            split.end,
            candles.len()
        ));
    }

    let rate = scenario.per_side_rate();
    let mut equity = init_fund;
    let mut trading_costs = Decimal::ZERO;
    let mut traded_notional = Decimal::ZERO;
    let mut points = Vec::with_capacity(split.len());

    for index in split.start..split.end {
        let candle = &candles[index];
        let mut day_cost = Decimal::ZERO;
        let mut day_notional = Decimal::ZERO;

        if index == split.start {
            let entry_notional = equity;
            let entry_cost = mul(rate, entry_notional).map_err(|error| error.to_string())?;
            equity = sub(equity, entry_cost).map_err(|error| error.to_string())?;
            day_cost = add(day_cost, entry_cost).map_err(|error| error.to_string())?;
            day_notional = add(day_notional, entry_notional).map_err(|error| error.to_string())?;
        } else {
            let gap =
                div(candle.open, candles[index - 1].close).map_err(|error| error.to_string())?;
            equity = mul(equity, gap).map_err(|error| error.to_string())?;
        }

        let intraday = div(candle.close, candle.open).map_err(|error| error.to_string())?;
        equity = mul(equity, intraday).map_err(|error| error.to_string())?;

        if index + 1 == split.end {
            let exit_notional = equity;
            let exit_cost = mul(rate, exit_notional).map_err(|error| error.to_string())?;
            equity = sub(equity, exit_cost).map_err(|error| error.to_string())?;
            day_cost = add(day_cost, exit_cost).map_err(|error| error.to_string())?;
            day_notional = add(day_notional, exit_notional).map_err(|error| error.to_string())?;
        }

        trading_costs = add(trading_costs, day_cost).map_err(|error| error.to_string())?;
        traded_notional = add(traded_notional, day_notional).map_err(|error| error.to_string())?;
        points.push(EquityPoint {
            open_time_ms: candle.open_time_ms,
            equity,
            exposure: Decimal::ONE,
            regime: regime_at(candles, index, REGIME_SMA_DAYS),
            trading_cost: day_cost,
            borrow_cost: Decimal::ZERO,
            traded_notional: day_notional,
        });
    }

    let pnl = sub(equity, init_fund).map_err(|error| error.to_string())?;
    let trade = TradeEpisode {
        side: TradeSide::Long,
        entry_index: split.start,
        exit_index: split.end,
        entry_time_ms: candles[split.start].open_time_ms,
        exit_time_ms: candles[split.end - 1].open_time_ms,
        pnl,
        holding_days: split.len(),
        closed_at_end: true,
    };

    Ok(BenchmarkOutcome {
        points,
        trades: vec![trade],
        trading_costs,
        traded_notional,
    })
}

fn positive_f64(value: Decimal) -> Option<f64> {
    value.to_f64().filter(|value| *value > 0.0)
}

fn decimal_f64(value: Decimal) -> Option<f64> {
    value.to_f64().filter(|value| value.is_finite())
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn daily_returns(start: Option<f64>, equity: &[f64]) -> Vec<f64> {
    let mut returns = Vec::with_capacity(equity.len());
    let mut previous = start;
    for value in equity {
        let current = *value;
        let result = match previous {
            Some(previous) if previous > 0.0 && current.is_finite() => {
                let result = current / previous - 1.0;
                if result.is_finite() {
                    result
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        returns.push(result);
        previous = Some(current);
    }
    returns
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    finite(sum / values.len() as f64)
}

fn mean_option(values: &[Option<f64>]) -> Option<f64> {
    let present: Vec<f64> = values.iter().filter_map(|value| *value).collect();
    mean(&present)
}

fn sample_std(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = mean(values)?;
    let sum: f64 = values
        .iter()
        .map(|value| {
            let difference = value - mean;
            difference * difference
        })
        .sum();
    let variance = sum / (values.len() - 1) as f64;
    variance
        .is_finite()
        .then(|| variance.sqrt())
        .filter(|value| *value > 0.0)
}

fn sharpe(returns: &[f64]) -> Option<f64> {
    let mean = mean(returns)?;
    let std = sample_std(returns)?;
    finite(mean / std * DAYS_PER_YEAR.sqrt())
}

fn sortino(returns: &[f64]) -> Option<f64> {
    let mean = mean(returns)?;
    let downside: f64 = returns
        .iter()
        .map(|value| {
            let negative = value.min(0.0);
            negative * negative
        })
        .sum::<f64>()
        / returns.len() as f64;
    if downside <= 0.0 || !downside.is_finite() {
        return None;
    }
    finite(mean / downside.sqrt() * DAYS_PER_YEAR.sqrt())
}

fn cumulative_return(returns: &[f64]) -> Option<f64> {
    let mut growth = 1.0_f64;
    for value in returns {
        growth *= 1.0 + value;
        if !growth.is_finite() {
            return None;
        }
    }
    finite(growth - 1.0)
}

fn cagr(total_return: f64, days: usize) -> Option<f64> {
    if days == 0 || total_return <= -1.0 {
        return None;
    }
    let years = days as f64 / DAYS_PER_YEAR;
    let growth = 1.0 + total_return;
    if growth <= 0.0 || years <= 0.0 {
        return None;
    }
    finite(growth.powf(1.0 / years) - 1.0)
}

fn drawdown_from_equity(start: Option<f64>, equity: &[f64]) -> Option<f64> {
    let mut peak = start?;
    let mut max_drawdown = 0.0_f64;
    for value in equity {
        if *value > peak {
            peak = *value;
        }
        if peak <= 0.0 {
            return None;
        }
        let drawdown = (peak - value) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }
    finite(max_drawdown)
}

fn drawdown_from_returns(returns: &[f64]) -> Option<f64> {
    let mut equity = 1.0_f64;
    let mut peak = 1.0_f64;
    let mut max_drawdown = 0.0_f64;
    for value in returns {
        equity *= 1.0 + value;
        if !equity.is_finite() {
            return None;
        }
        if equity > peak {
            peak = equity;
        }
        if peak <= 0.0 {
            return None;
        }
        let drawdown = (peak - equity) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }
    finite(max_drawdown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::ts_momentum::TsMomentumConfig;
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal::Decimal;

    const DAY_MS: i64 = 86_400_000;

    fn d(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    fn candle(index: usize, open: Decimal, close: Decimal) -> Candle {
        Candle {
            open_time_ms: index as i64 * DAY_MS,
            open,
            high: open.max(close),
            low: open.min(close),
            close,
            volume: Decimal::ONE,
        }
    }

    fn closes_to_candles(closes: &[Decimal]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(index, close)| {
                let open = if index == 0 {
                    *close
                } else {
                    closes[index - 1]
                };
                candle(index, open, *close)
            })
            .collect()
    }

    fn trend_closes(count: usize, daily_return: f64) -> Vec<Decimal> {
        let mut price = 100.0_f64;
        let mut closes = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                let noise = if index % 2 == 0 { 0.0005 } else { -0.0005 };
                price *= 1.0 + daily_return + noise;
            }
            closes.push(Decimal::from_f64(price).expect("finite price"));
        }
        closes
    }

    fn params(variant: Variant, mode: Mode) -> TsBacktestParams {
        TsBacktestParams {
            variant,
            mode,
            fee_bps: Decimal::ZERO,
            slip_bps: Decimal::ZERO,
            ..TsBacktestParams::default()
        }
    }

    fn approx(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn splits_are_sixty_twenty_twenty() {
        let splits = splits(1_000);
        assert_eq!((splits.train.start, splits.train.end), (0, 600));
        assert_eq!((splits.validation.start, splits.validation.end), (600, 800));
        assert_eq!((splits.test.start, splits.test.end), (800, 1_000));
    }

    #[test]
    fn next_open_execution_does_not_capture_the_signal_gap() {
        let mut closes = trend_closes(33, 0.001);
        closes[31] = closes[30];
        closes[32] = closes[30] * d("1.65");
        let mut candles = closes_to_candles(&closes);
        candles[31].open = closes[30] * d("1.50");
        candles[31].close = candles[31].open;
        candles[31].high = candles[31].open;
        candles[31].low = candles[31].open;
        candles[32].open = candles[31].close;

        let cfg = TsMomentumConfig::default();
        let outcome = run(&candles, &cfg, &params(Variant::Fixed, Mode::Flat)).unwrap();
        approx(outcome.points[30].exposure.to_f64().unwrap(), 0.0, 1e-9);
        approx(outcome.points[31].exposure.to_f64().unwrap(), 1.0, 1e-9);
        approx(outcome.points[30].equity.to_f64().unwrap(), 1_000.0, 1e-6);
        approx(outcome.points[31].equity.to_f64().unwrap(), 1_000.0, 1e-6);
        approx(outcome.points[32].equity.to_f64().unwrap(), 1_100.0, 1e-6);
    }

    #[test]
    fn costs_and_turnover_are_charged_on_traded_notional() {
        let closes = trend_closes(32, 0.001);
        let mut candles = closes_to_candles(&closes);
        let last = candles.len() - 1;
        candles[last].open = closes[last - 1];
        candles[last].close = closes[last - 1];
        candles[last].high = candles[last].open;
        candles[last].low = candles[last].open;

        let cfg = TsMomentumConfig::default();
        let mut p = params(Variant::Fixed, Mode::Flat);
        p.fee_bps = Decimal::from(10);
        p.slip_bps = Decimal::from(5);
        let outcome = run(&candles, &cfg, &p).unwrap();

        approx(outcome.trading_costs.to_f64().unwrap(), 1.5, 1e-9);
        approx(outcome.traded_notional.to_f64().unwrap(), 1_000.0, 1e-9);
        approx(outcome.turnover_ratio(d("1000")), 1.0, 1e-9);
        approx(outcome.points[last].equity.to_f64().unwrap(), 998.5, 1e-9);
    }

    #[test]
    fn flat_mode_never_shorts_and_short_mode_can() {
        let closes = trend_closes(64, -0.002);
        let candles = closes_to_candles(&closes);

        let flat_cfg = TsMomentumConfig::default();
        let flat = run(&candles, &flat_cfg, &params(Variant::Fixed, Mode::Flat)).unwrap();
        assert!(flat
            .points
            .iter()
            .all(|point| point.exposure >= Decimal::ZERO));
        assert!(flat
            .points
            .iter()
            .all(|point| point.exposure == Decimal::ZERO));
        assert!(flat.trades.is_empty());

        let short_cfg = TsMomentumConfig {
            negative_momentum_mode: NegativeMomentumMode::Short,
            ..TsMomentumConfig::default()
        };
        let short = run(&candles, &short_cfg, &params(Variant::Fixed, Mode::Short)).unwrap();
        assert!(short
            .points
            .iter()
            .any(|point| point.exposure < Decimal::ZERO));
        assert!(!short.trades.is_empty());
        assert!(short
            .trades
            .iter()
            .all(|trade| trade.side == TradeSide::Short));

        // `params.mode` is authoritative even when the caller passes a config
        // that still says Flat.
        let short_from_flat_cfg =
            run(&candles, &flat_cfg, &params(Variant::Fixed, Mode::Short)).unwrap();
        assert!(short_from_flat_cfg
            .points
            .iter()
            .any(|point| point.exposure < Decimal::ZERO));

        let mut borrowed_params = params(Variant::Fixed, Mode::Short);
        borrowed_params.borrow_bps_per_day = Decimal::from(10);
        let borrowed = run(&candles, &short_cfg, &borrowed_params).unwrap();
        assert_eq!(short.borrow_costs, Decimal::ZERO);
        assert!(borrowed.borrow_costs > Decimal::ZERO);
        assert!(borrowed.metrics(d("1000")).total_borrow_costs > Decimal::ZERO);
    }

    #[test]
    fn regime_filter_blocks_longs_in_bear_regime_and_allows_them_in_bull_regime() {
        let mut bear_closes = trend_closes(220, -0.004);
        let mut price = bear_closes.last().unwrap().to_f64().unwrap();
        for index in 0..31 {
            let noise = if index % 2 == 0 { 0.0005 } else { -0.0005 };
            price *= 1.0 + 0.005 + noise;
            bear_closes.push(Decimal::from_f64(price).unwrap());
        }
        let bear_candles = closes_to_candles(&bear_closes);
        assert_eq!(
            regime_at(&bear_candles, 250, REGIME_SMA_DAYS),
            Some(false),
            "test fixture should be in a bear regime"
        );

        let cfg = TsMomentumConfig::default();
        let unfiltered = run(&bear_candles, &cfg, &params(Variant::Fixed, Mode::Flat)).unwrap();
        assert!(
            unfiltered.points[250].exposure > Decimal::ZERO,
            "the unfiltered signal should be long in the bear fixture"
        );

        let mut filtered_params = params(Variant::Fixed, Mode::Flat);
        filtered_params.regime_filter = true;
        let filtered = run(&bear_candles, &cfg, &filtered_params).unwrap();
        assert_eq!(filtered.points[250].exposure, Decimal::ZERO);

        let bull_candles = closes_to_candles(&trend_closes(251, 0.002));
        assert_eq!(
            regime_at(&bull_candles, 250, REGIME_SMA_DAYS),
            Some(true),
            "test fixture should be in a bull regime"
        );
        let bull = run(&bull_candles, &cfg, &filtered_params).unwrap();
        assert!(bull.points[250].exposure > Decimal::ZERO);
    }

    #[test]
    fn drawdown_control_halts_then_resumes_after_recovery() {
        let mut returns = Vec::new();
        returns.push(0.0);
        for index in 0..60 {
            returns.push(0.010 + if index % 2 == 0 { 0.0005 } else { -0.0005 });
        }
        returns.push(-0.220);
        for index in 0..60 {
            returns.push(0.015 + if index % 2 == 0 { 0.0005 } else { -0.0005 });
        }
        let mut closes = Vec::with_capacity(returns.len());
        let mut price = 100.0_f64;
        for daily_return in returns {
            price *= 1.0 + daily_return;
            closes.push(Decimal::from_f64(price).expect("finite price"));
        }
        let candles = closes_to_candles(&closes);

        let cfg = TsMomentumConfig {
            target_annualized_volatility: d("10"),
            ..TsMomentumConfig::default()
        };
        let mut base_params = params(Variant::VolTarget, Mode::Flat);
        base_params.max_drawdown = d("0.20");
        base_params.drawdown_resume = d("0.10");
        let mut guard_params = base_params.clone();
        guard_params.variant = Variant::VolTargetDrawdown;

        let base = run(&candles, &cfg, &base_params).unwrap();
        let guarded = run(&candles, &cfg, &guard_params).unwrap();
        let halt_index = base
            .points
            .iter()
            .zip(&guarded.points)
            .position(|(base, guarded)| {
                base.exposure > Decimal::ZERO && guarded.exposure == Decimal::ZERO
            });
        let halt_index = halt_index.expect("expected the guard to force a flat period");
        let resumed = guarded.points[halt_index + 1..]
            .iter()
            .any(|point| point.exposure > Decimal::ZERO);
        assert!(resumed, "expected the guard to resume trading");

        let base_metrics = base.metrics(d("1000"));
        let guarded_metrics = guarded.metrics(d("1000"));
        assert!(
            guarded_metrics.max_drawdown.unwrap_or(1.0)
                <= base_metrics.max_drawdown.unwrap_or(1.0) + 1e-9,
            "guard should not increase max drawdown"
        );
    }

    #[test]
    fn metrics_compute_returns_drawdown_and_trade_stats() {
        let equity = vec![d("100"), d("110"), d("99"), d("120")];
        let exposures = vec![d("1"), d("1"), d("0"), d("0")];
        let regime = vec![Some(true), Some(true), Some(false), None];
        let trades = vec![
            TradeEpisode {
                side: TradeSide::Long,
                entry_index: 0,
                exit_index: 2,
                entry_time_ms: 0,
                exit_time_ms: 2 * DAY_MS,
                pnl: d("10"),
                holding_days: 2,
                closed_at_end: false,
            },
            TradeEpisode {
                side: TradeSide::Long,
                entry_index: 2,
                exit_index: 4,
                entry_time_ms: 2 * DAY_MS,
                exit_time_ms: 4 * DAY_MS,
                pnl: d("-5"),
                holding_days: 2,
                closed_at_end: true,
            },
        ];

        let metrics = metrics(
            d("100"),
            &equity,
            &exposures,
            &regime,
            &trades,
            d("1.5"),
            d("0.5"),
            2.0,
        );

        approx(metrics.net_cumulative_return.unwrap(), 0.20, 1e-9);
        approx(metrics.max_drawdown.unwrap(), 0.10, 1e-9);
        approx(metrics.win_rate.unwrap(), 0.5, 1e-9);
        assert_eq!(metrics.trades, 2);
        approx(metrics.avg_holding_days.unwrap(), 2.0, 1e-9);
        approx(metrics.avg_abs_exposure.unwrap(), 0.5, 1e-9);
        approx(metrics.turnover, 2.0, 1e-9);
        assert_eq!(metrics.total_trading_costs, d("1.5"));
        assert_eq!(metrics.total_borrow_costs, d("0.5"));
        assert_eq!(metrics.bull.days, 2);
        assert_eq!(metrics.bear.days, 1);
        assert_eq!(metrics.unknown_regime.days, 1);
    }

    #[test]
    fn benchmark_charges_one_entry_and_one_exit() {
        let closes = trend_closes(10, 0.01);
        let candles = closes_to_candles(&closes);
        let scenario = CostScenario {
            label: "base".into(),
            fee_bps: Decimal::from(10),
            slip_bps: Decimal::from(5),
        };
        let split = Split {
            name: SplitName::Full,
            start: 0,
            end: candles.len(),
        };
        let benchmark = benchmark(&candles, d("1000"), &scenario, split).unwrap();

        assert_eq!(benchmark.trades.len(), 1);
        assert_eq!(benchmark.trades[0].side, TradeSide::Long);
        let expected_after_entry = 1_000.0 * (1.0 - 0.0015);
        let price_ratio =
            candles.last().unwrap().close.to_f64().unwrap() / candles[0].open.to_f64().unwrap();
        let expected_before_exit = expected_after_entry * price_ratio;
        let expected = expected_before_exit * (1.0 - 0.0015);
        approx(
            benchmark.points.last().unwrap().equity.to_f64().unwrap(),
            expected,
            1e-6,
        );
        approx(
            benchmark.trading_costs.to_f64().unwrap(),
            1_000.0 * 0.0015 + expected_before_exit * 0.0015,
            1e-6,
        );
    }
}
