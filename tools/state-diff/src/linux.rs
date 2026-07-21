//! Linux state collector: nftables ruleset, routes, DNS, sysctls, systemd units.
//!
//! Linux is the reference platform, so this is the most detailed collector — it
//! is what proves the uninstall contract for the firewall, DHCP, DNS, and
//! sharing features that only Linux implements in full.

use crate::collect::{
    Config, Facet, Snapshot, StateCollector, apply_ignores, command_facet, files_facet, run_lines,
};

pub(crate) struct LinuxCollector;

impl StateCollector for LinuxCollector {
    fn os_name(&self) -> &'static str {
        "linux"
    }

    fn collect(&self, config: &Config) -> Snapshot {
        let mut snap = Snapshot::new("linux");

        // Firewall: the whole nftables ruleset. The uninstall contract requires
        // that every `networkhub_*` table is gone afterward; capturing the full
        // ruleset lets the diff prove it. Needs root.
        snap.insert(
            "firewall_nftables",
            command_facet("nft list ruleset", "nft", &["list", "ruleset"]),
        );

        // Routing tables, IPv4 and IPv6.
        snap.insert(
            "routes_v4",
            command_facet(
                "ip -4 route show",
                "ip",
                &["-4", "route", "show", "table", "all"],
            ),
        );
        snap.insert(
            "routes_v6",
            command_facet(
                "ip -6 route show",
                "ip",
                &["-6", "route", "show", "table", "all"],
            ),
        );

        // Addresses (a hotspot leaves an interface address behind if teardown fails).
        snap.insert(
            "addresses",
            command_facet("ip addr show", "ip", &["-o", "addr", "show"]),
        );

        // DNS: resolv.conf and, where present, resolved status.
        snap.insert(
            "dns_resolv",
            command_facet("cat /etc/resolv.conf", "cat", &["/etc/resolv.conf"]),
        );

        // The sysctls NetworkHub changes (ip_forward is the F-9 hazard).
        let mut sysctls = Vec::new();
        for key in ["net.ipv4.ip_forward", "net.ipv6.conf.all.forwarding"] {
            match run_lines("sysctl", &["-n", key]) {
                Ok(l) => sysctls.push(format!("{key} = {}", l.join(""))),
                Err(e) => sysctls.push(format!("{key}: {e}")),
            }
        }
        snap.insert("sysctls", Facet::from_lines("sysctl", sysctls));

        // systemd units matching our namespace. Listing all unit files and
        // filtering keeps the facet small and stable.
        let units = match run_lines(
            "systemctl",
            &["list-unit-files", "--no-legend", "--no-pager"],
        ) {
            Ok(lines) => lines
                .into_iter()
                .filter(|l| l.contains("networkhub"))
                .collect(),
            Err(e) => vec![format!("systemctl: {e}")],
        };
        snap.insert(
            "services_systemd",
            Facet::from_lines("systemctl list-unit-files | grep networkhub", units),
        );

        // Files under watched directories.
        snap.insert("files", files_facet(config));

        apply_ignores(&mut snap, config);
        snap
    }
}
