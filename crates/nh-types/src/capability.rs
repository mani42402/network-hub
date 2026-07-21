//! The capability model (AD-6).
//!
//! Whether a machine can host a Wi-Fi access point, run AP and client mode at
//! once, shape traffic per device, or serve a captive portal is a function of
//! the operating system, the driver, and the Wi-Fi chipset — not of NetworkHub.
//! Rather than discover this by trying an operation and handling the failure at
//! the worst moment, the daemon **probes** what is actually possible at startup
//! and publishes a matrix of [`Capability`] values. The UI renders from that
//! matrix and never hides a feature silently: an unsupported capability shows
//! *why*.
//!
//! This module is the vocabulary for that matrix. The probing itself lives in
//! the platform adapters; here we define only the shape of the answer, so the
//! daemon, the clients, and the docs all speak the same words.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Whether a capability is usable, and if not, why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Fully usable on this machine.
    Supported,
    /// Usable, but with limitations the `reason`/`details` explain (e.g. Windows
    /// hotspot without a custom subnet, or Ethernet sharing that resets on
    /// reboot).
    Degraded,
    /// Not usable here. The `reason` says why, so the UI can tell the user
    /// whether it is fixable (a permission, a different adapter) or inherent
    /// (no public API on this OS).
    Unsupported,
    /// Not yet probed, or probing failed. Distinct from `Unsupported`: we do not
    /// know, rather than know it cannot. The UI treats it as "checking…", not as
    /// a definite no.
    Unknown,
}

impl CapabilityStatus {
    /// Whether the feature can be used at all (supported or degraded).
    #[must_use]
    pub const fn is_usable(self) -> bool {
        matches!(self, Self::Supported | Self::Degraded)
    }
}

/// One entry in the capability matrix: a status plus a human explanation and
/// optional structured detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// The verdict.
    pub status: CapabilityStatus,
    /// A short, human-readable explanation, shown in the UI. Required even for
    /// `Supported` (it can note the mechanism), and *especially* meaningful for
    /// `Degraded`/`Unsupported`, where it tells the user why.
    pub reason: String,
    /// Optional machine-readable specifics for the UI to render — supported
    /// bands, a client-count cap, the name of a conflicting service. Ordered so
    /// serialization is stable and two equal matrices compare equal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl Capability {
    /// A supported capability with an explanation of how it is provided.
    pub fn supported(reason: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Supported,
            reason: reason.into(),
            details: BTreeMap::new(),
        }
    }

    /// A degraded capability, with the limitation explained.
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Degraded,
            reason: reason.into(),
            details: BTreeMap::new(),
        }
    }

    /// An unsupported capability, with the reason it cannot work here.
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Unsupported,
            reason: reason.into(),
            details: BTreeMap::new(),
        }
    }

    /// An unknown/not-yet-probed capability.
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Unknown,
            reason: reason.into(),
            details: BTreeMap::new(),
        }
    }

    /// Attaches a structured detail (builder style).
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Convenience for [`CapabilityStatus::is_usable`].
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.status.is_usable()
    }
}

/// The full set of capabilities a machine reports, keyed by a stable dotted name
/// such as `hotspot.wifi_ap` or `vpn.kill_switch` (the keys in ARCHITECTURE.md
/// §3.4). A `BTreeMap` keeps the matrix ordered and diffable.
pub type CapabilityMatrix = BTreeMap<String, Capability>;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn usable_covers_supported_and_degraded_only() {
        assert!(CapabilityStatus::Supported.is_usable());
        assert!(CapabilityStatus::Degraded.is_usable());
        assert!(!CapabilityStatus::Unsupported.is_usable());
        assert!(!CapabilityStatus::Unknown.is_usable());
    }

    #[test]
    fn builder_attaches_details() {
        let cap = Capability::degraded("Windows Mobile Hotspot; no custom subnet")
            .with_detail("max_clients", "8")
            .with_detail("band", "2.4GHz+5GHz");
        assert_eq!(cap.status, CapabilityStatus::Degraded);
        assert_eq!(cap.details.get("max_clients").unwrap(), "8");
        assert!(cap.is_usable());
    }

    #[test]
    fn serializes_with_stable_shape() {
        // A representative slice of the §3.4 matrix.
        let mut matrix = CapabilityMatrix::new();
        matrix.insert(
            "hotspot.wifi_ap".into(),
            Capability::unsupported("macOS exposes no public API for programmatic hotspot"),
        );
        matrix.insert(
            "vpn.wireguard".into(),
            Capability::supported("userspace WireGuard via utun"),
        );

        let json = serde_json::to_string_pretty(&matrix).unwrap();
        let back: CapabilityMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(matrix, back);
        // Empty details are omitted from the wire form.
        assert!(!json.contains("details"));
        // Status serializes in snake_case for the clients.
        assert!(json.contains("\"unsupported\""));
    }

    #[test]
    fn unknown_is_distinct_from_unsupported() {
        // The UI must treat "not yet probed" differently from "cannot work".
        assert_ne!(
            Capability::unknown("probe not run").status,
            Capability::unsupported("no driver support").status,
        );
    }
}
