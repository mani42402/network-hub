//! Exponential backoff with jitter, for reconciler retries.
//!
//! When a reconcile fails, retrying immediately in a tight loop burns CPU and
//! hammers whatever is failing. Backoff spreads retries out exponentially;
//! jitter spreads *many* reconcilers' retries apart so they do not thunder
//! together after a shared outage (e.g. sleep/resume). Jitter is configurable to
//! zero so tests are deterministic.

use std::time::Duration;

/// Computes retry delays: `base * 2^(attempt-1)`, capped at `max`, optionally
/// with up to `jitter_fraction` of random reduction.
#[derive(Debug, Clone)]
pub struct Backoff {
    base: Duration,
    max: Duration,
    /// 0.0 = no jitter (deterministic, for tests); 0.5 = up to 50% earlier.
    jitter_fraction: f64,
    attempt: u32,
}

impl Backoff {
    /// A backoff starting at `base`, doubling to a ceiling of `max`, with the
    /// given jitter fraction (0.0–1.0).
    #[must_use]
    pub fn new(base: Duration, max: Duration, jitter_fraction: f64) -> Self {
        Self {
            base,
            max,
            jitter_fraction: jitter_fraction.clamp(0.0, 1.0),
            attempt: 0,
        }
    }

    /// A sensible default for reconcilers: 500ms base, 60s cap, 50% jitter.
    #[must_use]
    pub fn reconciler_default() -> Self {
        Self::new(Duration::from_millis(500), Duration::from_mins(1), 0.5)
    }

    /// Returns the next delay and advances the attempt counter.
    pub fn next_delay(&mut self) -> Duration {
        self.attempt = self.attempt.saturating_add(1);
        // 2^(attempt-1), saturating so a long-failing reconciler does not
        // overflow — it just pins at `max`.
        let exp = self.attempt.saturating_sub(1).min(32);
        let scaled = self.base.saturating_mul(2u32.saturating_pow(exp));
        let capped = scaled.min(self.max);

        if self.jitter_fraction == 0.0 {
            return capped;
        }
        // Full-jitter style: subtract up to jitter_fraction of the delay.
        let reduction = capped.mul_f64(self.jitter_fraction * fastrand::f64());
        capped.saturating_sub(reduction)
    }

    /// Resets the counter after a success, so the next failure starts from
    /// `base` again.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// How many delays have been produced since the last reset.
    #[must_use]
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_jitter_it_doubles_and_caps() {
        let mut b = Backoff::new(Duration::from_millis(100), Duration::from_millis(800), 0.0);
        assert_eq!(b.next_delay(), Duration::from_millis(100)); // 100 * 2^0
        assert_eq!(b.next_delay(), Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(b.next_delay(), Duration::from_millis(400)); // 100 * 2^2
        assert_eq!(b.next_delay(), Duration::from_millis(800)); // 100 * 2^3
        assert_eq!(b.next_delay(), Duration::from_millis(800)); // capped
    }

    #[test]
    fn reset_returns_to_base() {
        let mut b = Backoff::new(Duration::from_millis(100), Duration::from_secs(10), 0.0);
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt(), 2);
        b.reset();
        assert_eq!(b.attempt(), 0);
        assert_eq!(b.next_delay(), Duration::from_millis(100));
    }

    #[test]
    fn jitter_only_ever_reduces_below_the_cap() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(1), 0.5);
        for _ in 0..100 {
            let d = b.next_delay();
            // With 50% jitter the delay is in (500ms, 1000ms].
            assert!(d <= Duration::from_secs(1));
            assert!(d >= Duration::from_millis(500));
            b.reset();
        }
    }

    #[test]
    fn does_not_overflow_after_many_attempts() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_mins(1), 0.0);
        for _ in 0..1000 {
            let d = b.next_delay();
            assert!(d <= Duration::from_mins(1));
        }
    }
}
