use crate::harness::dataset::Dataset;
use crate::harness::replay::build_replay;
use crate::harness::sim::{
    simulate, unrealized_pnl, SimOutcome, SimSignal, REASON_GAP_SL, REASON_GAP_TP, REASON_SL,
    REASON_TP,
};
use crate::harness::sweep::costs::{net_open_pnl, net_trade_pnl, CostScenario};
use crate::harness::sweep::partition::{slice_with_warmup, Slice, Splits};
use crate::strategy::config::Config;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PartitionMetrics {
    pub trades: usize,
    pub open_at_end: usize,
    pub gross_pnl: Decimal,
    pub closed_net_pnl: Decimal,
    pub open_net_pnl: Decimal,
    /// Closed net PnL plus the mark-to-market of positions still open at partition end.
    pub net_pnl: Decimal,
    /// Mean net PnL of closed trades.
    pub net_expectancy: Decimal,
    /// Mean of closed net PnL divided by the position's initial notional `E`.
    pub net_expectancy_r: Decimal,
    pub net_profit_factor: Option<Decimal>,
    /// Win rate over closed trades only, as a percentage.
    pub win_rate: Decimal,
    pub max_drawdown: Decimal,
    pub max_drawdown_pct: Decimal,
    /// Sum of candle durations held by closed trades and open positions.
    pub exposure_candles: usize,
    /// Exposure as a percentage of replayed candles.
    pub time_in_market_pct: Decimal,
    /// Standard error of the mean closed net PnL (`std / sqrt(n)`).
    pub expectancy_stderr: Decimal,
    pub tp_hits: usize,
    pub sl_hits: usize,
    pub other_exits: usize,
}

/// Computes partition metrics from a simulated outcome.
///
/// Closed trades pay entry and exit costs on their actual notionals. Positions open at the
/// partition boundary are marked at `last_close` (the partition's final close) and also pay an
/// estimated exit cost; the mark is included in the final net equity point for drawdown.
///
/// A closed trade with a zero `fill` cannot be costed; its gross value is used unchanged.
/// Zero fill is unreachable with validated data; if it occurs, an open position contributes
/// nothing to PnL (its count is still reported).
pub fn metrics_from_outcome(
    outcome: &SimOutcome,
    last_close: Decimal,
    last_index: usize,
    replayed_candles: usize,
    init_fund: Decimal,
    scenario: &CostScenario,
) -> PartitionMetrics {
    let mut gross = Decimal::ZERO;
    let mut closed_net = Decimal::ZERO;
    let mut wins = 0usize;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    let mut r_sum = Decimal::ZERO;
    let mut r_n = 0usize;
    let mut equity = init_fund;
    let mut peak = init_fund;
    let mut max_dd = Decimal::ZERO;
    let mut pnls: Vec<Decimal> = Vec::with_capacity(outcome.trades.len());
    let (mut tp, mut sl, mut other) = (0usize, 0usize, 0usize);
    let mut exposure_candles = 0usize;

    for trade in &outcome.trades {
        let net = if trade.fill.is_zero() {
            trade.pnl
        } else {
            net_trade_pnl(
                scenario, trade.side, trade.e, trade.fill, trade.exit, trade.pnl,
            )
        };
        gross += trade.pnl;
        closed_net += net;
        pnls.push(net);
        if net > Decimal::ZERO {
            wins += 1;
            gross_profit += net;
        } else if net < Decimal::ZERO {
            gross_loss += net;
        }
        if !trade.e.is_zero() {
            r_sum += net / trade.e;
            r_n += 1;
        }
        match trade.reason.as_str() {
            REASON_TP | REASON_GAP_TP => tp += 1,
            REASON_SL | REASON_GAP_SL => sl += 1,
            _ => other += 1,
        }
        exposure_candles += trade.exit_index.saturating_sub(trade.entry_index);
        equity += net;
        if equity > peak {
            peak = equity;
        }
        let drawdown = peak - equity;
        if drawdown > max_dd {
            max_dd = drawdown;
        }
    }

    // Mark positions still open at the partition boundary; they are never carried forward.
    let mut open_net = Decimal::ZERO;
    for position in &outcome.open_positions {
        if position.fill.is_zero() {
            continue;
        }
        let gross_unrealized = unrealized_pnl(position.side, position.fill, last_close, position.e);
        let net = net_open_pnl(
            scenario,
            position.e,
            position.fill,
            last_close,
            gross_unrealized,
        );
        gross += gross_unrealized;
        open_net += net;
        exposure_candles += last_index.saturating_sub(position.entry_index);
    }

    // The final open mark closes the net equity curve.
    equity += open_net;
    if equity > peak {
        peak = equity;
    }
    let drawdown = peak - equity;
    if drawdown > max_dd {
        max_dd = drawdown;
    }

    let n_trades = Decimal::from(outcome.trades.len());
    let net_expectancy = if outcome.trades.is_empty() {
        Decimal::ZERO
    } else {
        closed_net / n_trades
    };
    let net_expectancy_r = if r_n == 0 {
        Decimal::ZERO
    } else {
        r_sum / Decimal::from(r_n)
    };
    let variance = if pnls.len() < 2 {
        Decimal::ZERO
    } else {
        let sum_sq: Decimal = pnls
            .iter()
            .map(|pnl| {
                let diff = *pnl - net_expectancy;
                diff * diff
            })
            .sum();
        sum_sq / Decimal::from(pnls.len() - 1)
    };
    let expectancy_stderr = if pnls.len() < 2 {
        Decimal::ZERO
    } else {
        // `rust_decimal`'s `sqrt` is behind the optional `maths` feature; bridge through f64
        // (the standard error is a reporting statistic, not an accounting value).
        let mean_variance = (variance / Decimal::from(pnls.len()))
            .to_f64()
            .unwrap_or(0.0);
        Decimal::from_f64(mean_variance.sqrt()).unwrap_or(Decimal::ZERO)
    };
    let net_profit_factor = if gross_loss.is_zero() {
        None
    } else {
        Some(gross_profit / gross_loss.abs())
    };
    let win_rate = if outcome.trades.is_empty() {
        Decimal::ZERO
    } else {
        Decimal::from(wins) / n_trades * Decimal::from(100)
    };
    let max_drawdown_pct = if init_fund.is_zero() {
        Decimal::ZERO
    } else {
        max_dd / init_fund * Decimal::from(100)
    };
    let time_in_market_pct = if replayed_candles == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(exposure_candles) / Decimal::from(replayed_candles) * Decimal::from(100)
    };

    PartitionMetrics {
        trades: outcome.trades.len(),
        open_at_end: outcome.open_positions.len(),
        gross_pnl: gross,
        closed_net_pnl: closed_net,
        open_net_pnl: open_net,
        net_pnl: closed_net + open_net,
        net_expectancy,
        net_expectancy_r,
        net_profit_factor,
        win_rate,
        max_drawdown: max_dd,
        max_drawdown_pct,
        exposure_candles,
        time_in_market_pct,
        expectancy_stderr,
        tp_hits: tp,
        sl_hits: sl,
        other_exits: other,
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ConfigResult {
    /// scenario label -> partition name -> metrics
    pub partitions: BTreeMap<String, BTreeMap<String, PartitionMetrics>>,
}

/// Evaluates one configuration on every partition and cost scenario.
///
/// Each partition is a self-contained backtest: `window` warm-up candles immediately preceding
/// the partition start, a fresh account seeded with `init_fund`, and only signals inside the
/// evaluated range. Positions open at partition end are marked and dropped.
pub fn evaluate_config(
    dataset: &Dataset,
    cfg: &Config,
    max_age: Option<usize>,
    splits: &Splits,
    window: usize,
    init_fund: i64,
    scenarios: &[CostScenario],
) -> Result<ConfigResult, String> {
    if window != cfg.window {
        return Err(format!("window {window} != cfg.window {}", cfg.window));
    }
    let mut partitions: Vec<(String, Slice)> = vec![
        ("train".into(), splits.train),
        ("validation".into(), splits.validation),
        ("holdout".into(), splits.holdout),
    ];
    for (index, fold) in splits.folds.iter().enumerate() {
        partitions.push((format!("fold{}_val", index + 1), fold.val));
    }

    let init_fund_decimal = Decimal::from(init_fund);
    let mut result = ConfigResult::default();

    for (name, slice) in partitions {
        let partition = slice_with_warmup(&dataset.candles, slice, window);
        let eval_start = window.max(partition.partition_start);
        let n_candles = partition.candles.len();
        let last_index = n_candles.saturating_sub(1);
        let replayed_candles = n_candles.saturating_sub(window);
        let slice_dataset = Dataset {
            symbol: dataset.symbol.clone(),
            candles: partition.candles,
            provenance: dataset.provenance.clone(),
        };
        let replay = build_replay(&slice_dataset, cfg, max_age)
            .map_err(|error| format!("partition {name}: {error}"))?;
        let sim_signals: Vec<SimSignal> = replay
            .signals
            .into_iter()
            .filter(|signal| {
                signal.candle_index >= eval_start && signal.candle_index + 1 < n_candles
            })
            .map(|signal| SimSignal {
                candle_index: signal.candle_index,
                signal: signal.signal,
                signal_id: signal.signal_id,
            })
            .collect();
        let outcome = simulate(&slice_dataset.candles, &sim_signals, init_fund, window);
        let last_close = slice_dataset
            .candles
            .last()
            .expect("partition always contains candles")
            .close;
        for scenario in scenarios {
            let metrics = metrics_from_outcome(
                &outcome,
                last_close,
                last_index,
                replayed_candles,
                init_fund_decimal,
                scenario,
            );
            result
                .partitions
                .entry(scenario.label.clone())
                .or_default()
                .insert(name.clone(), metrics);
        }
    }

    Ok(result)
}
