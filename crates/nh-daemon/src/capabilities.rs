//! The capability matrix (AD-6) — provisional for M0.
//!
//! The real per-OS probing lands in WP-M0.8. For the walking skeleton this
//! returns a minimal, honest matrix: it reports the OS and marks the
//! feature capabilities `unknown` (not yet probed) rather than pretending they
//! are supported. That keeps the UI's capability-driven rendering correct even
//! before probing exists — an `unknown` capability shows "checking…", never a
//! working control (§3.4).

use nh_types::capability::{Capability, CapabilityMatrix};

/// Builds the current capability matrix. Provisional: OS is real, feature
/// capabilities are `unknown` until WP-M0.8 probes them.
#[must_use]
pub fn probe() -> CapabilityMatrix {
    let mut matrix = CapabilityMatrix::new();

    let os = std::env::consts::OS;
    matrix.insert(
        "system.os".to_owned(),
        Capability::supported(format!("running on {os}")).with_detail("os", os),
    );

    // The feature capabilities from ARCHITECTURE.md §3.4, reported honestly as
    // not-yet-probed. WP-M0.8 replaces these with real probe results.
    for key in [
        "hotspot.wifi_ap",
        "sharing.ethernet",
        "firewall.rules",
        "vpn.wireguard",
        "vpn.openvpn",
        "dns.local_resolver",
        "monitor.per_process",
    ] {
        matrix.insert(
            key.to_owned(),
            Capability::unknown("not yet probed (WP-M0.8)"),
        );
    }

    matrix
}

#[cfg(test)]
mod tests {
    use super::*;
    use nh_types::capability::CapabilityStatus;

    #[test]
    fn reports_os_and_unknown_features() {
        let m = probe();
        assert_eq!(m["system.os"].status, CapabilityStatus::Supported);
        // Features are honestly unknown, not falsely supported.
        assert_eq!(m["vpn.wireguard"].status, CapabilityStatus::Unknown);
    }
}
