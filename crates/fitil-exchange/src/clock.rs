use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// An exchange's clock, as an offset from the local one.
///
/// Countdowns and "is this candle closed" checks use the exchange's time, so they stay
/// right even when the computer's clock is a few seconds off.
#[derive(Debug, Default)]
pub struct Clock {
    /// Exchange clock minus local clock, in milliseconds.
    offset: AtomicI64,
}

impl Clock {
    /// Current time on the exchange's clock, in Unix milliseconds.
    pub fn now(&self) -> i64 {
        local_now() + self.offset()
    }

    pub fn offset(&self) -> i64 {
        self.offset.load(Ordering::Relaxed)
    }

    /// Records the exchange's `server_time`, read between the local times `sent` and
    /// `received`, and returns the new offset.
    pub fn record(&self, sent: i64, server_time: i64, received: i64) -> i64 {
        let offset = server_time - (sent + received) / 2;
        self.offset.store(offset, Ordering::Relaxed);
        offset
    }
}

/// The local clock, in Unix milliseconds.
pub fn local_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_assumes_the_server_answered_halfway() {
        let clock = Clock::default();
        assert_eq!(clock.record(1_000, 1_800, 1_200), 700);
        assert_eq!(clock.offset(), 700);
        assert!((clock.now() - local_now() - 700).abs() < 50);
    }
}
