//! Exchange clock (live spec §3a).
//!
//! `exchange_now = local monotonic + offset`, where the offset is measured against KuCoin's
//! `GET /api/v1/timestamp` and refreshed periodically. Using a monotonic base (rather than the wall
//! clock) means an NTP step on the host cannot make exchange time jump backwards between polls.
//! [`ManualClock`] is the injectable test double.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// A source of exchange time in UTC milliseconds.
pub trait Clock: Send + Sync {
    /// The current exchange time (UTC milliseconds).
    fn exchange_now_ms(&self) -> i64;

    /// Adjust the clock to an observed exchange time. A no-op for a clock that is not offset-based;
    /// the supervisor calls this through the trait object when refreshing the offset.
    fn sync(&self, _exchange_ms: i64) {}
}

/// Wall-clock milliseconds since the Unix epoch (used only to seed the monotonic base).
fn wall_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Monotonic exchange clock. Construct once, then [`ExchangeClock::sync`] to the exchange's time
/// endpoint periodically (an `offset_ms` of 0 means "assume the host clock is correct").
#[derive(Debug)]
pub struct ExchangeClock {
    base: Instant,
    base_wall_ms: i64,
    offset_ms: AtomicI64,
}

impl Default for ExchangeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ExchangeClock {
    pub fn new() -> Self {
        ExchangeClock {
            base: Instant::now(),
            base_wall_ms: wall_now_ms(),
            offset_ms: AtomicI64::new(0),
        }
    }

    /// Local monotonic time in ms (before the exchange offset).
    fn monotonic_now_ms(&self) -> i64 {
        self.base_wall_ms
            .saturating_add(self.base.elapsed().as_millis() as i64)
    }

    /// Measure the offset against a fresh exchange time reading. Safe to call concurrently.
    pub fn sync(&self, exchange_ms: i64) {
        let offset = exchange_ms.saturating_sub(self.monotonic_now_ms());
        self.offset_ms.store(offset, Ordering::Relaxed);
    }

    /// The current measured offset in ms.
    pub fn offset_ms(&self) -> i64 {
        self.offset_ms.load(Ordering::Relaxed)
    }
}

impl Clock for ExchangeClock {
    fn exchange_now_ms(&self) -> i64 {
        self.monotonic_now_ms()
            .saturating_add(self.offset_ms.load(Ordering::Relaxed))
    }

    fn sync(&self, exchange_ms: i64) {
        // Delegate to the inherent method so the trait object can refresh the offset.
        ExchangeClock::sync(self, exchange_ms);
    }
}

/// Injectable clock for tests (and the offline replay smoke): a settable UTC-ms value.
#[derive(Debug)]
pub struct ManualClock {
    now_ms: AtomicI64,
}

impl ManualClock {
    pub fn new(now_ms: i64) -> Self {
        ManualClock {
            now_ms: AtomicI64::new(now_ms),
        }
    }

    /// Set the current exchange time.
    pub fn set(&self, now_ms: i64) {
        self.now_ms.store(now_ms, Ordering::Relaxed);
    }

    /// Advance the current exchange time by `delta_ms`.
    pub fn advance(&self, delta_ms: i64) {
        self.now_ms.fetch_add(delta_ms, Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn exchange_now_ms(&self) -> i64 {
        self.now_ms.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_is_settable_and_advanceable() {
        let c = ManualClock::new(1_000);
        assert_eq!(c.exchange_now_ms(), 1_000);
        c.set(5_000);
        assert_eq!(c.exchange_now_ms(), 5_000);
        c.advance(500);
        assert_eq!(c.exchange_now_ms(), 5_500);
    }

    #[test]
    fn exchange_clock_applies_a_measured_offset() {
        let c = ExchangeClock::new();
        let raw = c.monotonic_now_ms();
        // Pretend the exchange is 12 s ahead of the local monotonic reading.
        c.sync(raw + 12_000);
        assert_eq!(c.offset_ms(), 12_000);
        let now = c.exchange_now_ms();
        assert!(
            (now - (raw + 12_000)).abs() < 2_000,
            "exchange now {now} should be near raw+offset"
        );
    }

    #[test]
    fn uncorrected_clock_tracks_monotonic_time() {
        let c = ExchangeClock::new();
        assert_eq!(c.offset_ms(), 0);
        // No sync: exchange now is just the local monotonic reading.
        let a = c.exchange_now_ms();
        let b = c.exchange_now_ms();
        assert!(b >= a);
    }
}
