use std::sync::Mutex;
use std::time::Duration;

use crate::RequestUsage;

/// Share of the exchange's limit Fitil allows itself, leaving room for other apps on the same IP.
const BUDGET: f64 = 0.8;

/// Spaces out requests so their cost stays under an exchange's limit per time window
/// (Binance: request weight per minute; others count requests per few seconds).
///
/// When the exchange reports the cost used so far, that number drives the waiting.
/// Times are Unix milliseconds on the exchange's clock.
#[derive(Debug)]
pub struct RateLimiter {
    window: i64,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    limit: u32,
    used: u32,
    /// Index of the window `used` belongs to.
    window: i64,
    blocked_until: i64,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            window: window.as_millis().max(1) as i64,
            state: Mutex::new(State {
                limit,
                used: 0,
                window: 0,
                blocked_until: 0,
            }),
        }
    }

    pub fn set_limit(&self, limit: u32) {
        self.lock().limit = limit;
    }

    /// Cost used in the window containing `now`, and the limit.
    pub fn usage(&self, now: i64) -> RequestUsage {
        let state = self.lock();
        RequestUsage {
            used: self.used_at(&state, now),
            limit: state.limit,
        }
    }

    /// How long to wait before sending a request costing `cost` at `now`.
    pub fn delay(&self, now: i64, cost: u32) -> Duration {
        let state = self.lock();
        if now < state.blocked_until {
            return millis(state.blocked_until - now);
        }
        let budget = f64::from(state.limit) * BUDGET;
        if f64::from(self.used_at(&state, now) + cost) > budget {
            millis(now.div_euclid(self.window) * self.window + self.window - now)
        } else {
            Duration::ZERO
        }
    }

    /// Records the cost the exchange says has been used in the window containing `now`.
    pub fn record(&self, now: i64, used: u32) {
        let mut state = self.lock();
        state.window = now.div_euclid(self.window);
        state.used = used;
    }

    /// Adds `cost` to the window containing `now`, for exchanges that do not report usage.
    pub fn spend(&self, now: i64, cost: u32) {
        let mut state = self.lock();
        let used = self.used_at(&state, now);
        state.window = now.div_euclid(self.window);
        state.used = used + cost;
    }

    /// Holds back every request until `until`, after the exchange asked us to slow down.
    pub fn block_until(&self, until: i64) {
        let mut state = self.lock();
        state.blocked_until = state.blocked_until.max(until);
    }

    fn used_at(&self, state: &State, now: i64) -> u32 {
        if now.div_euclid(self.window) == state.window {
            state.used
        } else {
            0
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn millis(ms: i64) -> Duration {
    Duration::from_millis(ms.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_793_390_400_000; // the start of a minute
    const MINUTE: Duration = Duration::from_secs(60);

    fn usage(used: u32, limit: u32) -> RequestUsage {
        RequestUsage { used, limit }
    }

    #[test]
    fn requests_go_out_while_under_budget() {
        let limiter = RateLimiter::new(1000, MINUTE);
        limiter.record(T + 1_000, 700);
        assert_eq!(limiter.delay(T + 2_000, 50), Duration::ZERO);
        assert_eq!(limiter.usage(T + 2_000), usage(700, 1000));
    }

    #[test]
    fn waits_for_the_next_window_when_over_budget() {
        let limiter = RateLimiter::new(1000, MINUTE);
        limiter.record(T + 1_000, 790);
        assert_eq!(limiter.delay(T + 45_000, 20), Duration::from_secs(15));
    }

    #[test]
    fn usage_resets_every_window() {
        let limiter = RateLimiter::new(1000, MINUTE);
        limiter.record(T + 1_000, 999);
        assert_eq!(limiter.usage(T + 61_000), usage(0, 1000));
        assert_eq!(limiter.delay(T + 61_000, 20), Duration::ZERO);
    }

    #[test]
    fn counts_locally_when_the_exchange_does_not_report() {
        let limiter = RateLimiter::new(20, Duration::from_secs(2));
        for _ in 0..16 {
            limiter.spend(T + 500, 1);
        }
        assert_eq!(limiter.usage(T + 500), usage(16, 20));
        assert_eq!(limiter.delay(T + 500, 1), Duration::from_millis(1_500));
        assert_eq!(limiter.delay(T + 2_000, 1), Duration::ZERO);
    }

    #[test]
    fn honours_a_block_from_the_exchange() {
        let limiter = RateLimiter::new(1000, MINUTE);
        limiter.block_until(T + 30_000);
        limiter.block_until(T + 10_000);
        assert_eq!(limiter.delay(T, 1), Duration::from_secs(30));
        assert_eq!(limiter.delay(T + 30_000, 1), Duration::ZERO);
    }
}
