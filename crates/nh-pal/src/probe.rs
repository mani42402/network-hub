//! Capability probing framework (AD-6, §3.4).
//!
//! The daemon must know, at startup and whenever the hardware changes, what the
//! machine can *actually* do — host a Wi-Fi AP, share Ethernet, shape traffic,
//! run a VPN — so the UI can render honestly instead of failing at the worst
//! moment (AD-6). That knowledge is split in two:
//!
//! - **Facts** ([`SystemFacts`]): the raw environment — OS and version, whether
//!   we are elevated, and the network interfaces present. Gathering these needs
//!   platform code, so it lives behind the [`SystemProbe`] trait, implemented by
//!   each `pal-*` backend.
//! - **Policy** ([`build_matrix`]): a pure, cross-platform function that turns
//!   facts into the published [`CapabilityMatrix`]. It encodes the §3.4 table —
//!   the single source of truth for what each OS will eventually support — and
//!   renders honest statuses for the current milestone. Because it is pure, the
//!   full matrix for every OS is unit-testable on any host.
//!
//! In M0 no feature backend is implemented yet, so every capability that will
//! *eventually* work is reported `Unsupported` with a reason that says "not yet
//! implemented, planned for M…". This is deliberately distinct from `Unknown`
//! ("not probed"): we *have* probed, and we know it does not work today. As each
//! backend lands, its key flips from this static policy to a real runtime probe.

use nh_types::capability::{Capability, CapabilityMatrix};
use serde::{Deserialize, Serialize};

/// The operating-system family the daemon is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    /// Linux.
    Linux,
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Any other platform (BSD, etc.) — supported only as far as the matrix's
    /// generic fallbacks allow.
    Other,
}

impl Os {
    /// The OS this binary was compiled for.
    #[must_use]
    pub fn current() -> Self {
        match std::env::consts::OS {
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            _ => Self::Other,
        }
    }

    /// A short, stable identifier for logs and capability details.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::MacOs => "macos",
            Self::Other => "other",
        }
    }
}

/// A coarse classification of a network interface.
///
/// M0 only needs enough to describe the machine to the UI and to count the kinds
/// of links present. Full, identity-stable interface modelling — addresses, link
/// state, counters, and change watchers — is WP-M0.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    /// The loopback interface.
    Loopback,
    /// A wired Ethernet interface.
    Ethernet,
    /// A Wi-Fi (wireless) interface.
    Wireless,
    /// A virtual interface (bridge, tunnel, veth, …) with no physical device.
    Virtual,
    /// Anything not classified above.
    Other,
}

impl InterfaceKind {
    /// A short, stable identifier for capability details.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Loopback => "loopback",
            Self::Ethernet => "ethernet",
            Self::Wireless => "wireless",
            Self::Virtual => "virtual",
            Self::Other => "other",
        }
    }
}

/// One network interface as seen by the M0 probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbedInterface {
    /// The OS interface name (e.g. `eth0`, `wlan0`, `en0`).
    pub name: String,
    /// Its coarse classification.
    pub kind: InterfaceKind,
    /// Whether the link is currently up.
    pub up: bool,
}

/// The environment facts the capability matrix is derived from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFacts {
    /// The operating-system family.
    pub os: Os,
    /// A best-effort human-readable OS version (distro + kernel, product
    /// version, or build number, depending on platform).
    pub os_version: String,
    /// The CPU architecture (`std::env::consts::ARCH`).
    pub arch: String,
    /// Whether the process is running with administrative privileges
    /// (root / elevated Administrator).
    pub privileged: bool,
    /// The network interfaces present.
    pub interfaces: Vec<ProbedInterface>,
}

impl SystemFacts {
    /// A minimal fact set for when the platform probe fails: OS and arch from the
    /// compile target, unprivileged, no interfaces. Building the matrix from this
    /// still yields an honest (if sparse) result rather than crashing startup —
    /// the fail-open invariant (AD-5) applies to probing too.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            os: Os::current(),
            os_version: "unknown".to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            privileged: false,
            interfaces: Vec::new(),
        }
    }
}

/// An error gathering system facts.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// An OS call or file read failed while probing.
    #[error("probe io error: {0}")]
    Io(String),
    /// The platform cannot be probed by this backend.
    #[error("probe unsupported on this platform: {0}")]
    Unsupported(String),
}

/// Gathers [`SystemFacts`] for the host. Implemented once per platform by the
/// `pal-*` backends; the daemon selects the right one at compile time.
pub trait SystemProbe: Send + Sync {
    /// Gathers the current system facts.
    ///
    /// # Errors
    /// Returns [`ProbeError`] if the platform calls needed to gather facts fail.
    /// Callers should fall back to [`SystemFacts::minimal`] rather than abort.
    fn facts(&self) -> Result<SystemFacts, ProbeError>;
}

// --- the §3.4 capability policy -------------------------------------------

/// What a capability will be *once its backend ships*, per OS. This mirrors the
/// verdicts in ARCHITECTURE.md §3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eventual {
    /// Will be fully supported.
    Supported,
    /// Will work with documented limitations.
    Degraded,
    /// Will depend on the Wi-Fi driver/chipset — probed at runtime once
    /// implemented.
    DriverProbe,
    /// Inherently unavailable on this OS (no public API); this verdict is final.
    Unsupported,
    /// Not a meaningful capability on this OS (e.g. a hotspot band where hotspot
    /// itself is unavailable).
    NotApplicable,
}

impl Eventual {
    /// How to describe the eventual outcome in a not-yet-implemented reason.
    const fn describe(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Degraded => "supported with limitations",
            Self::DriverProbe => "supported if the driver allows (probed at runtime)",
            // The two below never reach `describe`; they render their own reason.
            Self::Unsupported => "unsupported",
            Self::NotApplicable => "not applicable",
        }
    }

    /// Stable identifier for the `eventual` capability detail.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Degraded => "degraded",
            Self::DriverProbe => "driver_probe",
            Self::Unsupported => "unsupported",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// One row of the §3.4 matrix: a capability key, the milestone that implements
/// it, and the per-OS verdict + note.
struct Spec {
    key: &'static str,
    milestone: &'static str,
    win: (Eventual, &'static str),
    linux: (Eventual, &'static str),
    macos: (Eventual, &'static str),
}

use Eventual::{Degraded, DriverProbe, NotApplicable, Supported, Unsupported};

/// The §3.4 capability table, transcribed. The single source of truth for what
/// each OS will support; the UI never hardcodes a platform switch (AD-6).
const SPECS: &[Spec] = &[
    Spec {
        key: "hotspot.wifi_ap",
        milestone: "M2",
        win: (
            DriverProbe,
            "Windows Mobile Hotspot (WinRT tethering); requires adapter and driver support",
        ),
        linux: (
            DriverProbe,
            "nl80211/hostapd AP mode; depends on the Wi-Fi chipset and driver",
        ),
        macos: (
            Unsupported,
            "macOS exposes no programmatic hotspot API; assisted-manual guidance only",
        ),
    },
    Spec {
        key: "hotspot.band_5ghz",
        milestone: "M2",
        win: (DriverProbe, "5 GHz AP depends on the driver"),
        linux: (
            DriverProbe,
            "5 GHz AP depends on the driver and regulatory domain",
        ),
        macos: (NotApplicable, "hotspot is unsupported on macOS"),
    },
    Spec {
        key: "hotspot.band_6ghz",
        milestone: "M2",
        win: (DriverProbe, "6 GHz AP depends on the driver"),
        linux: (
            DriverProbe,
            "6 GHz AP depends on the driver and regulatory domain",
        ),
        macos: (NotApplicable, "hotspot is unsupported on macOS"),
    },
    Spec {
        key: "hotspot.sta_ap_concurrent",
        milestone: "M2",
        win: (
            DriverProbe,
            "concurrent station+AP is rarely supported by Windows drivers",
        ),
        linux: (
            DriverProbe,
            "concurrent station+AP via nl80211 interface-combination probe",
        ),
        macos: (Unsupported, "concurrent station+AP is unsupported on macOS"),
    },
    Spec {
        key: "hotspot.custom_subnet",
        milestone: "M2",
        win: (
            Unsupported,
            "Windows Mobile Hotspot fixes the subnet; it is not configurable",
        ),
        linux: (Supported, "full control over the access-point subnet"),
        macos: (NotApplicable, "hotspot is unsupported on macOS"),
    },
    Spec {
        key: "hotspot.captive_portal",
        milestone: "M3",
        win: (
            Unsupported,
            "no DHCP/DNS control under Windows Mobile Hotspot",
        ),
        linux: (
            Supported,
            "DHCP and DNS are controlled, so a portal redirect is possible",
        ),
        macos: (Unsupported, "hotspot is unsupported on macOS"),
    },
    Spec {
        key: "hotspot.client_isolation",
        milestone: "M2",
        win: (
            Unsupported,
            "no client-isolation control under Windows Mobile Hotspot",
        ),
        linux: (Supported, "hostapd `ap_isolate`"),
        macos: (NotApplicable, "hotspot is unsupported on macOS"),
    },
    Spec {
        key: "sharing.ethernet",
        milestone: "M1",
        win: (
            Degraded,
            "Internet Connection Sharing (ICS): fixed subnet and resets across reboots",
        ),
        linux: (Supported, "NAT via nftables with IP forwarding"),
        macos: (
            Unsupported,
            "macOS Internet Sharing is assisted-manual only; no programmatic API",
        ),
    },
    Spec {
        key: "firewall.rules",
        milestone: "M1",
        win: (Supported, "Windows Filtering Platform (WFP)"),
        linux: (Supported, "nftables"),
        macos: (Supported, "PF anchors"),
    },
    Spec {
        key: "firewall.per_device",
        milestone: "M1",
        win: (Degraded, "partial: subject to IP/MAC volatility caveats"),
        linux: (Supported, "per-device firewall rules"),
        macos: (Supported, "per-device rules via the hotspot bridge"),
    },
    Spec {
        key: "shaping.per_device",
        milestone: "M2",
        win: (
            Unsupported,
            "no usable per-client shaping API without a signed callout driver (v1)",
        ),
        linux: (Supported, "per-device shaping via tc"),
        macos: (Degraded, "partial via dummynet/pf (under evaluation)"),
    },
    Spec {
        key: "quota.per_device",
        milestone: "M2",
        win: (Degraded, "accounting only; no enforcement"),
        linux: (Supported, "per-device accounting and enforcement"),
        macos: (Degraded, "partial"),
    },
    Spec {
        key: "vpn.wireguard",
        milestone: "M2",
        win: (Supported, "WireGuard via the wintun driver"),
        linux: (Supported, "kernel or userspace WireGuard"),
        macos: (Supported, "userspace WireGuard via utun"),
    },
    Spec {
        key: "vpn.openvpn",
        milestone: "M2",
        win: (Supported, "bundled OpenVPN process"),
        linux: (Supported, "bundled OpenVPN process"),
        macos: (Supported, "bundled OpenVPN process"),
    },
    Spec {
        key: "vpn.kill_switch",
        milestone: "M2",
        win: (Supported, "kill switch via WFP"),
        linux: (Supported, "kill switch via nftables"),
        macos: (Supported, "kill switch via PF"),
    },
    Spec {
        key: "dns.local_resolver",
        milestone: "M1",
        win: (Supported, "per-adapter DNS pointing (DoH/DoT/filtering)"),
        linux: (Supported, "local resolver with DoH/DoT/filtering"),
        macos: (Supported, "resolver configuration with DoH/DoT/filtering"),
    },
    Spec {
        key: "monitor.per_process",
        milestone: "M2",
        win: (Supported, "per-process attribution via IP Helper / ETW"),
        linux: (
            DriverProbe,
            "per-process attribution via eBPF; requires kernel eBPF support",
        ),
        macos: (
            Unsupported,
            "deferred: no supported per-process attribution API on macOS",
        ),
    },
];

/// Builds the published capability matrix from the probed facts (AD-6).
///
/// The result always contains every §3.4 feature key plus the `system.*`
/// environment entries, so the UI can render a complete, stable shape on any OS.
#[must_use]
pub fn build_matrix(facts: &SystemFacts) -> CapabilityMatrix {
    let mut matrix = CapabilityMatrix::new();

    // Environment capabilities — these are really probed in M0.
    matrix.insert("system.os".to_owned(), system_os(facts));
    matrix.insert("system.privileged".to_owned(), system_privileged(facts));
    matrix.insert("system.interfaces".to_owned(), system_interfaces(facts));

    // Feature capabilities — rendered from the §3.4 policy under M0 rules.
    for spec in SPECS {
        matrix.insert(spec.key.to_owned(), render(spec, facts.os));
    }

    matrix
}

fn system_os(facts: &SystemFacts) -> Capability {
    Capability::supported(format!(
        "running on {} {} ({})",
        facts.os.as_str(),
        facts.os_version,
        facts.arch
    ))
    .with_detail("os", facts.os.as_str())
    .with_detail("version", facts.os_version.clone())
    .with_detail("arch", facts.arch.clone())
}

fn system_privileged(facts: &SystemFacts) -> Capability {
    if facts.privileged {
        Capability::supported("running with administrative privileges")
    } else {
        // Expected for the unprivileged daemon-sim; the real daemon runs as
        // root/Administrator. Degraded, not Unsupported: the daemon still serves.
        Capability::degraded(
            "running without elevation; privileged network operations are unavailable \
             (expected for daemon-sim / development)",
        )
        .with_detail("limitation", "not_elevated")
    }
}

fn system_interfaces(facts: &SystemFacts) -> Capability {
    let mut counts: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();
    for iface in &facts.interfaces {
        *counts.entry(iface.kind.as_str()).or_default() += 1;
    }
    let mut cap = Capability::supported(format!(
        "{} network interface(s) detected",
        facts.interfaces.len()
    ))
    .with_detail("total", facts.interfaces.len().to_string());
    for kind in [
        InterfaceKind::Loopback,
        InterfaceKind::Ethernet,
        InterfaceKind::Wireless,
        InterfaceKind::Virtual,
        InterfaceKind::Other,
    ] {
        cap = cap.with_detail(
            kind.as_str(),
            counts.get(kind.as_str()).copied().unwrap_or(0).to_string(),
        );
    }
    cap
}

fn render(spec: &Spec, os: Os) -> Capability {
    let (eventual, note) = match os {
        Os::Windows => spec.win,
        Os::Linux => spec.linux,
        Os::MacOs => spec.macos,
        Os::Other => (Unsupported, "unsupported operating system"),
    };
    match eventual {
        NotApplicable => {
            Capability::unsupported(format!("Not applicable on this platform: {note}"))
                .with_detail("limitation", "not_applicable")
        }
        Unsupported => {
            Capability::unsupported(note.to_owned()).with_detail("limitation", "platform")
        }
        Supported | Degraded | DriverProbe => Capability::unsupported(format!(
            "Not yet implemented — planned for {}. When available: {}. ({note})",
            spec.milestone,
            eventual.describe()
        ))
        .with_detail("limitation", "not_implemented")
        .with_detail("planned", spec.milestone)
        .with_detail("eventual", eventual.as_str()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nh_types::capability::CapabilityStatus;

    fn facts_for(os: Os, privileged: bool) -> SystemFacts {
        SystemFacts {
            os,
            os_version: "test-1.0".to_owned(),
            arch: "x86_64".to_owned(),
            privileged,
            interfaces: vec![
                ProbedInterface {
                    name: "lo".to_owned(),
                    kind: InterfaceKind::Loopback,
                    up: true,
                },
                ProbedInterface {
                    name: "eth0".to_owned(),
                    kind: InterfaceKind::Ethernet,
                    up: true,
                },
            ],
        }
    }

    #[test]
    fn matrix_contains_every_spec_key_and_system_entries_on_all_os() {
        for os in [Os::Linux, Os::Windows, Os::MacOs, Os::Other] {
            let m = build_matrix(&facts_for(os, true));
            for spec in SPECS {
                assert!(m.contains_key(spec.key), "{os:?} missing {}", spec.key);
            }
            for key in ["system.os", "system.privileged", "system.interfaces"] {
                assert!(m.contains_key(key), "{os:?} missing {key}");
            }
        }
    }

    #[test]
    fn every_entry_has_a_nonempty_reason() {
        for os in [Os::Linux, Os::Windows, Os::MacOs, Os::Other] {
            let m = build_matrix(&facts_for(os, true));
            for (key, cap) in &m {
                assert!(!cap.reason.trim().is_empty(), "{os:?} {key} has no reason");
            }
        }
    }

    #[test]
    fn inherent_os_limits_are_unsupported_with_platform_detail() {
        // macOS hotspot is a final, inherent "no".
        let m = build_matrix(&facts_for(Os::MacOs, true));
        let cap = &m["hotspot.wifi_ap"];
        assert_eq!(cap.status, CapabilityStatus::Unsupported);
        assert_eq!(cap.details.get("limitation").unwrap(), "platform");
        // A band under an unsupported hotspot is "not applicable".
        assert_eq!(
            m["hotspot.band_5ghz"].details.get("limitation").unwrap(),
            "not_applicable"
        );
    }

    #[test]
    fn eventually_supported_features_are_unsupported_pending_impl_in_m0() {
        // Linux custom subnet will be supported, but no backend exists yet.
        let m = build_matrix(&facts_for(Os::Linux, true));
        let cap = &m["hotspot.custom_subnet"];
        assert_eq!(cap.status, CapabilityStatus::Unsupported);
        assert_eq!(cap.details.get("limitation").unwrap(), "not_implemented");
        assert_eq!(cap.details.get("eventual").unwrap(), "supported");
        assert_eq!(cap.details.get("planned").unwrap(), "M2");
    }

    #[test]
    fn privilege_and_interface_facts_are_reflected() {
        let elevated = build_matrix(&facts_for(Os::Linux, true));
        assert_eq!(
            elevated["system.privileged"].status,
            CapabilityStatus::Supported
        );

        let sim = build_matrix(&facts_for(Os::Linux, false));
        assert_eq!(sim["system.privileged"].status, CapabilityStatus::Degraded);

        let ifaces = &elevated["system.interfaces"];
        assert_eq!(ifaces.details.get("total").unwrap(), "2");
        assert_eq!(ifaces.details.get("ethernet").unwrap(), "1");
        assert_eq!(ifaces.details.get("loopback").unwrap(), "1");
        assert_eq!(ifaces.details.get("wireless").unwrap(), "0");
    }

    #[test]
    fn minimal_facts_still_build_a_full_matrix() {
        let m = build_matrix(&SystemFacts::minimal());
        assert!(m.contains_key("vpn.wireguard"));
        assert!(m.contains_key("system.os"));
    }
}
