//! Detecting a losing fight with another network agent (§5.3, F-6).
//!
//! If something external — firewalld, an antivirus product, a policy script —
//! keeps reverting NetworkHub's state, a naive reconciler re-applies forever,
//! burning CPU and filling logs. Instead the runtime counts how often it has to
//! re-converge state it had already converged (a "restore cycle"). After too many
//! in a short window it stops fighting, marks the module *conflicted*, and names
//! the likely suspect, so the user gets a diagnosable message instead of a
//! silent war.
//!
//! Uses [`tokio::time::Instant`], which tests can control with
//! `tokio::time::pause`/`advance`, so the window behaviour is deterministic.

use std::collections::VecDeque;
use std::time::Duration;

use tokio::time::Instant;

/// A sliding window of restore-cycle timestamps.
#[derive(Debug)]
pub(crate) struct InterferenceTracker {
    threshold: u32,
    window: Duration,
    events: VecDeque<Instant>,
}

impl InterferenceTracker {
    pub(crate) fn new(threshold: u32, window: Duration) -> Self {
        Self {
            threshold,
            window,
            events: VecDeque::new(),
        }
    }

    /// Records a restore cycle at `now` and returns whether the interference
    /// threshold has been reached within the window.
    pub(crate) fn record(&mut self, now: Instant) -> bool {
        self.events.push_back(now);
        self.evict_older_than(now);
        // Compare in usize to avoid a lossy usize->u32 cast; the window can never
        // hold more events than the threshold plus a handful anyway.
        self.events.len() >= self.threshold as usize
    }

    /// Clears the history — called when desired state legitimately changes, so a
    /// user editing config is never mistaken for interference.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
    }

    fn evict_older_than(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window);
        while let Some(&front) = self.events.front() {
            match cutoff {
                Some(cutoff) if front < cutoff => {
                    self.events.pop_front();
                }
                _ => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn triggers_after_threshold_within_window() {
        let mut t = InterferenceTracker::new(3, Duration::from_mins(1));
        let start = Instant::now();
        assert!(!t.record(start));
        assert!(!t.record(start + Duration::from_secs(10)));
        // Third within the window → interference.
        assert!(t.record(start + Duration::from_secs(20)));
    }

    #[tokio::test(start_paused = true)]
    async fn old_events_fall_out_of_the_window() {
        let mut t = InterferenceTracker::new(3, Duration::from_mins(1));
        let start = Instant::now();
        t.record(start);
        t.record(start + Duration::from_secs(1));
        // The third is 61s after the first, so the first has aged out: only two
        // remain in the window, below the threshold.
        assert!(!t.record(start + Duration::from_secs(61)));
    }

    #[tokio::test(start_paused = true)]
    async fn clear_resets_the_count() {
        let mut t = InterferenceTracker::new(2, Duration::from_mins(1));
        let start = Instant::now();
        t.record(start);
        t.clear();
        // After clear, a single event is below threshold again.
        assert!(!t.record(start + Duration::from_secs(1)));
    }
}
