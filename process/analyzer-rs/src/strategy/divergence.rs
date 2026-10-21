use crate::strategy::config::Config;
use crate::strategy::decimal::neg;
use crate::strategy::error::Result;
use crate::strategy::indicators::peaks::find_peaks;
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::types::Candle;
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergences {
    pub rsi: Vec<Option<Decimal>>,
    pub regular_bullish: Vec<bool>,
    pub regular_bearish: Vec<bool>,
    pub hidden_bullish: Vec<bool>,
    pub hidden_bearish: Vec<bool>,
}

pub fn detect(candles: &[Candle], cfg: &Config) -> Result<Divergences> {
    let n = candles.len();
    let closes: Vec<Option<Decimal>> = candles.iter().map(|c| Some(c.close)).collect();
    let highs: Vec<Option<Decimal>> = candles.iter().map(|c| Some(c.high)).collect();
    let lows: Vec<Option<Decimal>> = candles.iter().map(|c| Some(c.low)).collect();

    let rsi_values = rsi(&closes, cfg.rsi_period)?;

    let mut inverted_lows = Vec::with_capacity(n);
    for low in &lows {
        inverted_lows.push(match low {
            Some(v) => Some(neg(*v)?),
            None => None,
        });
    }

    let mut inverted_rsi = Vec::with_capacity(n);
    for value in &rsi_values {
        inverted_rsi.push(match value {
            Some(v) => Some(neg(*v)?),
            None => None,
        });
    }

    let price_peaks = find_peaks(&highs, cfg.rsi_order);
    let price_troughs = find_peaks(&inverted_lows, cfg.rsi_order);
    let rsi_peaks = find_peaks(&rsi_values, cfg.rsi_order);
    let rsi_troughs = find_peaks(&inverted_rsi, cfg.rsi_order);

    let mut regular_bullish = vec![false; n];
    let mut regular_bearish = vec![false; n];
    let mut hidden_bullish = vec![false; n];
    let mut hidden_bearish = vec![false; n];

    let matched_troughs = match_extrema(&price_troughs, &rsi_troughs, cfg.rsi_order);
    for pair in matched_troughs.windows(2) {
        let (prev_p, prev_r) = pair[0];
        let (curr_p, curr_r) = pair[1];
        let p_prev = lows[prev_p].unwrap_or(Decimal::ZERO);
        let p_curr = lows[curr_p].unwrap_or(Decimal::ZERO);
        let r_prev = rsi_values[prev_r].unwrap_or(Decimal::ZERO);
        let r_curr = rsi_values[curr_r].unwrap_or(Decimal::ZERO);
        if p_curr < p_prev && r_curr > r_prev {
            regular_bullish[curr_p] = true;
        } else if p_curr > p_prev && r_curr < r_prev {
            hidden_bullish[curr_p] = true;
        }
    }

    let matched_peaks = match_extrema(&price_peaks, &rsi_peaks, cfg.rsi_order);
    for pair in matched_peaks.windows(2) {
        let (prev_p, prev_r) = pair[0];
        let (curr_p, curr_r) = pair[1];
        let p_prev = highs[prev_p].unwrap_or(Decimal::ZERO);
        let p_curr = highs[curr_p].unwrap_or(Decimal::ZERO);
        let r_prev = rsi_values[prev_r].unwrap_or(Decimal::ZERO);
        let r_curr = rsi_values[curr_r].unwrap_or(Decimal::ZERO);
        if p_curr > p_prev && r_curr < r_prev {
            regular_bearish[curr_p] = true;
        } else if p_curr < p_prev && r_curr > r_prev {
            hidden_bearish[curr_p] = true;
        }
    }

    Ok(Divergences {
        rsi: rsi_values,
        regular_bullish,
        regular_bearish,
        hidden_bullish,
        hidden_bearish,
    })
}

pub fn match_extrema(price: &[usize], rsi_idx: &[usize], order: usize) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let mut used = vec![false; rsi_idx.len()];
    let mut prev_rsi: Option<usize> = None;

    for &p_idx in price {
        let mut best_distance = usize::MAX;
        let mut best_slot: Option<usize> = None;
        for (slot, &r_idx) in rsi_idx.iter().enumerate() {
            if used[slot] {
                continue;
            }
            if let Some(prev) = prev_rsi {
                if r_idx <= prev {
                    continue;
                }
            }
            let distance = p_idx.abs_diff(r_idx);
            if distance <= order && distance < best_distance {
                best_distance = distance;
                best_slot = Some(slot);
            }
        }
        if let Some(slot) = best_slot {
            let r_idx = rsi_idx[slot];
            used[slot] = true;
            matches.push((p_idx, r_idx));
            prev_rsi = Some(r_idx);
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_extrema_greedily_consumes_the_nearest() {
        // Price 9 is left unmatched: price 6 greedily consumed RSI 7, the only remaining
        // extremum near it.
        assert_eq!(
            match_extrema(&[2, 6, 9], &[1, 4, 7], 5),
            vec![(2, 1), (6, 7)]
        );
    }

    #[test]
    fn matches_identical_extrema_in_order() {
        assert_eq!(
            match_extrema(&[1, 4, 7], &[1, 4, 7], 5),
            vec![(1, 1), (4, 4), (7, 7)]
        );
    }

    #[test]
    fn rejects_pairs_beyond_order() {
        assert_eq!(match_extrema(&[0], &[10], 5), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn ties_choose_the_first_rsi_extremum() {
        assert_eq!(match_extrema(&[5], &[4, 6], 5), vec![(5, 4)]);
    }

    #[test]
    fn rsi_extrema_cannot_be_reused() {
        assert_eq!(match_extrema(&[4, 5], &[5], 5), vec![(4, 5)]);
    }

    #[test]
    fn flat_series_has_no_divergences() {
        let candles: Vec<Candle> = (0..40)
            .map(|i| Candle {
                open_time_ms: i,
                open: Decimal::from(100),
                high: Decimal::from(100),
                low: Decimal::from(100),
                close: Decimal::from(100),
                volume: Decimal::ONE,
            })
            .collect();
        let out = detect(&candles, &Config::default()).unwrap();
        assert!(out.regular_bullish.iter().all(|v| !v));
        assert!(out.regular_bearish.iter().all(|v| !v));
        assert!(out.hidden_bullish.iter().all(|v| !v));
        assert!(out.hidden_bearish.iter().all(|v| !v));
    }
}
