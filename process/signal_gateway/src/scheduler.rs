use chrono::{DateTime, TimeDelta, Timelike, Utc};

pub const CANDLE_HOURS: i64 = 4;

/// Start of the 4h window containing `now`.
pub fn floor_to_4h(now: DateTime<Utc>) -> DateTime<Utc> {
    let hour = (now.hour() as i64 / CANDLE_HOURS) * CANDLE_HOURS;
    now.date_naive()
        .and_hms_opt(hour as u32, 0, 0)
        .expect("4h boundary is valid")
        .and_utc()
}

/// Open time of the most recent candle that has fully closed.
pub fn most_recent_closed_candle_open(now: DateTime<Utc>) -> DateTime<Utc> {
    floor_to_4h(now) - TimeDelta::hours(CANDLE_HOURS)
}

/// Close time of a candle that opened at `open`.
pub fn candle_close(open: DateTime<Utc>) -> DateTime<Utc> {
    open + TimeDelta::hours(CANDLE_HOURS)
}

/// Next wake time: the next 4h boundary plus `delay`.
pub fn next_close_after(now: DateTime<Utc>, delay: TimeDelta) -> DateTime<Utc> {
    floor_to_4h(now) + TimeDelta::hours(CANDLE_HOURS) + delay
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 7, hour, minute, second)
            .unwrap()
    }

    #[test]
    fn closed_candle_just_after_midnight_close() {
        assert_eq!(
            most_recent_closed_candle_open(at(0, 1, 30)),
            Utc.with_ymd_and_hms(2026, 11, 6, 20, 0, 0).unwrap()
        );
    }

    #[test]
    fn closed_candle_after_four_am_close() {
        assert_eq!(
            most_recent_closed_candle_open(at(4, 1, 30)),
            Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn next_close_is_the_following_boundary_plus_delay() {
        let delay = TimeDelta::seconds(90);
        assert_eq!(next_close_after(at(4, 1, 30), delay), at(8, 1, 30));
        assert_eq!(next_close_after(at(3, 59, 0), delay), at(4, 1, 30));
    }

    #[test]
    fn candle_close_is_four_hours_later() {
        assert_eq!(candle_close(at(0, 0, 0)), at(4, 0, 0));
    }
}
