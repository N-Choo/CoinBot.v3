use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRecord {
    pub direction: Direction,
    pub r: Decimal,
    pub pair: String,
    pub entry_time_ms: i64,
    pub exit_time_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub mean_r: f64,
    pub win_rate: f64,
    pub by_pair: BTreeMap<String, f64>,
    pub by_direction_long: f64,
    pub by_direction_short: f64,
}

pub fn summarize(trades: &[TradeRecord]) -> Summary {
    let n = trades.len();
    let mean = |xs: &[Decimal]| {
        if xs.is_empty() {
            0.0
        } else {
            xs.iter()
                .map(|d| d.to_string().parse::<f64>().unwrap())
                .sum::<f64>()
                / xs.len() as f64
        }
    };
    let all: Vec<Decimal> = trades.iter().map(|t| t.r).collect();
    let wins = trades.iter().filter(|t| t.r > Decimal::ZERO).count();
    let mut by_pair: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for t in trades {
        by_pair.entry(t.pair.clone()).or_default().push(t.r);
    }
    let longs: Vec<Decimal> = trades
        .iter()
        .filter(|t| t.direction == Direction::Long)
        .map(|t| t.r)
        .collect();
    let shorts: Vec<Decimal> = trades
        .iter()
        .filter(|t| t.direction == Direction::Short)
        .map(|t| t.r)
        .collect();
    Summary {
        n,
        mean_r: mean(&all),
        win_rate: if n == 0 { 0.0 } else { wins as f64 / n as f64 },
        by_pair: by_pair.into_iter().map(|(k, v)| (k, mean(&v))).collect(),
        by_direction_long: mean(&longs),
        by_direction_short: mean(&shorts),
    }
}

/// Adversarial composite: longs from `low_first`, shorts from `high_first`.
pub fn adversarial_composite(
    low_first: &[TradeRecord],
    high_first: &[TradeRecord],
) -> Vec<TradeRecord> {
    let mut out: Vec<TradeRecord> = low_first
        .iter()
        .filter(|t| t.direction == Direction::Long)
        .cloned()
        .collect();
    out.extend(
        high_first
            .iter()
            .filter(|t| t.direction == Direction::Short)
            .cloned(),
    );
    out
}

/// Flag when realised cost exceeds 0.5R; references the analyzer's floor.
pub fn cushion_warning(mean_cost_r: f64) -> Option<&'static str> {
    if mean_cost_r > 0.5 {
        Some("cost > 0.5R: the analyzer's floor (target_r in fixed_r, min_gross_rr in structural) is too low")
    } else {
        None
    }
}

/// Pre-registered **relative** tolerance between the 1m-derived 4H aggregate and the
/// independently fetched 4H series. Fixed **before** looking at any real data. A mismatched field
/// above this refuses the run; nothing is ever silently patched.
pub const AGG_TOLERANCE: f64 = 1e-9;

/// One OHLC/timestamp field that differs between an aggregated and a fetched 4H candle.
#[derive(Debug, Clone, PartialEq)]
pub struct AggMismatch {
    pub open_time_ms: i64,
    pub field: &'static str,
    pub aggregated: Decimal,
    pub fetched: Decimal,
    pub rel_diff: f64,
}

/// Compare a 1m-derived 4H series with the independently fetched 4H series field by field.
/// Returns every mismatched field (even sub-tolerance ones, so they can be reported); never
/// mutates or patches either series. The two series are aligned by index.
pub fn compare_aggregates(aggregated: &[Candle], fetched: &[Candle]) -> Vec<AggMismatch> {
    let mut out = Vec::new();
    for (a, f) in aggregated.iter().zip(fetched) {
        let fields: [(&'static str, Decimal, Decimal); 5] = [
            ("open", a.open, f.open),
            ("high", a.high, f.high),
            ("low", a.low, f.low),
            ("close", a.close, f.close),
            (
                "open_time_ms",
                Decimal::from(a.open_time_ms),
                Decimal::from(f.open_time_ms),
            ),
        ];
        for (field, av, fv) in fields {
            if av != fv {
                out.push(AggMismatch {
                    open_time_ms: a.open_time_ms,
                    field,
                    aggregated: av,
                    fetched: fv,
                    rel_diff: relative_diff(av, fv),
                });
            }
        }
    }
    out
}

/// Relative difference of `a` against the fetched value `b` (zero-safe: a zero fetched value
/// falls back to an absolute difference).
fn relative_diff(a: Decimal, b: Decimal) -> f64 {
    let denom = if b.is_zero() { Decimal::ONE } else { b.abs() };
    ((a - b).abs() / denom).to_string().parse::<f64>().unwrap()
}

/// Validate a 1m-derived 4H series against the fetched 4H series. Mismatched fields are reported;
/// if any field exceeds `AGG_TOLERANCE` the run is **refused** with an `Err`. Network is never
/// touched here — both series are local files loaded by the caller (`fetch-data` owns all I/O),
/// so the backtest path stays offline.
pub fn validate_aggregates(
    aggregated: &[Candle],
    fetched: &[Candle],
) -> Result<Vec<AggMismatch>, String> {
    if aggregated.len() != fetched.len() {
        return Err(format!(
            "aggregate length mismatch: {} aggregated vs {} fetched 4H candles",
            aggregated.len(),
            fetched.len()
        ));
    }
    let mismatches = compare_aggregates(aggregated, fetched);
    if let Some(first) = mismatches.iter().find(|m| m.rel_diff > AGG_TOLERANCE) {
        return Err(format!(
            "refusing 1m run: {} aggregate field(s) exceed AGG_TOLERANCE={AGG_TOLERANCE:e} (first: candle {} {} aggregated={} fetched={})",
            mismatches.iter().filter(|m| m.rel_diff > AGG_TOLERANCE).count(),
            first.open_time_ms,
            first.field,
            first.aggregated,
            first.fetched,
        ));
    }
    Ok(mismatches)
}

/// Fraction of candles where the assumed intrabar low/high order differed from the order observed
/// in 1m data. 0.0 when there is nothing to compare.
pub fn calibration_wrong_fraction(assumed_low_first: &[bool], observed_low_first: &[bool]) -> f64 {
    if assumed_low_first.is_empty() {
        return 0.0;
    }
    let wrong = assumed_low_first
        .iter()
        .zip(observed_low_first)
        .filter(|(a, o)| a != o)
        .count();
    wrong as f64 / assumed_low_first.len() as f64
}

/// Deterministic tiny LCG (no `rand`). `state = state * 6364136223846793005 + 1442695040888963407`.
pub(crate) struct Lcg(u64);
impl Lcg {
    pub(crate) fn new(seed: u64) -> Self {
        Lcg(seed)
    }
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

pub const BOOTSTRAP_BLOCK_MS: i64 = 14 * 86_400_000; // 2 weeks (pre-registered block length)

/// Calendar-block bootstrap: assign each trade to a calendar block by `entry_time_ms`,
/// resample whole blocks with replacement, return (mean, 5th-percentile lower bound).
pub fn time_blocked_bootstrap(
    trades: &[TradeRecord],
    block_ms: i64,
    resamples: usize,
    seed: u64,
) -> (f64, f64) {
    if trades.is_empty() {
        return (0.0, 0.0);
    }
    let vals = |r: Decimal| r.to_string().parse::<f64>().unwrap();
    let mean0 = trades.iter().map(|t| vals(t.r)).sum::<f64>() / trades.len() as f64;
    let t0 = trades.iter().map(|t| t.entry_time_ms).min().unwrap();
    let mut blocks: std::collections::BTreeMap<i64, Vec<f64>> = std::collections::BTreeMap::new();
    for t in trades {
        blocks
            .entry((t.entry_time_ms - t0) / block_ms)
            .or_default()
            .push(vals(t.r));
    }
    let block_list: Vec<Vec<f64>> = blocks.into_values().collect();
    let n = block_list.len();
    let mut rng = Lcg::new(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut acc = 0.0;
        let mut cnt = 0usize;
        for _ in 0..n {
            let b = rng.below(n);
            for v in &block_list[b] {
                acc += v;
                cnt += 1;
            }
        }
        means.push(acc / cnt as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (mean0, means[(means.len() as f64 * 0.05) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    fn t(r: &str, p: &str) -> TradeRecord {
        TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(r).unwrap(),
            pair: p.into(),
            entry_time_ms: 0,
            exit_time_ms: 0,
        }
    }

    #[test]
    fn summary_counts_and_splits() {
        let s = summarize(&[t("3.0", "A"), t("-1.0", "A"), t("3.0", "B")]);
        assert_eq!(s.n, 3);
        assert!(s.mean_r > 1.6 && s.mean_r < 1.7);
        assert_eq!(s.by_pair.len(), 2);
    }

    #[test]
    fn bootstrap_is_deterministic_and_calendar_blocked() {
        let data: Vec<_> = (0..200)
            .map(|i| TradeRecord {
                direction: Direction::Long,
                r: Decimal::from_str(if i % 3 == 0 { "-1.0" } else { "3.0" }).unwrap(),
                pair: "A".into(),
                entry_time_ms: i as i64 * 86_400_000,
                exit_time_ms: i as i64 * 86_400_000,
            })
            .collect();
        assert_eq!(
            time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42),
            time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42)
        );
    }

    #[test]
    fn cushion_warning_references_floor() {
        assert!(cushion_warning(0.6).unwrap().contains("floor"));
    }

    fn candle(open_time_ms: i64, open: &str, high: &str, low: &str, close: &str) -> Candle {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        Candle {
            open_time_ms,
            open: d(open),
            high: d(high),
            low: d(low),
            close: d(close),
        }
    }

    #[test]
    fn matching_aggregate_passes_and_reports_nothing() {
        let agg = vec![
            candle(0, "1.0", "2.0", "0.5", "1.5"),
            candle(14_400_000, "1.5", "2.5", "1.0", "2.0"),
        ];
        let fetched = agg.clone();
        assert!(compare_aggregates(&agg, &fetched).is_empty());
        assert_eq!(validate_aggregates(&agg, &fetched).unwrap(), Vec::new());
    }

    #[test]
    fn mismatched_aggregate_is_reported_and_refused_above_tolerance() {
        let agg = vec![candle(0, "1.0", "2.0", "0.5", "1.5")];
        // A fetched high of 2.000000001 is short of the tolerance; 2.5 is far above it.
        let sub = vec![candle(0, "1.0", "2.000000001", "0.5", "1.5")];
        let over = vec![candle(0, "1.0", "2.5", "0.5", "1.5")];

        let reported = compare_aggregates(&agg, &sub);
        assert_eq!(
            reported.len(),
            1,
            "a sub-tolerance difference is still reported"
        );
        assert_eq!(reported[0].field, "high");
        assert!(
            validate_aggregates(&agg, &sub).is_ok(),
            "sub-tolerance differences must not refuse the run"
        );

        let reported_over = compare_aggregates(&agg, &over);
        assert_eq!(
            reported_over.len(),
            1,
            "the mismatched candle must be reported"
        );
        let err = validate_aggregates(&agg, &over).unwrap_err();
        assert!(
            err.contains("refusing 1m run"),
            "above-tolerance mismatch must refuse the run: {err}"
        );
    }

    #[test]
    fn length_mismatch_is_refused() {
        let agg = vec![candle(0, "1.0", "2.0", "0.5", "1.5")];
        assert!(validate_aggregates(&agg, &[]).is_err());
    }

    #[test]
    fn calibration_wrong_fraction_counts_disagreements() {
        assert_eq!(
            calibration_wrong_fraction(&[true, false, true], &[true, false, true]),
            0.0
        );
        assert_eq!(
            calibration_wrong_fraction(&[true, true], &[false, false]),
            1.0
        );
        assert_eq!(
            calibration_wrong_fraction(&[true, true], &[true, false]),
            0.5
        );
        assert_eq!(calibration_wrong_fraction(&[], &[]), 0.0);
    }
}
