//! The macOS system probe (WP-M0.8).
//!
//! Gathers [`SystemFacts`] with no unsafe code: `nix` wraps `getifaddrs`,
//! `uname`, and `geteuid`, and the product version is read from the system
//! plist. Interface typing is a name-prefix heuristic here — distinguishing
//! Wi-Fi from Ethernet (both appear as `enN`) needs SystemConfiguration, which
//! arrives with the watched interface model in WP-M0.9.

use nh_pal::probe::{InterfaceKind, Os, ProbeError, ProbedInterface, SystemFacts, SystemProbe};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;

const SYSTEM_VERSION_PLIST: &str = "/System/Library/CoreServices/SystemVersion.plist";

/// Name prefixes for interfaces with no backing physical device.
const VIRTUAL_PREFIXES: &[&str] = &[
    "utun", "ipsec", "ppp", "gif", "stf", "bridge", "ap", "feth", "vlan",
];

/// The macOS implementation of [`SystemProbe`].
#[derive(Debug, Default, Clone, Copy)]
pub struct MacosProbe;

impl SystemProbe for MacosProbe {
    fn facts(&self) -> Result<SystemFacts, ProbeError> {
        Ok(SystemFacts {
            os: Os::MacOs,
            os_version: os_version(),
            arch: std::env::consts::ARCH.to_owned(),
            privileged: nix::unistd::Uid::effective().is_root(),
            interfaces: interfaces()?,
        })
    }
}

/// Product name + version from the system plist, plus the Darwin kernel release.
fn os_version() -> String {
    let product = plist_value("ProductName").unwrap_or_else(|| "macOS".to_owned());
    let version = plist_value("ProductVersion");
    let kernel = nix::sys::utsname::uname()
        .ok()
        .map(|u| u.release().to_string_lossy().into_owned());
    match (version, kernel) {
        (Some(v), Some(k)) => format!("{product} {v} (Darwin {k})"),
        (Some(v), None) => format!("{product} {v}"),
        (None, Some(k)) => format!("{product} (Darwin {k})"),
        (None, None) => product,
    }
}

/// Extracts a `<string>` value following a `<key>` from the system version
/// plist. A tiny scanner avoids a plist-parser dependency for two fields.
fn plist_value(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(SYSTEM_VERSION_PLIST).ok()?;
    let after_key = text.split(&format!("<key>{key}</key>")).nth(1)?;
    let start = after_key.find("<string>")? + "<string>".len();
    let rest = &after_key[start..];
    let end = rest.find("</string>")?;
    Some(rest[..end].trim().to_owned())
}

/// Enumerates interfaces, de-duplicating the per-address-family entries
/// `getifaddrs` returns into one record per interface name.
fn interfaces() -> Result<Vec<ProbedInterface>, ProbeError> {
    let addrs = getifaddrs().map_err(|e| ProbeError::Io(format!("getifaddrs: {e}")))?;
    let mut seen: std::collections::BTreeMap<String, ProbedInterface> =
        std::collections::BTreeMap::new();
    for ifaddr in addrs {
        let name = ifaddr.interface_name;
        seen.entry(name.clone()).or_insert_with(|| ProbedInterface {
            kind: classify(&name, ifaddr.flags),
            up: ifaddr.flags.contains(InterfaceFlags::IFF_UP),
            name,
        });
    }
    Ok(seen.into_values().collect())
}

/// Classifies an interface by name prefix. Coarse by design for M0: `enN` covers
/// both Wi-Fi and Ethernet on macOS, so it is reported as Ethernet until
/// SystemConfiguration typing lands (WP-M0.9).
fn classify(name: &str, flags: InterfaceFlags) -> InterfaceKind {
    if flags.contains(InterfaceFlags::IFF_LOOPBACK) {
        return InterfaceKind::Loopback;
    }
    // Apple Wireless Direct Link / low-latency WLAN are Wi-Fi-based.
    if name.starts_with("awdl") || name.starts_with("llw") {
        return InterfaceKind::Wireless;
    }
    if name.starts_with("en") {
        return InterfaceKind::Ethernet;
    }
    if VIRTUAL_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return InterfaceKind::Virtual;
    }
    InterfaceKind::Other
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_macos_and_loopback() {
        let facts = MacosProbe.facts().unwrap();
        assert_eq!(facts.os, Os::MacOs);
        assert!(!facts.arch.is_empty());
        assert!(facts.os_version.starts_with("macOS") || !facts.os_version.is_empty());
        let lo = facts.interfaces.iter().find(|i| i.name == "lo0");
        assert!(lo.is_some(), "loopback lo0 should be present");
        assert_eq!(lo.unwrap().kind, InterfaceKind::Loopback);
    }
}
