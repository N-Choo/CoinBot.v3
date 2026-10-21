use rust_decimal::Decimal;

/// A `Decimal`/`Option` port of `scipy.signal.find_peaks(values, distance=distance)`.
/// `None` behaves like NaN: it never forms a peak and never blocks one.
///
/// It matches scipy exactly when peak priorities (heights) are distinct. For equal-priority
/// peaks, scipy processes them in the order produced by numpy's unstable `argsort`, which is an
/// implementation detail rather than part of scipy's API contract; this port instead uses a
/// deterministic descending-priority / descending-index tie-break. Real KuCoin data (12
/// symbol/timeframe datasets checked) produced no equal-priority peak pairs within `distance`, so
/// fixtures are unaffected by that difference.
pub fn find_peaks(values: &[Option<Decimal>], distance: usize) -> Vec<usize> {
    let peaks = local_maxima(values);
    if distance <= 1 {
        return peaks;
    }
    select_by_distance(&peaks, values, distance)
}

fn local_maxima(values: &[Option<Decimal>]) -> Vec<usize> {
    let n = values.len();
    let mut peaks = Vec::new();
    let mut i = 1usize;
    while i + 1 < n {
        let (prev, cur, next) = (values[i - 1], values[i], values[i + 1]);
        let (Some(prev), Some(cur), Some(next)) = (prev, cur, next) else {
            i += 1;
            continue;
        };
        if cur > prev && cur > next {
            peaks.push(i);
            i += 1;
        } else if cur > prev && cur == next {
            let mut j = i + 1;
            while j + 1 < n && values[j] == Some(cur) && values[j + 1] == Some(cur) {
                j += 1;
            }
            if let Some(after) = values.get(j + 1).copied().flatten() {
                if after < cur {
                    peaks.push(i + (j - i) / 2);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    peaks
}

fn select_by_distance(peaks: &[usize], values: &[Option<Decimal>], distance: usize) -> Vec<usize> {
    use std::cmp::Ordering;
    let mut order: Vec<usize> = (0..peaks.len()).collect();
    order.sort_by(|&a, &b| {
        let va = values[peaks[a]].unwrap_or(Decimal::ZERO);
        let vb = values[peaks[b]].unwrap_or(Decimal::ZERO);
        match va.cmp(&vb) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => peaks[b].cmp(&peaks[a]),
        }
    });

    let mut kept: Vec<usize> = Vec::new();
    for &index in &order {
        let position = peaks[index];
        if kept
            .iter()
            .all(|&k| position.abs_diff(peaks[k]) >= distance)
        {
            kept.push(index);
        }
    }

    let mut result: Vec<usize> = kept.into_iter().map(|i| peaks[i]).collect();
    result.sort_unstable();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn vals(v: &[i64]) -> Vec<Option<Decimal>> {
        v.iter().map(|x| Some(Decimal::from(*x))).collect()
    }

    #[test]
    fn plateau_reports_the_middle() {
        assert_eq!(find_peaks(&vals(&[0, 1, 1, 1, 0]), 1), vec![2]);
        assert_eq!(find_peaks(&vals(&[0, 0, 1, 1, 1, 0, 0]), 1), vec![3]);
        assert_eq!(find_peaks(&vals(&[0, 1, 1, 1, 1, 0]), 1), vec![2]);
    }

    #[test]
    fn endpoints_and_trailing_plateaus_are_not_peaks() {
        assert_eq!(find_peaks(&vals(&[1, 1, 1, 0]), 1), Vec::<usize>::new());
        assert_eq!(find_peaks(&vals(&[0, 1, 1, 1]), 1), Vec::<usize>::new());
    }

    #[test]
    fn strict_peaks_are_reported() {
        assert_eq!(find_peaks(&vals(&[0, 1, 1, 2, 1, 1, 0]), 1), vec![3]);
        assert_eq!(find_peaks(&vals(&[0, 5, 4, 3, 4, 5, 0]), 1), vec![1, 5]);
    }

    #[test]
    fn distance_removes_smaller_peaks_first() {
        assert_eq!(find_peaks(&vals(&[0, 3, 2, 3, 0]), 2), vec![1, 3]);
        assert_eq!(find_peaks(&vals(&[0, 3, 2, 3, 0]), 3), vec![3]);
        assert_eq!(find_peaks(&vals(&[0, 3, 0, 3, 0, 3, 0]), 3), vec![1, 5]);
        assert_eq!(
            find_peaks(&vals(&[0, 3, 0, 3, 0, 3, 0, 3, 0]), 3),
            vec![3, 7]
        );
    }

    #[test]
    fn equal_height_ties_keep_the_later_peak() {
        assert_eq!(find_peaks(&vals(&[0, 3, 0, 0, 3, 0]), 4), vec![4]);
        assert_eq!(find_peaks(&vals(&[0, 3, 0, 0, 0, 3, 0]), 5), vec![5]);
    }

    #[test]
    fn equal_priority_tie_break_is_deterministic() {
        // scipy's unstable `argsort` processes these equal-height peaks in a different order and
        // returns [3, 11]; this port's descending-index tie-break keeps peak 6 and returns [6, 11].
        let first = [0, 0, 4, 5, 4, 1, 5, 2, 1, 3, 0, 6, 0, 4, 1];
        assert_eq!(find_peaks(&vals(&first), 4), vec![6, 11]);

        // Likewise scipy returns [2, 9] here; the deterministic tie-break yields [5, 12].
        let second = [0, 2, 3, 1, 1, 3, 2, 0, 0, 2, 1, 0, 2, 0];
        assert_eq!(find_peaks(&vals(&second), 4), vec![5, 12]);
    }
}
