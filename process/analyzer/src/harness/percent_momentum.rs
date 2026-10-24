use crate::strategy::types::Candle;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::Serialize;

const TRADING_DAYS_PER_YEAR: f64 = 365.0;

/// Percentage mean-reversion rule: compare C1, the close `lookback` days
/// earlier, with C2, the current close, and use a fractional threshold
/// (0.05 = 5%). A prior price above C2 targets a long; below C2 targets a short.
#[derive(Debug, Clone, Serialize)]
pub struct PercentMomentumConfig {
    pub lookback: usize,
    pub threshold: Decimal,
}

#[derive(Debug, Clone)]
pub struct PercentMomentumParams {
    pub init_fund: Decimal,
    pub fee_bps: Decimal,
    pub slip_bps: Decimal,
}

impl PercentMomentumParams {
    /// Per-side fee plus slippage as a fraction (bps / 10_000).
    pub fn per_side_rate(&self) -> Decimal {
        (self.fee_bps + self.slip_bps) / Decimal::from(10_000)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    /// +1 for long, -1 for short.
    pub side: i8,
    pub entry_index: usize,
    pub exit_index: usize,
    pub entry_time_ms: i64,
    pub exit_time_ms: i64,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub pnl: Decimal,
    pub return_pct: Decimal,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PercentMomentumMetrics {
    pub total_return_pct: Decimal,
    pub cagr_pct: Decimal,
    pub max_drawdown_pct: Decimal,
    pub sharpe: Decimal,
    pub sortino: Decimal,
    pub win_rate_pct: Decimal,
    pub trades: usize,
    pub avg_holding_days: Decimal,
    pub turnover: Decimal,
    pub costs: Decimal,
    /// Mean of |position| over the simulated days; 1.0 means 100% exposure.
    pub avg_abs_exposure: Decimal,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    /// Daily close equity, preceded by the initial equity at the start open.
    pub equity: Vec<Decimal>,
    /// Target position after each day's open execution.
    pub positions: Vec<i8>,
    pub trades: Vec<Trade>,
    pub turnover: Decimal,
    pub costs: Decimal,
    pub metrics: PercentMomentumMetrics,
}

#[derive(Debug, Clone)]
pub struct BenchmarkOutcome {
    pub equity: Vec<Decimal>,
    pub costs: Decimal,
    pub metrics: PercentMomentumMetrics,
}

/// Simulates the percentage mean-reversion rule on `candles[start..end]`.
///
/// C2 is the close of day `t - 1` and C1 is the close `lookback` days earlier.
/// The signal is executed at the open of day `t`. Overnight moves accrue at the
/// old position, costs are charged on turnover, and the intraday move accrues
/// at the new position. Any position still open at the end is liquidated at
/// the final close so the result is net of round-trip costs.
pub fn simulate(
    candles: &[Candle],
    start: usize,
    end: usize,
    cfg: &PercentMomentumConfig,
    params: &PercentMomentumParams,
) -> Result<Outcome, String> {
    validate_range(candles, start, end)?;
    if cfg.lookback == 0 {
        return Err("lookback must be at least 1".into());
    }
    if cfg.threshold < Decimal::ZERO {
        return Err("threshold must be non-negative".into());
    }
    if params.init_fund <= Decimal::ZERO {
        return Err("init_fund must be positive".into());
    }

    let rate = params.per_side_rate();
    let mut equity = params.init_fund;
    let mut curve = Vec::with_capacity(end - start + 1);
    curve.push(equity);
    let mut positions = Vec::with_capacity(end - start);
    let mut trades: Vec<Trade> = Vec::new();
    let mut open_trade: Option<OpenTrade> = None;
    let mut position: i8 = 0;
    let mut costs = Decimal::ZERO;
    let mut turnover = Decimal::ZERO;
    let mut exposure_days = Decimal::ZERO;

    for index in start..end {
        let candle = &candles[index];

        // The position carried overnight earns the gap from the previous close
        // to this open before any new signal is executed.
        if index > start && position != 0 {
            let previous_close = candles[index - 1].close;
            if previous_close.is_zero() {
                return Err(format!("zero close at candle {index}"));
            }
            let gap = candle.open / previous_close - Decimal::ONE;
            equity *= Decimal::ONE + Decimal::from(i32::from(position)) * gap;
        }

        if let Some(new_position) = signal_target(candles, index, cfg) {
            if new_position != position {
                let old_position = position;
                let equity_at_open = equity;
                let exit_cost =
                    Decimal::from(u32::from(old_position.unsigned_abs())) * equity_at_open * rate;
                let entry_cost =
                    Decimal::from(u32::from(new_position.unsigned_abs())) * equity_at_open * rate;
                let total_cost = exit_cost + entry_cost;

                equity = equity_at_open - total_cost;
                costs += total_cost;
                turnover += Decimal::from(
                    (i32::from(new_position) - i32::from(old_position)).unsigned_abs(),
                );

                if old_position != 0 {
                    if let Some(trade) = open_trade.take() {
                        // The old trade bears the exit leg only; the new entry
                        // cost belongs to the replacement position.
                        close_trade(
                            &mut trades,
                            trade,
                            equity_at_open - exit_cost,
                            index,
                            candle.open,
                            candle.open_time_ms,
                        );
                    }
                }
                if new_position != 0 {
                    open_trade = Some(OpenTrade {
                        side: new_position,
                        entry_equity: equity,
                        entry_index: index,
                        entry_price: candle.open,
                        entry_time_ms: candle.open_time_ms,
                    });
                }
                position = new_position;
            }
        }

        if position != 0 {
            if candle.open.is_zero() {
                return Err(format!("zero open at candle {index}"));
            }
            let intraday = candle.close / candle.open - Decimal::ONE;
            equity *= Decimal::ONE + Decimal::from(i32::from(position)) * intraday;
        }

        exposure_days += Decimal::from(u32::from(position.unsigned_abs()));
        positions.push(position);
        curve.push(equity);
    }

    // Close any remaining position at the final close so every partition
    // reports a complete round trip net of costs.
    if position != 0 {
        let last = &candles[end - 1];
        let abs_position = Decimal::from(u32::from(position.unsigned_abs()));
        let exit_cost = abs_position * equity * rate;
        equity -= exit_cost;
        costs += exit_cost;
        turnover += abs_position;
        if let Some(trade) = open_trade.take() {
            close_trade(
                &mut trades,
                trade,
                equity,
                end - 1,
                last.close,
                last.open_time_ms,
            );
        }
        if let Some(last_equity) = curve.last_mut() {
            *last_equity = equity;
        }
    }

    let metrics = metrics_from_curve(
        &curve,
        &trades,
        costs,
        turnover,
        exposure_days,
        params.init_fund,
        end - start,
    );
    Ok(Outcome {
        equity: curve,
        positions,
        trades,
        turnover,
        costs,
        metrics,
    })
}

/// Buy-and-hold over `candles[start..end]` with one entry and one exit cost.
pub fn buy_and_hold(
    candles: &[Candle],
    start: usize,
    end: usize,
    params: &PercentMomentumParams,
) -> Result<BenchmarkOutcome, String> {
    validate_range(candles, start, end)?;
    if params.init_fund <= Decimal::ZERO {
        return Err("init_fund must be positive".into());
    }

    let rate = params.per_side_rate();
    let first = &candles[start];
    if first.open.is_zero() {
        return Err(format!("zero open at candle {start}"));
    }

    let mut equity = params.init_fund;
    let mut curve = Vec::with_capacity(end - start + 1);
    curve.push(equity);
    let entry_cost = equity * rate;
    equity -= entry_cost;
    let mut costs = entry_cost;

    for index in start..end {
        let candle = &candles[index];
        if index > start {
            let previous_close = candles[index - 1].close;
            if previous_close.is_zero() {
                return Err(format!("zero close at candle {index}"));
            }
            equity *= candle.open / previous_close;
        }
        if candle.open.is_zero() {
            return Err(format!("zero open at candle {index}"));
        }
        equity *= candle.close / candle.open;
        curve.push(equity);
    }

    let last = &candles[end - 1];
    let exit_cost = equity * rate;
    equity -= exit_cost;
    costs += exit_cost;
    if let Some(last_equity) = curve.last_mut() {
        *last_equity = equity;
    }

    let pnl = equity - params.init_fund;
    let return_pct = if params.init_fund.is_zero() {
        Decimal::ZERO
    } else {
        pnl / params.init_fund * Decimal::from(100)
    };
    let trade = Trade {
        side: 1,
        entry_index: start,
        exit_index: end - 1,
        entry_time_ms: first.open_time_ms,
        exit_time_ms: last.open_time_ms,
        entry_price: first.open,
        exit_price: last.close,
        pnl,
        return_pct,
    };
    let metrics = metrics_from_curve(
        &curve,
        &[trade],
        costs,
        Decimal::from(2),
        Decimal::from(end - start),
        params.init_fund,
        end - start,
    );
    Ok(BenchmarkOutcome {
        equity: curve,
        costs,
        metrics,
    })
}

#[derive(Debug, Clone)]
struct OpenTrade {
    side: i8,
    entry_equity: Decimal,
    entry_index: usize,
    entry_price: Decimal,
    entry_time_ms: i64,
}

fn validate_range(candles: &[Candle], start: usize, end: usize) -> Result<(), String> {
    if start >= end {
        return Err(format!(
            "invalid range: start {start} must be before end {end}"
        ));
    }
    if end > candles.len() {
        return Err(format!("range end {end} exceeds {} candles", candles.len()));
    }
    Ok(())
}

/// Returns the target position at `index`'s open using the previous close.
/// C2 is the previous close (day `t - 1`) and C1 is the close `lookback` days
/// earlier. `None` means the signal is neutral and the current position is held.
fn signal_target(candles: &[Candle], index: usize, cfg: &PercentMomentumConfig) -> Option<i8> {
    let signal_index = index.checked_sub(1)?;
    if signal_index < cfg.lookback {
        return None;
    }
    let c2 = candles[signal_index].close;
    let c1 = candles[signal_index - cfg.lookback].close;
    if c2.is_zero() {
        return None;
    }
    // C1 / C2 - 1 > 0 means the earlier price was above the later close (price
    // fell), so mean reversion buys; a negative value means price rose, so it sells.
    let change = c1 / c2 - Decimal::ONE;
    if change > cfg.threshold {
        Some(1)
    } else if change < -cfg.threshold {
        Some(-1)
    } else {
        None
    }
}

fn close_trade(
    trades: &mut Vec<Trade>,
    open: OpenTrade,
    exit_equity: Decimal,
    exit_index: usize,
    exit_price: Decimal,
    exit_time_ms: i64,
) {
    let pnl = exit_equity - open.entry_equity;
    let return_pct = if open.entry_equity.is_zero() {
        Decimal::ZERO
    } else {
        pnl / open.entry_equity * Decimal::from(100)
    };
    trades.push(Trade {
        side: open.side,
        entry_index: open.entry_index,
        exit_index,
        entry_time_ms: open.entry_time_ms,
        exit_time_ms,
        entry_price: open.entry_price,
        exit_price,
        pnl,
        return_pct,
    });
}

fn metrics_from_curve(
    curve: &[Decimal],
    trades: &[Trade],
    costs: Decimal,
    turnover: Decimal,
    exposure_days: Decimal,
    init_fund: Decimal,
    days: usize,
) -> PercentMomentumMetrics {
    let final_equity = curve.last().copied().unwrap_or(init_fund);
    let total_return_pct = if init_fund.is_zero() {
        Decimal::ZERO
    } else {
        (final_equity - init_fund) / init_fund * Decimal::from(100)
    };
    let returns = daily_returns(curve);
    let wins = trades
        .iter()
        .filter(|trade| trade.pnl > Decimal::ZERO)
        .count();
    let win_rate_pct = if trades.is_empty() {
        Decimal::ZERO
    } else {
        Decimal::from(wins) / Decimal::from(trades.len()) * Decimal::from(100)
    };
    let avg_holding_days = if trades.is_empty() {
        Decimal::ZERO
    } else {
        let total: usize = trades
            .iter()
            .map(|trade| trade.exit_index.saturating_sub(trade.entry_index))
            .sum();
        Decimal::from(total) / Decimal::from(trades.len())
    };
    let avg_abs_exposure = if days == 0 {
        Decimal::ZERO
    } else {
        exposure_days / Decimal::from(days)
    };

    PercentMomentumMetrics {
        total_return_pct,
        cagr_pct: cagr_pct(init_fund, final_equity, days),
        max_drawdown_pct: max_drawdown_pct(curve),
        sharpe: sharpe_ratio(&returns),
        sortino: sortino_ratio(&returns),
        win_rate_pct,
        trades: trades.len(),
        avg_holding_days,
        turnover,
        costs,
        avg_abs_exposure,
    }
}

fn daily_returns(curve: &[Decimal]) -> Vec<f64> {
    let mut returns = Vec::with_capacity(curve.len().saturating_sub(1));
    for pair in curve.windows(2) {
        let Some(previous) = pair[0].to_f64() else {
            continue;
        };
        let Some(next) = pair[1].to_f64() else {
            continue;
        };
        if previous != 0.0 {
            returns.push(next / previous - 1.0);
        }
    }
    returns
}

fn mean_and_std(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let count = values.len() as f64;
    let mean = values.iter().sum::<f64>() / count;
    if values.len() < 2 {
        return (mean, 0.0);
    }
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    (mean, variance.sqrt())
}

fn sharpe_ratio(returns: &[f64]) -> Decimal {
    if returns.len() < 2 {
        return Decimal::ZERO;
    }
    let (mean, std) = mean_and_std(returns);
    if std == 0.0 {
        return Decimal::ZERO;
    }
    Decimal::from_f64(mean / std * TRADING_DAYS_PER_YEAR.sqrt()).unwrap_or(Decimal::ZERO)
}

fn sortino_ratio(returns: &[f64]) -> Decimal {
    if returns.len() < 2 {
        return Decimal::ZERO;
    }
    let count = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / count;
    let downside = (returns
        .iter()
        .map(|value| value.min(0.0).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    if downside == 0.0 {
        return Decimal::ZERO;
    }
    Decimal::from_f64(mean / downside * TRADING_DAYS_PER_YEAR.sqrt()).unwrap_or(Decimal::ZERO)
}

fn cagr_pct(init_fund: Decimal, final_equity: Decimal, days: usize) -> Decimal {
    if days == 0 || init_fund <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let Some(init) = init_fund.to_f64() else {
        return Decimal::ZERO;
    };
    let Some(final_value) = final_equity.to_f64() else {
        return Decimal::ZERO;
    };
    if init <= 0.0 {
        return Decimal::ZERO;
    }
    if final_value <= 0.0 {
        return Decimal::from(-100);
    }
    let years = days as f64 / TRADING_DAYS_PER_YEAR;
    let cagr = (final_value / init).powf(1.0 / years) - 1.0;
    Decimal::from_f64(cagr * 100.0).unwrap_or(Decimal::ZERO)
}

fn max_drawdown_pct(curve: &[Decimal]) -> Decimal {
    let mut peak = Decimal::ZERO;
    let mut max_drawdown = Decimal::ZERO;
    for &equity in curve {
        if equity > peak {
            peak = equity;
        }
        if peak > Decimal::ZERO {
            let drawdown = (peak - equity) / peak * Decimal::from(100);
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
    }
    max_drawdown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(open_time_ms: i64, open: i64, close: i64) -> Candle {
        Candle {
            open_time_ms,
            open: Decimal::from(open),
            high: Decimal::from(open.max(close)),
            low: Decimal::from(open.min(close)),
            close: Decimal::from(close),
            volume: Decimal::ZERO,
        }
    }

    fn cfg(lookback: usize) -> PercentMomentumConfig {
        PercentMomentumConfig {
            lookback,
            threshold: Decimal::new(5, 2),
        }
    }

    fn params(fee_bps: i64, slip_bps: i64) -> PercentMomentumParams {
        PercentMomentumParams {
            init_fund: Decimal::from(1000),
            fee_bps: Decimal::from(fee_bps),
            slip_bps: Decimal::from(slip_bps),
        }
    }

    #[test]
    fn c1_five_percent_above_c2_goes_long() {
        // C1 = close[0] = 106, C2 = close[1] = 100: C1 / C2 - 1 = +6%, so the
        // earlier price was above the later close (price fell) and we buy.
        let candles = vec![
            candle(0, 106, 106),
            candle(1, 100, 100),
            candle(2, 100, 110),
        ];

        let outcome = simulate(&candles, 0, 3, &cfg(1), &params(0, 0)).unwrap();

        assert_eq!(outcome.positions, vec![0, 0, 1]);
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.side, 1);
        assert_eq!(trade.entry_index, 2);
        assert_eq!(trade.entry_price, Decimal::from(100));
        assert_eq!(trade.exit_price, Decimal::from(110));
        assert_eq!(trade.pnl, Decimal::from(100));
        assert_eq!(outcome.metrics.total_return_pct, Decimal::from(10));
    }

    #[test]
    fn c1_five_percent_below_c2_goes_short() {
        // C1 = close[0] = 94, C2 = close[1] = 100: C1 / C2 - 1 = -6%, so the
        // earlier price was below the later close (price rose) and we sell.
        let candles = vec![candle(0, 94, 94), candle(1, 100, 100), candle(2, 100, 90)];

        let outcome = simulate(&candles, 0, 3, &cfg(1), &params(0, 0)).unwrap();

        assert_eq!(outcome.positions, vec![0, 0, -1]);
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.side, -1);
        assert_eq!(trade.pnl, Decimal::from(100));
        assert_eq!(outcome.metrics.total_return_pct, Decimal::from(10));
    }

    #[test]
    fn neutral_signal_holds_existing_long() {
        let candles = vec![
            candle(0, 110, 110),
            candle(1, 100, 100),
            candle(2, 100, 100),
            candle(3, 100, 102),
        ];

        let outcome = simulate(&candles, 0, 4, &cfg(1), &params(0, 0)).unwrap();

        // C1 / C2 - 1 is +10% at close[1] (long), then 0% at close[2], so the
        // long is held into the third bar.
        assert_eq!(outcome.positions, vec![0, 0, 1, 1]);
        assert_eq!(outcome.trades.len(), 1);
        assert_eq!(outcome.metrics.total_return_pct, Decimal::from(2));
    }

    #[test]
    fn neutral_signal_from_flat_stays_flat() {
        let candles = vec![
            candle(0, 100, 100),
            candle(1, 100, 104),
            candle(2, 100, 110),
        ];

        let outcome = simulate(&candles, 0, 3, &cfg(1), &params(0, 0)).unwrap();

        assert_eq!(outcome.positions, vec![0, 0, 0]);
        assert!(outcome.trades.is_empty());
        assert_eq!(outcome.metrics.total_return_pct, Decimal::ZERO);
    }

    #[test]
    fn next_open_execution_uses_the_open_after_the_signal_close() {
        let candles = vec![
            candle(0, 120, 120),
            candle(1, 105, 100),
            candle(2, 120, 130),
        ];

        let outcome = simulate(&candles, 0, 3, &cfg(1), &params(0, 0)).unwrap();

        let trade = &outcome.trades[0];
        assert_eq!(trade.entry_index, 2);
        assert_eq!(trade.entry_price, Decimal::from(120));
        assert_eq!(trade.exit_price, Decimal::from(130));
        assert_eq!(
            outcome.metrics.total_return_pct,
            (Decimal::from(130) / Decimal::from(120) - Decimal::ONE) * Decimal::from(100)
        );
    }

    #[test]
    fn costs_are_charged_on_turnover() {
        let candles = vec![
            candle(0, 106, 106),
            candle(1, 100, 100),
            candle(2, 100, 100),
        ];

        let outcome = simulate(&candles, 0, 3, &cfg(1), &params(10, 5)).unwrap();

        // Entry: 1000 * 15bps = 1.5. Exit: 998.5 * 15bps = 1.49775.
        assert_eq!(outcome.turnover, Decimal::from(2));
        assert_eq!(outcome.costs, Decimal::new(299_775, 5));
        assert_eq!(outcome.equity.last(), Some(&Decimal::new(99_700_225, 5)));
    }

    #[test]
    fn buy_and_hold_pays_one_entry_and_one_exit_cost() {
        let candles = vec![candle(0, 100, 110)];

        let benchmark = buy_and_hold(&candles, 0, 1, &params(10, 5)).unwrap();

        // Entry: 1000 * 15bps = 1.5. Exit: 1098.35 * 15bps = 1.647525.
        assert_eq!(benchmark.costs, Decimal::new(3_147_525, 6));
        assert_eq!(
            benchmark.equity.last(),
            Some(&Decimal::new(1_096_702_475, 6))
        );
        assert_eq!(benchmark.metrics.trades, 1);
        assert_eq!(benchmark.metrics.turnover, Decimal::from(2));
        assert_eq!(benchmark.metrics.avg_abs_exposure, Decimal::ONE);
    }
}
