use crate::harness::dataset::Dataset;
use crate::harness::replay::{build_replay, ReplaySignal};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::{metrics_from_outcome, pool, PartitionMetrics};
use crate::harness::sweep::execution::{simulate_execution, ExecModel, ExecPartition};
use crate::harness::sweep::freeze::DecisionRule;
use crate::harness::sweep::partition::{slice_with_warmup, Slice};
use crate::strategy::config::Config;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;

/// Result of the pre-registered confirmation rule.
#[derive(Debug, Clone, Serialize)]
pub struct Decision {
    pub passed: bool,
    pub positive_symbols: usize,
    pub symbol_count: usize,
    /// Empty when the confirmation passes; otherwise one entry per failed condition.
    pub reasons: Vec<String>,
}

fn zero_cost() -> CostScenario {
    CostScenario {
        label: "net".into(),
        fee_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    }
}

fn fill_rates(
    filled: usize,
    signals: usize,
    skipped_funds: usize,
    invalid_levels: usize,
) -> (Decimal, Decimal) {
    let capital_fill_rate = if signals == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(filled) / Decimal::from(signals)
    };
    let executable = signals
        .saturating_sub(skipped_funds)
        .saturating_sub(invalid_levels);
    let execution_fill_rate = if executable == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(filled) / Decimal::from(executable)
    };
    (capital_fill_rate, execution_fill_rate)
}

/// Evaluates the frozen config and execution model over the whole dataset.
///
/// The first `window` candles are warm-up only. Every signal at or after them is simulated with a
/// fresh account, and positions still open at the end are closed at the final close by the
/// execution simulator.
pub fn evaluate_full_period(
    dataset: &Dataset,
    cfg: &Config,
    max_age: Option<usize>,
    window: usize,
    init_fund: i64,
    model: &ExecModel,
) -> Result<ExecPartition, String> {
    if window != cfg.window {
        return Err(format!("window {window} != cfg.window {}", cfg.window));
    }
    let slice = slice_with_warmup(
        &dataset.candles,
        Slice {
            start: 0,
            end: dataset.candles.len(),
        },
        window,
    );
    let slice_dataset = Dataset {
        symbol: dataset.symbol.clone(),
        candles: slice.candles,
        provenance: dataset.provenance.clone(),
    };
    let replay = build_replay(&slice_dataset, cfg, max_age).map_err(|error| error.to_string())?;
    let eval_start = window.max(slice.partition_start);
    let signals: Vec<ReplaySignal> = replay
        .signals
        .into_iter()
        .filter(|signal| signal.candle_index >= eval_start)
        .collect();

    let outcome = simulate_execution(&slice_dataset.candles, &signals, init_fund, window, model);
    let last_close = slice_dataset
        .candles
        .last()
        .ok_or_else(|| format!("{} has no candles", dataset.symbol))?
        .close;
    let last_index = slice_dataset.candles.len().saturating_sub(1);
    let replayed_candles = slice_dataset.candles.len().saturating_sub(window);
    let sim_outcome = outcome.to_sim_outcome();
    let metrics = metrics_from_outcome(
        &sim_outcome,
        last_close,
        last_index,
        replayed_candles,
        Decimal::from(init_fund),
        &zero_cost(),
    );
    let (capital_fill_rate, execution_fill_rate) = fill_rates(
        outcome.filled,
        outcome.signals_total,
        outcome.skipped_funds,
        outcome.invalid_levels,
    );

    Ok(ExecPartition {
        metrics,
        signals: outcome.signals_total,
        filled: outcome.filled,
        expired: outcome.expired,
        skipped_funds: outcome.skipped_funds,
        invalid_levels: outcome.invalid_levels,
        capital_fill_rate,
        execution_fill_rate,
    })
}

/// Pools per-symbol partitions, summing counts and recomputing the fill rates.
pub fn pool_parts(parts: &[ExecPartition]) -> ExecPartition {
    let metrics: Vec<PartitionMetrics> = parts.iter().map(|part| part.metrics.clone()).collect();
    let mut pooled = ExecPartition {
        metrics: pool(&metrics),
        signals: 0,
        filled: 0,
        expired: 0,
        skipped_funds: 0,
        invalid_levels: 0,
        capital_fill_rate: Decimal::ZERO,
        execution_fill_rate: Decimal::ZERO,
    };
    for part in parts {
        pooled.signals += part.signals;
        pooled.filled += part.filled;
        pooled.expired += part.expired;
        pooled.skipped_funds += part.skipped_funds;
        pooled.invalid_levels += part.invalid_levels;
    }
    let (capital_fill_rate, execution_fill_rate) = fill_rates(
        pooled.filled,
        pooled.signals,
        pooled.skipped_funds,
        pooled.invalid_levels,
    );
    pooled.capital_fill_rate = capital_fill_rate;
    pooled.execution_fill_rate = execution_fill_rate;
    pooled
}

fn display_decimal(value: Decimal) -> String {
    value.round_dp(4).to_string()
}

fn display_profit_factor(value: Option<Decimal>) -> String {
    value
        .map(|value| value.round_dp(2).to_string())
        .unwrap_or_else(|| "-".to_string())
}

/// Applies the lock's pre-registered rule to the pooled and per-symbol results.
pub fn decide(
    pooled: &ExecPartition,
    per_symbol: &[ExecPartition],
    rule: &DecisionRule,
) -> Decision {
    let mut reasons = Vec::new();

    if pooled.metrics.trades < rule.min_pooled_trades {
        reasons.push(format!(
            "pooled closed trades {} were below the minimum {}",
            pooled.metrics.trades, rule.min_pooled_trades
        ));
    }

    if rule.require_positive_net_expectancy && pooled.metrics.net_expectancy <= Decimal::ZERO {
        reasons.push(format!(
            "pooled net expectancy {} was not positive",
            display_decimal(pooled.metrics.net_expectancy)
        ));
    }

    let profit_factor_threshold =
        Decimal::from_f64(rule.require_profit_factor_above).unwrap_or(Decimal::ONE);
    let profit_factor_ok = match pooled.metrics.net_profit_factor {
        Some(value) => value > profit_factor_threshold,
        // No losing trades makes the ratio undefined; treat a profitable book as above the
        // threshold, matching the sweep's profit-factor convention.
        None => pooled.metrics.trades > 0 && pooled.metrics.gross_profit > Decimal::ZERO,
    };
    if !profit_factor_ok {
        reasons.push(format!(
            "pooled net profit factor {} was not above {}",
            display_profit_factor(pooled.metrics.net_profit_factor),
            rule.require_profit_factor_above
        ));
    }

    let positive_symbols = per_symbol
        .iter()
        .filter(|part| part.metrics.net_expectancy > Decimal::ZERO)
        .count();
    if positive_symbols < rule.min_positive_symbols {
        reasons.push(format!(
            "only {positive_symbols} of {} symbols had positive net expectancy (minimum {} positive symbols)",
            per_symbol.len(),
            rule.min_positive_symbols
        ));
    }

    Decision {
        passed: reasons.is_empty(),
        positive_symbols,
        symbol_count: per_symbol.len(),
        reasons,
    }
}
