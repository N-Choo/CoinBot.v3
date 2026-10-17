/// Plan time-window requests that page KuCoin's 1500-record cap.
pub fn page_windows(start_s: i64, end_s: i64, interval_s: i64) -> Vec<(i64, i64)> {
    let step = 1500 * interval_s;
    let mut out = Vec::new();
    let mut s = start_s;
    while s < end_s {
        let e = (s + step).min(end_s);
        out.push((s, e));
        s = e;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_respect_1500_cap() {
        let p = page_windows(0, 3000 * 60, 60);
        assert_eq!(p, vec![(0, 1500 * 60), (1500 * 60, 3000 * 60)]);
    }
}
