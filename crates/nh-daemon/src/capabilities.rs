//! The capability matrix (AD-6): probe, publish, and re-probe.
//!
//! [`probe`] gathers system facts through the target OS's `pal-*` backend and
//! renders the §3.4 matrix via [`nh_pal::probe::build_matrix`]. [`Capabilities`]
//! wraps that in a live holder the daemon serves from: it can be re-probed and
//! publishes a [`CapabilityEvent`] when the matrix changes. The hardware-change
//! trigger that calls [`Capabilities::reprobe`] arrives with the interface
//! watchers (WP-M0.9); the mechanism and the event are established here.

use std::sync::{Arc, PoisonError, RwLock};

use nh_pal::probe::SystemFacts;
use nh_state::event_bus::{EventBus, Subscription};
use nh_types::capability::CapabilityMatrix;

const EVENT_CAPACITY: usize = 64;

/// Probes the host and builds the current capability matrix (AD-6).
#[must_use]
pub fn probe() -> CapabilityMatrix {
    nh_pal::probe::build_matrix(&detect_facts())
}

/// Gathers system facts via the platform probe, falling back to a minimal fact
/// set if it errors — probing must never crash startup (fail-open, AD-5).
fn detect_facts() -> SystemFacts {
    match platform_probe() {
        Ok(facts) => facts,
        Err(e) => {
            tracing::warn!("capability probe failed; using minimal facts: {e}");
            SystemFacts::minimal()
        }
    }
}

#[cfg(target_os = "linux")]
fn platform_probe() -> Result<SystemFacts, nh_pal::probe::ProbeError> {
    use nh_pal::probe::SystemProbe;
    pal_linux::LinuxProbe.facts()
}

#[cfg(target_os = "macos")]
fn platform_probe() -> Result<SystemFacts, nh_pal::probe::ProbeError> {
    use nh_pal::probe::SystemProbe;
    pal_macos::MacosProbe.facts()
}

#[cfg(windows)]
fn platform_probe() -> Result<SystemFacts, nh_pal::probe::ProbeError> {
    use nh_pal::probe::SystemProbe;
    pal_windows::WindowsProbe.facts()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_probe() -> Result<SystemFacts, nh_pal::probe::ProbeError> {
    // No backend for this OS yet; the minimal matrix is still honest.
    Ok(SystemFacts::minimal())
}

/// An event published when the capability matrix changes (AD-6).
#[derive(Debug, Clone)]
pub enum CapabilityEvent {
    /// The matrix changed. `changed_keys` lists the capability keys that differ.
    Changed {
        /// The keys whose capability was added, removed, or altered.
        changed_keys: Vec<String>,
    },
}

/// The daemon's live capability matrix: probed at startup, re-probable on
/// demand, and publishing a [`CapabilityEvent`] whenever it changes.
pub struct Capabilities {
    matrix: RwLock<Arc<CapabilityMatrix>>,
    events: EventBus<CapabilityEvent>,
}

impl std::fmt::Debug for Capabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capabilities")
            .field("entries", &self.current().len())
            .field("subscribers", &self.events.subscriber_count())
            .finish_non_exhaustive()
    }
}

impl Capabilities {
    /// Probes the host now and builds the initial matrix.
    #[must_use]
    pub fn probe_now() -> Self {
        Self {
            matrix: RwLock::new(Arc::new(probe())),
            events: EventBus::new(EVENT_CAPACITY),
        }
    }

    /// Builds a holder from an already-computed matrix (used by tests).
    #[must_use]
    pub fn from_matrix(matrix: CapabilityMatrix) -> Self {
        Self {
            matrix: RwLock::new(Arc::new(matrix)),
            events: EventBus::new(EVENT_CAPACITY),
        }
    }

    /// The current matrix (a cheap `Arc` clone).
    #[must_use]
    pub fn current(&self) -> Arc<CapabilityMatrix> {
        self.matrix
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Subscribes to change events. Pair with [`current`](Self::current) so a
    /// subscriber starts from a known snapshot (§5.4).
    #[must_use]
    pub fn subscribe(&self) -> Subscription<CapabilityEvent> {
        self.events.subscribe()
    }

    /// Re-probes the host. If the matrix changed, swaps it in and publishes a
    /// [`CapabilityEvent::Changed`]. Returns whether anything changed.
    pub fn reprobe(&self) -> bool {
        self.set(probe())
    }

    /// Swaps in `fresh`, publishing a change event iff it differs. Returns
    /// whether anything changed. Exposed for tests that inject a matrix.
    pub fn set(&self, fresh: CapabilityMatrix) -> bool {
        let current = self.current();
        if *current == fresh {
            return false;
        }
        let changed_keys = diff_keys(&current, &fresh);
        {
            let mut guard = self.matrix.write().unwrap_or_else(PoisonError::into_inner);
            *guard = Arc::new(fresh);
        }
        tracing::info!(?changed_keys, "capability matrix changed");
        self.events
            .publish(CapabilityEvent::Changed { changed_keys });
        true
    }
}

/// The keys whose capability differs between two matrices (added, removed, or
/// changed), sorted and de-duplicated.
fn diff_keys(before: &CapabilityMatrix, after: &CapabilityMatrix) -> Vec<String> {
    let mut keys = std::collections::BTreeSet::new();
    for (k, v) in before {
        if after.get(k) != Some(v) {
            keys.insert(k.clone());
        }
    }
    for (k, v) in after {
        if before.get(k) != Some(v) {
            keys.insert(k.clone());
        }
    }
    keys.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nh_state::event_bus::Received;
    use nh_types::capability::{Capability, CapabilityStatus};

    #[test]
    fn probe_returns_the_full_matrix() {
        let m = probe();
        // §3.4 feature keys and the environment keys are all present.
        assert!(m.contains_key("vpn.wireguard"));
        assert!(m.contains_key("hotspot.wifi_ap"));
        assert!(m.contains_key("system.os"));
        assert_eq!(m["system.os"].status, CapabilityStatus::Supported);
    }

    #[test]
    fn set_with_identical_matrix_reports_no_change() {
        let caps = Capabilities::probe_now();
        let same = caps.current().as_ref().clone();
        assert!(!caps.set(same), "an identical matrix is not a change");
    }

    #[test]
    fn set_with_changed_matrix_publishes_event() {
        let caps = Capabilities::from_matrix(probe());
        let mut sub = caps.subscribe();

        let mut changed = caps.current().as_ref().clone();
        changed.insert(
            "vpn.wireguard".to_owned(),
            Capability::supported("now implemented"),
        );
        assert!(caps.set(changed), "a differing matrix is a change");

        match sub.try_recv() {
            Some(Received::Event(seq)) => {
                let CapabilityEvent::Changed { changed_keys } = seq.event;
                assert!(changed_keys.contains(&"vpn.wireguard".to_owned()));
            }
            other => panic!("expected a change event, got {other:?}"),
        }
    }

    #[test]
    fn diff_keys_detects_add_remove_change() {
        let mut a = CapabilityMatrix::new();
        a.insert("x".to_owned(), Capability::supported("a"));
        a.insert("y".to_owned(), Capability::supported("same"));
        let mut b = CapabilityMatrix::new();
        b.insert("y".to_owned(), Capability::supported("same"));
        b.insert("z".to_owned(), Capability::supported("added"));
        // x removed, z added; y unchanged.
        assert_eq!(diff_keys(&a, &b), vec!["x".to_owned(), "z".to_owned()]);
    }
}
