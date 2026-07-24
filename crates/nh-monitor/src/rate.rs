//! Rate derivation from raw interface counters (WP-M0.10).
//!
//! The two resilience rules the exit criteria demand live here, in a pure
//! function that is trivial to test:
//!
//! - **Monotonic deltas (E-26, F-11).** The caller measures the interval with a
//!   monotonic clock ([`std::time::Instant`]) and passes it as a [`Duration`],
//!   so a wall-clock step (`date -s`, NTP correction) can never turn into an
//!   absurd rate — the interval is simply unaffected.
//! - **Counter reset/wrap (E-25).** A counter that goes *down* between samples —
//!   an interface reset, or (in theory) a wrap — yields zero for that field, not
//!   a negative or a spurious spike. Interface counters are 64-bit on every
//!   platform, so a real wrap is not reachable in practice; a reset is, and zero
//!   is the safe reading for the straddling interval.

use std::time::Duration;

use nh_pal::InterfaceCounters;
use serde::{Deserialize, Serialize};

/// A derived throughput rate for one interface over one sampling interval.
///
/// Values are bytes/packets per second. The presentation layer converts to
/// bits/sec or human units; the monitor keeps the raw rate.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rate {
    /// Received bytes per second.
    pub rx_bytes_per_sec: f64,
    /// Transmitted bytes per second.
    pub tx_bytes_per_sec: f64,
    /// Received packets per second.
    pub rx_packets_per_sec: f64,
    /// Transmitted packets per second.
    pub tx_packets_per_sec: f64,
}

/// Derives a [`Rate`] from two counter readings and the monotonic interval
/// between them.
///
/// A non-positive interval (two samples at effectively the same instant) yields
/// a zero rate rather than a division by zero.
#[must_use]
pub fn derive_rate(prev: &InterfaceCounters, cur: &InterfaceCounters, dt: Duration) -> Rate {
    let secs = dt.as_secs_f64();
    if secs <= 0.0 {
        return Rate::default();
    }
    Rate {
        rx_bytes_per_sec: per_sec(cur.rx_bytes, prev.rx_bytes, secs),
        tx_bytes_per_sec: per_sec(cur.tx_bytes, prev.tx_bytes, secs),
        rx_packets_per_sec: per_sec(cur.rx_packets, prev.rx_packets, secs),
        tx_packets_per_sec: per_sec(cur.tx_packets, prev.tx_packets, secs),
    }
}

/// `(cur - prev) / secs`, clamped so a counter decrease reads as zero (E-25).
// Precision loss only matters past 2^53 bytes/interval (petabytes per second);
// rates are inherently approximate, so the cast is fine.
#[allow(clippy::cast_precision_loss)]
fn per_sec(cur: u64, prev: u64, secs: f64) -> f64 {
    cur.saturating_sub(prev) as f64 / secs
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn counters(rx_bytes: u64, tx_bytes: u64, rx_pkts: u64, tx_pkts: u64) -> InterfaceCounters {
        InterfaceCounters {
            rx_bytes,
            tx_bytes,
            rx_packets: rx_pkts,
            tx_packets: tx_pkts,
            ..InterfaceCounters::default()
        }
    }

    #[test]
    fn steady_rate_over_two_seconds() {
        let prev = counters(1_000, 500, 10, 5);
        let cur = counters(3_000, 1_500, 30, 25);
        let rate = derive_rate(&prev, &cur, Duration::from_secs(2));
        // 2000 bytes over 2s = 1000 B/s rx; 1000 bytes tx = 500 B/s.
        assert_eq!(rate.rx_bytes_per_sec, 1_000.0);
        assert_eq!(rate.tx_bytes_per_sec, 500.0);
        assert_eq!(rate.rx_packets_per_sec, 10.0);
        assert_eq!(rate.tx_packets_per_sec, 10.0);
    }

    #[test]
    fn counter_reset_reads_as_zero_not_negative() {
        // Interface went down and back up: counters restarted lower than before.
        let prev = counters(1_000_000, 2_000_000, 100, 200);
        let cur = counters(500, 800, 2, 3);
        let rate = derive_rate(&prev, &cur, Duration::from_secs(1));
        assert_eq!(rate.rx_bytes_per_sec, 0.0);
        assert_eq!(rate.tx_bytes_per_sec, 0.0);
        assert_eq!(rate.rx_packets_per_sec, 0.0);
        assert_eq!(rate.tx_packets_per_sec, 0.0);
    }

    #[test]
    fn zero_interval_yields_zero_rate() {
        let prev = counters(10, 10, 1, 1);
        let cur = counters(999, 999, 9, 9);
        assert_eq!(derive_rate(&prev, &cur, Duration::ZERO), Rate::default());
    }

    #[test]
    fn sub_second_interval_scales_up() {
        let prev = counters(0, 0, 0, 0);
        let cur = counters(500, 0, 0, 0);
        // 500 bytes in 0.5s = 1000 B/s.
        let rate = derive_rate(&prev, &cur, Duration::from_millis(500));
        assert_eq!(rate.rx_bytes_per_sec, 1_000.0);
    }
}
