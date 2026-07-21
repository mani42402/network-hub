//! macOS state collector: PF anchors, routes, DNS, sysctls, launchd services.

use crate::collect::{
    Config, Facet, Snapshot, StateCollector, apply_ignores, command_facet, files_facet, run_lines,
};

pub(crate) struct MacosCollector;

impl StateCollector for MacosCollector {
    fn os_name(&self) -> &'static str {
        "macos"
    }

    fn collect(&self, config: &Config) -> Snapshot {
        let mut snap = Snapshot::new("macos");

        // Firewall: PF anchors. Listing anchors needs root; captured as an error
        // facet otherwise so a diff never mistakes "could not read" for "empty".
        snap.insert(
            "firewall_pf_anchors",
            command_facet("pfctl -sA", "pfctl", &["-sA"]),
        );

        // Routing table.
        snap.insert("routes", command_facet("netstat -rn", "netstat", &["-rn"]));

        // DNS resolver configuration.
        snap.insert("dns", command_facet("scutil --dns", "scutil", &["--dns"]));

        // The sysctls NetworkHub touches (forwarding above all — F-9).
        let mut sysctls = Vec::new();
        for key in ["net.inet.ip.forwarding", "net.inet6.ip6.forwarding"] {
            match run_lines("sysctl", &[key]) {
                Ok(mut l) => sysctls.append(&mut l),
                Err(e) => sysctls.push(format!("{key}: {e}")),
            }
        }
        snap.insert("sysctls", Facet::from_lines("sysctl", sysctls));

        // launchd system daemons.
        snap.insert(
            "services_launchd",
            command_facet("launchctl list", "launchctl", &["list"]),
        );

        // Files under watched directories.
        snap.insert("files", files_facet(config));

        apply_ignores(&mut snap, config);
        snap
    }
}
