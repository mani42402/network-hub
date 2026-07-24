//! The Linux system probe (WP-M0.8).
//!
//! Gathers [`SystemFacts`] with no unsafe code: `nix` wraps `getifaddrs`,
//! `uname`, and `geteuid`, and interface typing reads `/sys/class/net`. The
//! richer, watched interface model (identity, addresses, counters) is WP-M0.9.

use std::path::Path;

use nh_pal::probe::{InterfaceKind, Os, ProbeError, ProbedInterface, SystemFacts, SystemProbe};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;

/// The Linux implementation of [`SystemProbe`].
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxProbe;

impl SystemProbe for LinuxProbe {
    fn facts(&self) -> Result<SystemFacts, ProbeError> {
        Ok(SystemFacts {
            os: Os::Linux,
            os_version: os_version(),
            arch: std::env::consts::ARCH.to_owned(),
            privileged: nix::unistd::Uid::effective().is_root(),
            interfaces: interfaces()?,
        })
    }
}

/// Distro pretty-name (from `/etc/os-release`) plus the kernel release.
fn os_version() -> String {
    let pretty = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_owned())
            })
        });
    let kernel = nix::sys::utsname::uname()
        .ok()
        .map(|u| u.release().to_string_lossy().into_owned());
    match (pretty, kernel) {
        (Some(p), Some(k)) => format!("{p} (kernel {k})"),
        (Some(p), None) => p,
        (None, Some(k)) => format!("Linux (kernel {k})"),
        (None, None) => "Linux".to_owned(),
    }
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

/// Classifies an interface from `/sys/class/net/<name>`, falling back to the
/// kernel flags if sysfs is unreadable.
fn classify(name: &str, flags: InterfaceFlags) -> InterfaceKind {
    if flags.contains(InterfaceFlags::IFF_LOOPBACK) {
        return InterfaceKind::Loopback;
    }
    let base = Path::new("/sys/class/net").join(name);
    // ARPHRD_LOOPBACK is 772; a loopback that somehow lacks the flag is caught here.
    if let Ok(t) = std::fs::read_to_string(base.join("type")) {
        if t.trim() == "772" {
            return InterfaceKind::Loopback;
        }
    }
    // A `wireless` directory or `phy80211` link marks an 802.11 device.
    if base.join("wireless").is_dir() || base.join("phy80211").exists() {
        return InterfaceKind::Wireless;
    }
    // A `device` link means a backing hardware device (Ethernet, USB NIC);
    // its absence means a purely virtual interface (bridge, veth, tun, docker0).
    if base.join("device").exists() {
        InterfaceKind::Ethernet
    } else {
        InterfaceKind::Virtual
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_linux_and_at_least_loopback() {
        let facts = LinuxProbe.facts().unwrap();
        assert_eq!(facts.os, Os::Linux);
        assert!(!facts.arch.is_empty());
        assert!(!facts.os_version.is_empty());
        assert!(
            facts.interfaces.iter().any(|i| i.name == "lo"),
            "loopback should be present"
        );
        // The loopback must be typed as loopback.
        let lo = facts.interfaces.iter().find(|i| i.name == "lo").unwrap();
        assert_eq!(lo.kind, InterfaceKind::Loopback);
    }
}
