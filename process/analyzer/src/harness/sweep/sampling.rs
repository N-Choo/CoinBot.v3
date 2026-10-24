use crate::strategy::config::Config;
use rust_decimal::Decimal;
use serde::Serialize;

pub const SEED_DEFAULT: u64 = 20261023;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candidate {
    pub label: String,
    pub origin: String,
    pub rsi_period: usize,
    pub rsi_long_threshold: Decimal,
    pub rsi_short_threshold: Decimal,
}

impl Candidate {
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.rsi_period,
            self.rsi_long_threshold.normalize(),
            self.rsi_short_threshold.normalize()
        )
    }

    pub fn to_config(&self) -> Config {
        Config {
            rsi_period: self.rsi_period,
            rsi_long_threshold: self.rsi_long_threshold,
            rsi_short_threshold: self.rsi_short_threshold,
            ..Config::default()
        }
    }
}

pub fn baseline() -> Candidate {
    Candidate {
        label: "baseline".into(),
        origin: "baseline".into(),
        rsi_period: 21,
        rsi_long_threshold: Decimal::from(60),
        rsi_short_threshold: Decimal::from(50),
    }
}

/// Generates the full 2 × 4 × 3 RSI grid, with the baseline first.
///
/// The grid is exhaustive, so `seed` does not affect the output; it is retained
/// for sweep artifact compatibility.
pub fn generate(_seed: u64) -> Vec<Candidate> {
    let base = baseline();
    let mut out = vec![base.clone()];

    let periods = [14usize, 21];
    let longs = [
        Decimal::from(55),
        Decimal::from(60),
        Decimal::from(65),
        Decimal::from(70),
    ];
    let shorts = [Decimal::from(40), Decimal::from(45), Decimal::from(50)];
    for period in periods {
        for long in longs {
            for short in shorts {
                let candidate = Candidate {
                    label: format!("rsi{period}-long{long}-short{short}"),
                    origin: "grid".into(),
                    rsi_period: period,
                    rsi_long_threshold: long,
                    rsi_short_threshold: short,
                };
                if candidate.key() != base.key() {
                    out.push(candidate);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_the_researched_profile() {
        let c = baseline();
        assert_eq!(c.rsi_period, 21);
        assert_eq!(c.rsi_long_threshold, Decimal::from(60));
        assert_eq!(c.rsi_short_threshold, Decimal::from(50));
    }

    #[test]
    fn generated_candidates_are_unique_and_valid() {
        let candidates = generate(SEED_DEFAULT);
        assert_eq!(candidates.len(), 24);
        let mut keys = std::collections::HashSet::new();
        for candidate in &candidates {
            assert!(keys.insert(candidate.key()));
            assert!(candidate.rsi_short_threshold < candidate.rsi_long_threshold);
            assert!(candidate.to_config().validate().is_ok());
        }
    }
}
