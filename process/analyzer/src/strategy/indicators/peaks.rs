use rust_decimal::Decimal;

/// A `Decimal`/`Option` port of `scipy.signal.find_peaks(values, distance=distance)`.
/// `None` behaves like NaN: it cannot itself be a peak, and a `None` adjacent to a candidate
/// suppresses that peak (scipy does the same for NaN). `distance <= 1` is treated as no filtering;
/// scipy rejects `distance < 1`, and the core config guarantees `>= 1`.
///
/// Distance filtering ports NumPy's scalar `aquicksort_` argsort order used by SciPy's
/// `_select_by_peak_distance`. The oracle pins `NPY_DISABLE_CPU_FEATURES`, so both sides use the
/// scalar path and resolve equal-height peaks identically.
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
            // A plateau is a peak only when the value after it is lower. Report its midpoint
            // (rounded down) and jump past the plateau, which cannot itself be a peak.
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

const SMALL_QUICKSORT: usize = 15;

fn select_by_distance(peaks: &[usize], values: &[Option<Decimal>], distance: usize) -> Vec<usize> {
    let order = numpy_argsort_order(peaks, values);

    let mut kept: Vec<usize> = Vec::new();
    // SciPy processes priorities from the end of the ascending argsort.
    for &index in order.iter().rev() {
        let position = peaks[index];
        // Descending priority means any higher-priority neighbour was already processed, so
        // keeping a peak only when it is far from every already-kept peak reproduces scipy's
        // remove-within-distance pass.
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

fn numpy_argsort_order(peaks: &[usize], values: &[Option<Decimal>]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..peaks.len()).collect();
    numpy_aquicksort(&mut order, peaks, values);
    order
}

/// Port of NumPy's scalar `aquicksort_` from `npysort/quicksort.cpp`.
fn numpy_aquicksort(order: &mut [usize], peaks: &[usize], values: &[Option<Decimal>]) {
    let n = order.len();
    if n == 0 {
        return;
    }

    let mut pl = 0usize;
    let mut pr = n - 1;
    let mut stack: Vec<(usize, usize, i32)> = Vec::new();
    let mut cdepth = (numpy_msb(n) * 2) as i32;

    loop {
        if cdepth < 0 {
            numpy_aheapsort(&mut order[pl..=pr], peaks, values);
        } else {
            while pr - pl > SMALL_QUICKSORT {
                // Median-of-three pivot selection.
                let pm = pl + ((pr - pl) >> 1);
                if less(order[pm], order[pl], peaks, values) {
                    order.swap(pm, pl);
                }
                if less(order[pr], order[pm], peaks, values) {
                    order.swap(pr, pm);
                }
                if less(order[pm], order[pl], peaks, values) {
                    order.swap(pm, pl);
                }

                let pivot = order[pm];
                let mut pi = pl as isize;
                let mut pj = (pr - 1) as isize;
                order.swap(pm, pr - 1);

                loop {
                    pi += 1;
                    while less(order[pi as usize], pivot, peaks, values) {
                        pi += 1;
                    }
                    pj -= 1;
                    while less(pivot, order[pj as usize], peaks, values) {
                        pj -= 1;
                    }
                    if pi >= pj {
                        break;
                    }
                    order.swap(pi as usize, pj as usize);
                }

                let pk = pr - 1;
                order.swap(pi as usize, pk);

                // Push the larger partition and continue with the smaller one.
                let next_depth = cdepth - 1;
                if (pi as usize) - pl < pr - (pi as usize) {
                    stack.push(((pi + 1) as usize, pr, next_depth));
                    pr = (pi - 1) as usize;
                } else {
                    stack.push((pl, (pi - 1) as usize, next_depth));
                    pl = (pi + 1) as usize;
                }
                cdepth = next_depth;
            }

            for pi in pl + 1..=pr {
                let vi = order[pi];
                let mut pj = pi;
                let mut pk = pi - 1;
                while pj > pl && less(vi, order[pk], peaks, values) {
                    order[pj] = order[pk];
                    pj -= 1;
                    pk = pj.saturating_sub(1);
                }
                order[pj] = vi;
            }
        }

        match stack.pop() {
            Some((next_pl, next_pr, next_depth)) => {
                pl = next_pl;
                pr = next_pr;
                cdepth = next_depth;
            }
            None => break,
        }
    }
}

/// Port of NumPy's scalar `aheapsort_` from `npysort/npysort_heapsort.hpp`.
fn numpy_aheapsort(order: &mut [usize], peaks: &[usize], values: &[Option<Decimal>]) {
    let n = order.len();
    if n <= 1 {
        return;
    }

    // NumPy uses 1-based indexing for the heap; `order[i - 1]` is `a[i]`.
    for l in (1..=n / 2).rev() {
        let tmp = order[l - 1];
        let mut i = l;
        let mut j = l << 1;
        while j <= n {
            if j < n && less(order[j - 1], order[j], peaks, values) {
                j += 1;
            }
            if less(tmp, order[j - 1], peaks, values) {
                order[i - 1] = order[j - 1];
                i = j;
                j += j;
            } else {
                break;
            }
        }
        order[i - 1] = tmp;
    }

    let mut remaining = n;
    while remaining > 1 {
        let tmp = order[remaining - 1];
        order[remaining - 1] = order[0];
        remaining -= 1;

        let mut i = 1;
        let mut j = 2;
        while j <= remaining {
            if j < remaining && less(order[j - 1], order[j], peaks, values) {
                j += 1;
            }
            if less(tmp, order[j - 1], peaks, values) {
                order[i - 1] = order[j - 1];
                i = j;
                j += j;
            } else {
                break;
            }
        }
        order[i - 1] = tmp;
    }
}

fn less(a: usize, b: usize, peaks: &[usize], values: &[Option<Decimal>]) -> bool {
    // Peaks always index `Some` values; the zero fallback is defensive only.
    values[peaks[a]].unwrap_or(Decimal::ZERO) < values[peaks[b]].unwrap_or(Decimal::ZERO)
}

fn numpy_msb(n: usize) -> u32 {
    usize::BITS - 1 - n.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn vals(v: &[i64]) -> Vec<Option<Decimal>> {
        v.iter().map(|x| Some(Decimal::from(*x))).collect()
    }

    fn opts(v: &[Option<i64>]) -> Vec<Option<Decimal>> {
        v.iter().map(|x| x.map(Decimal::from)).collect()
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
    fn distance_filtering_prefers_higher_priority() {
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
    fn equal_priority_tie_break_matches_scalar_numpy() {
        // Expected values are scipy with NPY_DISABLE_CPU_FEATURES pinning scalar argsort.
        let first = [0, 0, 4, 5, 4, 1, 5, 2, 1, 3, 0, 6, 0, 4, 1];
        assert_eq!(find_peaks(&vals(&first), 4), vec![6, 11]);

        let second = [0, 2, 3, 1, 1, 3, 2, 0, 0, 2, 1, 0, 2, 0];
        assert_eq!(find_peaks(&vals(&second), 4), vec![5, 12]);
    }

    #[test]
    fn numpy_aquicksort_matches_scalar_numpy_ties() {
        let values = vals(&[2, 1, 3, 0, 0, 0, 2, 0, 1, 0, 0, 3, 3, 0, 1, 0, 3]);
        let peaks: Vec<usize> = (0..values.len()).collect();
        let mut order: Vec<usize> = (0..values.len()).collect();
        numpy_aquicksort(&mut order, &peaks, &values);
        assert_eq!(
            order,
            vec![3, 4, 5, 13, 7, 15, 9, 10, 8, 1, 14, 0, 6, 12, 2, 11, 16],
            "scalar numpy quicksort argsort permutation"
        );
    }

    #[test]
    fn numpy_aheapsort_matches_scalar_numpy_ties() {
        let values = vals(&[5, 5, 5, 1, 3, 3, 0, 2, 2]);
        let peaks: Vec<usize> = (0..values.len()).collect();
        let mut order: Vec<usize> = (0..values.len()).collect();
        numpy_aheapsort(&mut order, &peaks, &values);
        assert_eq!(
            order,
            vec![6, 3, 7, 8, 5, 4, 2, 1, 0],
            "scalar numpy heapsort argsort permutation"
        );
    }

    #[test]
    fn distance_filtering_matches_scalar_numpy_on_tied_random_arrays() {
        // Generated once with scipy.signal.find_peaks on the pinned scalar argsort path.
        let cases: &[(&[i64], usize, &[usize])] = &[
            (
                &[
                    1, 4, 3, 5, 0, 2, 5, 3, 1, 1, 4, 2, 3, 2, 4, 4, 0, 4, 3, 5, 1, 4, 2, 5, 2, 3,
                    2, 4, 0, 3, 3, 2, 4, 1, 2, 0, 4, 2, 5, 1,
                ],
                7,
                &[3, 10, 19, 27, 38],
            ),
            (
                &[
                    2, 2, 5, 3, 5, 2, 4, 4, 2, 1, 1, 0, 4, 1, 2, 1, 4, 0, 2, 3, 0, 2, 0, 4, 1, 5,
                    1, 3, 3, 0, 4, 1, 2, 2, 5, 0, 0, 1, 0, 4, 4, 4, 2, 0, 4, 2, 0, 4, 4, 3,
                ],
                3,
                &[2, 6, 12, 16, 19, 25, 30, 34, 37, 40, 44, 47],
            ),
            (
                &[
                    5, 5, 4, 5, 2, 1, 3, 2, 3, 5, 2, 5, 5, 1, 1, 0, 4, 1, 1, 3, 2, 1, 3, 1, 1, 3,
                    3, 2, 1, 4, 2, 0, 5, 0, 5, 5, 1, 1, 4, 5, 4, 1, 4, 1, 0, 2, 4, 5, 2, 1, 4, 4,
                    3, 1, 4, 1, 5, 0, 2, 0,
                ],
                3,
                &[3, 6, 9, 16, 19, 22, 25, 29, 34, 39, 42, 47, 50, 56],
            ),
        ];

        for (values, distance, expected) in cases {
            assert_eq!(
                find_peaks(&vals(values), *distance),
                expected.to_vec(),
                "values={values:?}, distance={distance}"
            );
        }
    }

    #[test]
    fn none_values_suppress_adjacent_candidates() {
        // Expected values are the verified scipy 1.16.1 outputs.
        assert_eq!(
            find_peaks(&opts(&[Some(0), Some(5), None, Some(0)]), 1),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_peaks(&opts(&[None, Some(5), Some(0)]), 1),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_peaks(
                &opts(&[Some(0), Some(5), Some(0), None, Some(0), Some(5), Some(0)]),
                1
            ),
            vec![1, 5]
        );
        assert_eq!(
            find_peaks(&opts(&[Some(0), Some(1), Some(1), None, Some(0)]), 1),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_peaks(&opts(&[Some(0), None, Some(1), Some(1), Some(0)]), 1),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_peaks(
                &opts(&[Some(0), Some(1), Some(1), None, Some(2), Some(1), Some(0)]),
                1
            ),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_peaks(
                &opts(&[Some(0), None, Some(1), Some(1), Some(2), Some(1), Some(0)]),
                1
            ),
            vec![4]
        );
    }
}
