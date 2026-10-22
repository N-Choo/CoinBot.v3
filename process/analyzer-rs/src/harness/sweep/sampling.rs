use crate::strategy::config::Config;
use rust_decimal::Decimal;
use serde::Serialize;

pub const SEED_DEFAULT: u64 = 20261022;
pub const SMA_PERIOD: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candidate {
    pub label: String,
    pub origin: String,
    pub rsi_period: usize,
    pub rsi_order: usize,
    pub max_age: Option<usize>, // None = uncapped
    pub sma_enabled: bool,
    pub sma_period: usize,
    pub atr_sl_multiplier: Decimal,
    pub atr_tp_multiplier: Decimal,
}

impl Candidate {
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.rsi_period,
            self.rsi_order,
            self.max_age
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".into()),
            self.sma_enabled,
            self.sma_period,
            self.atr_sl_multiplier.normalize(),
            self.atr_tp_multiplier.normalize()
        )
    }

    pub fn to_config(&self) -> Config {
        Config {
            rsi_period: self.rsi_period,
            rsi_order: self.rsi_order,
            price_diff_threshold: Decimal::new(5, 2),
            sma_enabled: self.sma_enabled,
            sma_period: self.sma_period,
            atr_period: 14,
            atr_sl_multiplier: self.atr_sl_multiplier,
            atr_tp_multiplier: self.atr_tp_multiplier,
            atr_pct_high: Decimal::new(50, 1),
            atr_high_adjust: Decimal::new(15, 1),
            atr_sl_max_pct: Decimal::ZERO,
            window: 100,
        }
    }
}

pub fn baseline() -> Candidate {
    Candidate {
        label: "baseline".into(),
        origin: "baseline".into(),
        rsi_period: 14,
        rsi_order: 5,
        max_age: Some(12),
        sma_enabled: true,
        sma_period: SMA_PERIOD,
        atr_sl_multiplier: Decimal::new(15, 1),
        atr_tp_multiplier: Decimal::new(20, 1),
    }
}

pub fn generate(seed: u64) -> Vec<Candidate> {
    let base = baseline();
    let mut out = vec![base.clone()];
    let mut seen = std::collections::HashSet::new();
    seen.insert(base.key());

    // OFAT anchors.
    let mut anchor = |label: String, f: &dyn Fn(&mut Candidate)| {
        let mut c = base.clone();
        c.label = label.clone();
        c.origin = format!("ofat:{label}");
        f(&mut c);
        if seen.insert(c.key()) {
            out.push(c);
        }
    };
    for v in [10usize, 21] {
        anchor(format!("rsi_period={v}"), &|c| c.rsi_period = v);
    }
    for v in [3usize, 7] {
        anchor(format!("rsi_order={v}"), &|c| c.rsi_order = v);
    }
    for v in [Some(4usize), Some(8), None] {
        let label = format!(
            "max_age={}",
            v.map(|x| x.to_string())
                .unwrap_or_else(|| "uncapped".into())
        );
        anchor(label, &|c| c.max_age = v);
    }
    anchor("sma_enabled=false".into(), &|c| c.sma_enabled = false);
    for v in [Decimal::new(20, 1), Decimal::new(25, 1)] {
        anchor(format!("atr_sl={v}"), &|c| {
            c.atr_sl_multiplier = v;
        });
    }
    for v in [Decimal::new(30, 1), Decimal::new(40, 1)] {
        anchor(format!("atr_tp={v}"), &|c| {
            c.atr_tp_multiplier = v;
        });
    }

    // Deterministic seeded joint draws.
    let rsi = [10usize, 14, 21];
    let order = [3usize, 5, 7];
    let age = [Some(4usize), Some(8), Some(12), None];
    let sma = [false, true];
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
        // Pull each index into a local first so the mutable borrow of `next`
        // ends before the candidate is constructed.
        let a = pick(rsi.len(), &mut next);
        let b = pick(order.len(), &mut next);
        let c = pick(age.len(), &mut next);
        let d = pick(sma.len(), &mut next);
        let e = pick(sl.len(), &mut next);
        let f = pick(tp.len(), &mut next);
        let mut cand = Candidate {
            label: String::new(),
            origin: "joint".into(),
            rsi_period: rsi[a],
            rsi_order: order[b],
            max_age: age[c],
            sma_enabled: sma[d],
            sma_period: SMA_PERIOD,
            atr_sl_multiplier: sl[e],
            atr_tp_multiplier: tp[f],
        };
        if seen.insert(cand.key()) {
            cand.label = format!("joint-{:02}", out.len());
            out.push(cand);
        }
    }
    out
}
