//! The live-throughput sampler (WP-M0.10).
//!
//! [`Monitor`] samples every interface's counters on a timer, derives a [`Rate`]
//! against the previous reading, keeps a bounded ring of recent rates per
//! interface, and broadcasts each update on a stream the UI subscribes to.
//! Sampling is driven off a monotonic clock, so the derived rates are immune to
//! wall-clock steps (E-26); see [`crate::rate`]. No persistence yet — retention
//! and roll-ups arrive in M2.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use nh_pal::{InterfaceCounters, InterfaceId, InterfaceInfo, InterfaceStats};
use tokio::sync::broadcast;

use crate::rate::{Rate, derive_rate};

/// Default sampling cadence.
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);
/// Default number of recent rates kept per interface (one minute at 1 Hz).
pub const DEFAULT_RING: usize = 60;
const STREAM_CAPACITY: usize = 1024;

/// A per-interface rate update, broadcast once per sample.
#[derive(Debug, Clone)]
pub struct MonitorUpdate {
    /// The interface's stable id.
    pub id: InterfaceId,
    /// The interface's current OS name (for display).
    pub name: String,
    /// The rate derived over the last interval.
    pub rate: Rate,
}

/// Recent history for one interface.
struct History {
    /// The last counter reading and when (monotonic) it was taken.
    last: (Instant, InterfaceCounters),
    /// The most recent derived rates, oldest first, capped at the ring size.
    rates: VecDeque<Rate>,
}

/// Samples interface counters and publishes derived throughput rates.
pub struct Monitor {
    info: Arc<dyn InterfaceInfo>,
    stats: Arc<dyn InterfaceStats>,
    histories: Mutex<BTreeMap<InterfaceId, History>>,
    updates: broadcast::Sender<MonitorUpdate>,
    interval: Duration,
    ring: usize,
}

impl std::fmt::Debug for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Monitor")
            .field("interval", &self.interval)
            .field("ring", &self.ring)
            .finish_non_exhaustive()
    }
}

impl Monitor {
    /// A monitor over the given interface source, with default cadence and ring.
    #[must_use]
    pub fn new(info: Arc<dyn InterfaceInfo>, stats: Arc<dyn InterfaceStats>) -> Arc<Self> {
        Self::with_config(info, stats, DEFAULT_INTERVAL, DEFAULT_RING)
    }

    /// A monitor with an explicit sampling `interval` and ring `capacity`.
    #[must_use]
    pub fn with_config(
        info: Arc<dyn InterfaceInfo>,
        stats: Arc<dyn InterfaceStats>,
        interval: Duration,
        capacity: usize,
    ) -> Arc<Self> {
        let (updates, _) = broadcast::channel(STREAM_CAPACITY);
        Arc::new(Self {
            info,
            stats,
            histories: Mutex::new(BTreeMap::new()),
            updates,
            interval,
            ring: capacity.max(1),
        })
    }

    /// Starts the periodic sampling loop on the current runtime.
    ///
    /// # Panics
    /// Must be called from within a Tokio runtime.
    pub fn start(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(this.interval);
            loop {
                ticker.tick().await;
                this.sample_once().await;
            }
        });
    }

    /// Subscribes to the live rate stream. A lagging subscriber drops updates
    /// and can resync from [`all_latest`](Self::all_latest) (§5.4).
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<MonitorUpdate> {
        self.updates.subscribe()
    }

    /// The most recent rate for one interface, if it has been sampled twice.
    #[must_use]
    pub fn latest(&self, id: &InterfaceId) -> Option<Rate> {
        self.lock().get(id).and_then(|h| h.rates.back().copied())
    }

    /// The recent rate history for one interface, oldest first.
    #[must_use]
    pub fn history(&self, id: &InterfaceId) -> Vec<Rate> {
        self.lock()
            .get(id)
            .map(|h| h.rates.iter().copied().collect())
            .unwrap_or_default()
    }

    /// The latest rate for every interface currently tracked.
    #[must_use]
    pub fn all_latest(&self) -> Vec<(InterfaceId, Rate)> {
        self.lock()
            .iter()
            .filter_map(|(id, h)| h.rates.back().map(|r| (id.clone(), *r)))
            .collect()
    }

    /// Takes one sample now (monotonic).
    pub async fn sample_once(&self) {
        self.sample_once_at(Instant::now()).await;
    }

    /// Takes one sample, using `now` as the monotonic timestamp. Exposed for
    /// deterministic tests; production goes through [`sample_once`](Self::sample_once).
    // The list match binds the error for logging, so a let-else would lose it.
    #[allow(clippy::single_match_else)]
    pub async fn sample_once_at(&self, now: Instant) {
        let interfaces = match self.info.list().await {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("monitor: interface list failed: {e}");
                return;
            }
        };

        let mut seen = BTreeSet::new();
        for iface in interfaces {
            seen.insert(iface.id.clone());
            let counters = match self.stats.counters(&iface.id).await {
                Ok(Some(c)) => c,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("monitor: counters for {} failed: {e}", iface.id);
                    continue;
                }
            };
            if let Some(rate) = self.record(&iface.id, now, counters) {
                let _ = self.updates.send(MonitorUpdate {
                    id: iface.id,
                    name: iface.name,
                    rate,
                });
            }
        }

        // Drop history for interfaces that have gone away.
        self.lock().retain(|id, _| seen.contains(id));
    }

    /// Records a counter reading, returning the derived rate once a baseline
    /// exists. The first reading only establishes the baseline.
    fn record(&self, id: &InterfaceId, now: Instant, counters: InterfaceCounters) -> Option<Rate> {
        let mut histories = self.lock();
        if let Some(history) = histories.get_mut(id) {
            let dt = now.duration_since(history.last.0);
            let rate = derive_rate(&history.last.1, &counters, dt);
            history.last = (now, counters);
            if history.rates.len() >= self.ring {
                history.rates.pop_front();
            }
            history.rates.push_back(rate);
            Some(rate)
        } else {
            // First reading: establish the baseline, no rate yet.
            histories.insert(
                id.clone(),
                History {
                    last: (now, counters),
                    rates: VecDeque::new(),
                },
            );
            None
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<InterfaceId, History>> {
        self.histories
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use nh_pal::mock::MockInterfaces;
    use nh_pal::{Interface, InterfaceKind};

    fn iface(id: &str, name: &str) -> Interface {
        Interface {
            id: InterfaceId(id.to_owned()),
            name: name.to_owned(),
            kind: InterfaceKind::Ethernet,
            is_up: true,
            mac: None,
            addresses: Vec::new(),
        }
    }

    fn counters(rx_bytes: u64, tx_bytes: u64) -> InterfaceCounters {
        InterfaceCounters {
            rx_bytes,
            tx_bytes,
            ..InterfaceCounters::default()
        }
    }

    fn sources(mock: &Arc<MockInterfaces>) -> (Arc<dyn InterfaceInfo>, Arc<dyn InterfaceStats>) {
        (mock.clone(), mock.clone())
    }

    #[tokio::test]
    async fn first_sample_is_baseline_second_yields_rate() {
        let mock = Arc::new(MockInterfaces::new());
        let eth = iface("eth:1", "eth0");
        mock.simulate_add(eth.clone());
        mock.simulate_counters(&eth.id, counters(1_000, 500));

        let (info, stats) = sources(&mock);
        let mon = Monitor::new(info, stats);
        let mut stream = mon.subscribe();

        let t0 = Instant::now();
        mon.sample_once_at(t0).await;
        // No rate yet from a single reading.
        assert!(mon.latest(&eth.id).is_none());

        mock.simulate_counters(&eth.id, counters(3_000, 1_500));
        mon.sample_once_at(t0 + Duration::from_secs(2)).await;

        let update = stream.recv().await.unwrap();
        assert_eq!(update.id, eth.id);
        assert_eq!(update.rate.rx_bytes_per_sec, 1_000.0);
        assert_eq!(update.rate.tx_bytes_per_sec, 500.0);
        assert_eq!(mon.latest(&eth.id), Some(update.rate));
    }

    #[tokio::test]
    async fn ring_is_bounded() {
        let mock = Arc::new(MockInterfaces::new());
        let eth = iface("eth:1", "eth0");
        mock.simulate_add(eth.clone());

        let (info, stats) = sources(&mock);
        let mon = Monitor::with_config(info, stats, DEFAULT_INTERVAL, 3);
        let t0 = Instant::now();
        for i in 0..10u64 {
            mock.simulate_counters(&eth.id, counters(i * 1_000, 0));
            mon.sample_once_at(t0 + Duration::from_secs(i)).await;
        }
        // At most `ring` = 3 rates retained.
        assert_eq!(mon.history(&eth.id).len(), 3);
    }

    #[tokio::test]
    async fn gone_interface_is_pruned() {
        let mock = Arc::new(MockInterfaces::new());
        let eth = iface("eth:1", "eth0");
        mock.simulate_add(eth.clone());
        mock.simulate_counters(&eth.id, counters(10, 10));

        let (info, stats) = sources(&mock);
        let mon = Monitor::new(info, stats);
        let t0 = Instant::now();
        mon.sample_once_at(t0).await;
        mon.sample_once_at(t0 + Duration::from_secs(1)).await;
        assert!(mon.latest(&eth.id).is_some());

        mock.simulate_remove(&eth.id);
        mon.sample_once_at(t0 + Duration::from_secs(2)).await;
        assert!(mon.latest(&eth.id).is_none(), "history dropped when gone");
    }
}
