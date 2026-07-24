//! Metric sampling, rate derivation, retention, and rollups.
//!
//! **Status (M0 subset, WP-M0.10):** live per-interface throughput. The monitor
//! samples the [`InterfaceStats`](nh_pal::InterfaceStats) counters delivered by
//! the platform backends on a monotonic timer, derives rates that survive
//! counter resets and wall-clock steps ([`rate`]), keeps a bounded ring of
//! recent rates, and broadcasts each update on a stream ([`monitor`]).
//! Persistence, retention, and roll-ups arrive in M2.

pub mod monitor;
pub mod rate;

pub use monitor::{DEFAULT_INTERVAL, DEFAULT_RING, Monitor, MonitorUpdate};
pub use rate::{Rate, derive_rate};
