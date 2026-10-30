use std::sync::Mutex;
use std::time::Duration;

const MINUTE: i64 = 60_000;

/// Share of the exchange's limit Fitil allows itself, leaving room for other apps on the same IP.
const BUDGET: f64 = 0.8;

/// Spaces out requests so the request weight stays under the exchange's per-minute limit.
///
/// The exchange reports the weight used so far in each response; that number, not a
/// local estimate, drives the waiting. Times are Unix milliseconds on the exchange's clock.
#[derive(Debug)]
pub struct RateLimiter {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    limit: u32,
    used: u32,
    minute: i64,
    blocked_until: i64,
}

impl RateLimiter {
    pub fn new(limit: u32) -> Self {
        Self {
            state: Mutex::new(State {
                limit,
                used: 0,
                minute: 0,
                blocked_until: 0,
            }),
        }
    }

    pub fn set_limit(&self, limit: u32) {
        self.lock().limit = limit;
    }

    /// Weight used in the current minute, and the limit.
    pub fn usage(&self, now: i64) -> (u32, u32) {
        let state = self.lock();
        (state.used_at(now), state.limit)
    }

    /// How long to wait before sending a request of `weight` at `now`.
    pub fn delay(&self, now: i64, weight: u32) -> Duration {
        let state = self.lock();
        if now < state.blocked_until {
            return millis(state.blocked_until - now);
        }
        let budget = f64::from(state.limit) * BUDGET;
        if f64::from(state.used_at(now) + weight) > budget {
            millis(now.div_euclid(MINUTE) * MINUTE + MINUTE - now)
        } else {
            Duration::ZERO
        }
    }

    /// Records the weight the exchange says has been used in the minute containing `now`.
    pub fn record(&self, now: i64, used: u32) {
        let mut state = self.lock();
        state.minute = now.div_euclid(MINUTE);
        state.used = used;
    }

    /// Holds back every request until `until`, after the exchange asked us to slow down.
    pub fn block_until(&self, until: i64) {
        let mut state = self.lock();
        state.blocked_until = state.blocked_until.max(until);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl State {
    fn used_at(&self, now: i64) -> u32 {
        if now.div_euclid(MINUTE) == self.minute {
            self.used
        } else {
            0
        }
    }
}

fn millis(ms: i64) -> Duration {
    Duration::from_millis(ms.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_793_390_400_000; // the start of a minute

    #[test]
    fn requests_go_out_while_under_budget() {
        let limiter = RateLimiter::new(1000);
        limiter.record(T + 1_000, 700);
        assert_eq!(limiter.delay(T + 2_000, 50), Duration::ZERO);
        assert_eq!(limiter.usage(T + 2_000), (700, 1000));
    }

    #[test]
    fn waits_for_the_next_minute_when_over_budget() {
        let limiter = RateLimiter::new(1000);
        limiter.record(T + 1_000, 790);
        assert_eq!(limiter.delay(T + 45_000, 20), Duration::from_secs(15));
    }

    #[test]
    fn usage_resets_every_minute() {
        let limiter = RateLimiter::new(1000);
        limiter.record(T + 1_000, 999);
        assert_eq!(limiter.usage(T + 61_000), (0, 1000));
        assert_eq!(limiter.delay(T + 61_000, 20), Duration::ZERO);
    }

    #[test]
    fn honours_a_block_from_the_exchange() {
        let limiter = RateLimiter::new(1000);
        limiter.block_until(T + 30_000);
        limiter.block_until(T + 10_000);
        assert_eq!(limiter.delay(T, 1), Duration::from_secs(30));
        assert_eq!(limiter.delay(T + 30_000, 1), Duration::ZERO);
    }
}
