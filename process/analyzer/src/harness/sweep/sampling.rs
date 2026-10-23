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
    pub atr_enabled: bool,
    pub atr_sl_multiplier: Decimal,
    pub atr_tp_multiplier: Decimal,
}

impl Candidate {
    pub fn key(&self) -> String {
        let (sl, tp) = if self.atr_enabled {
            (
                self.atr_sl_multiplier.normalize().to_string(),
                self.atr_tp_multiplier.normalize().to_string(),
            )
        } else {
            ("off".to_string(), "off".to_string())
        };
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.rsi_period,
            self.rsi_long_threshold.normalize(),
            self.rsi_short_threshold.normalize(),
            self.atr_enabled,
            sl,
            tp
        )
    }

    pub fn to_config(&self) -> Config {
        Config {
            rsi_period: self.rsi_period,
            rsi_long_threshold: self.rsi_long_threshold,
            rsi_short_threshold: self.rsi_short_threshold,
            atr_enabled: self.atr_enabled,
            atr_sl_multiplier: self.atr_sl_multiplier,
            atr_tp_multiplier: self.atr_tp_multiplier,
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
        atr_enabled: false,
        atr_sl_multiplier: Decimal::new(20, 1),
        atr_tp_multiplier: Decimal::new(30, 1),
    }
}

pub fn generate(seed: u64) -> Vec<Candidate> {
    let base = baseline();
    let mut out = vec![base.clone()];
    let mut seen = std::collections::HashSet::new();
    seen.insert(base.key());

    let mut anchor = |label: String, f: &dyn Fn(&mut Candidate)| {
        let mut c = base.clone();
        c.label = label.clone();
        c.origin = format!("ofat:{label}");
        f(&mut c);
        if seen.insert(c.key()) {
            out.push(c);
        }
    };
    for v in [14usize, 21] {
        anchor(format!("rsi_period={v}"), &|c| c.rsi_period = v);
    }
    for v in [
        Decimal::from(55),
        Decimal::from(60),
        Decimal::from(65),
        Decimal::from(70),
    ] {
        anchor(format!("rsi_long={v}"), &|c| c.rsi_long_threshold = v);
    }
    for v in [Decimal::from(40), Decimal::from(45), Decimal::from(50)] {
        anchor(format!("rsi_short={v}"), &|c| c.rsi_short_threshold = v);
    }
    anchor("atr_enabled=true".into(), &|c| c.atr_enabled = true);
    for v in [
        Decimal::new(15, 1),
        Decimal::new(20, 1),
        Decimal::new(25, 1),
    ] {
        anchor(format!("atr_sl={v}"), &|c| {
            c.atr_enabled = true;
            c.atr_sl_multiplier = v;
        });
    }
    for v in [
        Decimal::new(20, 1),
        Decimal::new(30, 1),
        Decimal::new(40, 1),
    ] {
        anchor(format!("atr_tp={v}"), &|c| {
            c.atr_enabled = true;
            c.atr_tp_multiplier = v;
        });
    }

    let periods = [14usize, 21];
    let longs = [
        Decimal::from(55),
        Decimal::from(60),
        Decimal::from(65),
        Decimal::from(70),
    ];
    let shorts = [Decimal::from(40), Decimal::from(45), Decimal::from(50)];
    let atr = [false, true];
    let sl = [
        Decimal::new(15, 1),
        Decimal::new(20, 1),
        Decimal::new(25, 1),
    ];
    let tp = [
        Decimal::new(20, 1),
        Decimal::new(30, 1),
        Decimal::new(40, 1),
    ];
    let mut state = seed | 1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    while out.len() < 30 {
        let pick = |n: usize, next: &mut dyn FnMut() -> u64| (next() % n as u64) as usize;
        let a = pick(periods.len(), &mut next);
        let b = pick(longs.len(), &mut next);
        let c = pick(shorts.len(), &mut next);
        let d = pick(atr.len(), &mut next);
        let e = pick(sl.len(), &mut next);
        let f = pick(tp.len(), &mut next);
        let mut cand = Candidate {
            label: String::new(),
            origin: "joint".into(),
            rsi_period: periods[a],
            rsi_long_threshold: longs[b],
            rsi_short_threshold: shorts[c],
            atr_enabled: atr[d],
            atr_sl_multiplier: sl[e],
            atr_tp_multiplier: tp[f],
        };
        if cand.rsi_short_threshold >= cand.rsi_long_threshold {
            continue;
        }
        if seen.insert(cand.key()) {
            cand.label = format!("joint-{:02}", out.len());
            out.push(cand);
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
        assert!(!c.atr_enabled);
    }

    #[test]
    fn atr_off_keys_ignore_level_multipliers() {
        let base = baseline();
        let mut different_levels = baseline();
        different_levels.atr_sl_multiplier = Decimal::new(99, 0);
        different_levels.atr_tp_multiplier = Decimal::new(88, 0);
        assert_eq!(base.key(), different_levels.key());

        let enabled = Candidate {
            atr_enabled: true,
            ..base
        };
        assert_ne!(different_levels.key(), enabled.key());
    }

    #[test]
    fn generated_candidates_are_unique_and_valid() {
        let candidates = generate(SEED_DEFAULT);
        let mut keys = std::collections::HashSet::new();
        for candidate in &candidates {
            assert!(keys.insert(candidate.key()));
            assert!(candidate.rsi_short_threshold < candidate.rsi_long_threshold);
            assert!(candidate.to_config().validate().is_ok());
        }
    }
}
