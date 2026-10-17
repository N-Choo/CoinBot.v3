use crate::strategy::types::Direction;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRecord { pub direction: Direction, pub r: Decimal, pub pair: String, pub entry_time_ms: i64, pub exit_time_ms: i64 }

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
    let mean = |xs: &[Decimal]| if xs.is_empty() { 0.0 } else { xs.iter().map(|d| d.to_string().parse::<f64>().unwrap()).sum::<f64>() / xs.len() as f64 };
    let all: Vec<Decimal> = trades.iter().map(|t| t.r).collect();
    let wins = trades.iter().filter(|t| t.r > Decimal::ZERO).count();
    let mut by_pair: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for t in trades { by_pair.entry(t.pair.clone()).or_default().push(t.r); }
    let longs: Vec<Decimal> = trades.iter().filter(|t| t.direction == Direction::Long).map(|t| t.r).collect();
    let shorts: Vec<Decimal> = trades.iter().filter(|t| t.direction == Direction::Short).map(|t| t.r).collect();
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
pub fn adversarial_composite(low_first: &[TradeRecord], high_first: &[TradeRecord]) -> Vec<TradeRecord> {
    let mut out: Vec<TradeRecord> = low_first.iter().filter(|t| t.direction == Direction::Long).cloned().collect();
    out.extend(high_first.iter().filter(|t| t.direction == Direction::Short).cloned());
    out
}

/// Flag when realised cost exceeds 0.5R; references the analyzer's floor.
pub fn cushion_warning(mean_cost_r: f64) -> Option<&'static str> {
    if mean_cost_r > 0.5 { Some("cost > 0.5R: the analyzer's floor (target_r in fixed_r, min_gross_rr in structural) is too low") } else { None }
}

/// Deterministic tiny LCG (no `rand`). `state = state * 6364136223846793005 + 1442695040888963407`.
pub(crate) struct Lcg(u64);
impl Lcg {
    pub(crate) fn new(seed: u64) -> Self { Lcg(seed) }
    pub(crate) fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
    pub(crate) fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

pub const BOOTSTRAP_BLOCK_MS: i64 = 14 * 86_400_000; // 2 weeks (pre-registered block length)

/// Calendar-block bootstrap: assign each trade to a calendar block by `entry_time_ms`,
/// resample whole blocks with replacement, return (mean, 5th-percentile lower bound).
pub fn time_blocked_bootstrap(trades: &[TradeRecord], block_ms: i64, resamples: usize, seed: u64) -> (f64, f64) {
    if trades.is_empty() { return (0.0, 0.0); }
    let vals = |r: Decimal| r.to_string().parse::<f64>().unwrap();
    let mean0 = trades.iter().map(|t| vals(t.r)).sum::<f64>() / trades.len() as f64;
    let t0 = trades.iter().map(|t| t.entry_time_ms).min().unwrap();
    let mut blocks: std::collections::BTreeMap<i64, Vec<f64>> = std::collections::BTreeMap::new();
    for t in trades { blocks.entry((t.entry_time_ms - t0) / block_ms).or_default().push(vals(t.r)); }
    let block_list: Vec<Vec<f64>> = blocks.into_values().collect();
    let n = block_list.len();
    let mut rng = Lcg::new(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut acc = 0.0; let mut cnt = 0usize;
        for _ in 0..n {
            let b = rng.below(n);
            for v in &block_list[b] { acc += v; cnt += 1; }
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
    fn t(r: &str, p: &str) -> TradeRecord { TradeRecord { direction: Direction::Long, r: Decimal::from_str(r).unwrap(), pair: p.into(), entry_time_ms: 0, exit_time_ms: 0 } }

    #[test]
    fn summary_counts_and_splits() {
        let s = summarize(&[t("3.0","A"), t("-1.0","A"), t("3.0","B")]);
        assert_eq!(s.n, 3);
        assert!(s.mean_r > 1.6 && s.mean_r < 1.7);
        assert_eq!(s.by_pair.len(), 2);
    }

    #[test]
    fn bootstrap_is_deterministic_and_calendar_blocked() {
        let data: Vec<_> = (0..200).map(|i| TradeRecord {
            direction: Direction::Long,
            r: Decimal::from_str(if i % 3 == 0 { "-1.0" } else { "3.0" }).unwrap(),
            pair: "A".into(), entry_time_ms: i as i64 * 86_400_000, exit_time_ms: i as i64 * 86_400_000,
        }).collect();
        assert_eq!(time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42), time_blocked_bootstrap(&data, BOOTSTRAP_BLOCK_MS, 500, 42));
    }

    #[test]
    fn cushion_warning_references_floor() {
        assert!(cushion_warning(0.6).unwrap().contains("floor"));
    }
}
