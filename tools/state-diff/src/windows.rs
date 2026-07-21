//! Windows state collector: WFP filters, routes, DNS, services.
//!
//! The uninstall contract on Windows requires that our WFP provider, sublayer,
//! and filters, plus the service, are all gone afterward.

use crate::collect::{Config, Snapshot, StateCollector, apply_ignores, command_facet, files_facet};

pub(crate) struct WindowsCollector;

impl StateCollector for WindowsCollector {
    fn os_name(&self) -> &'static str {
        "windows"
    }

    fn collect(&self, config: &Config) -> Snapshot {
        let mut snap = Snapshot::new("windows");

        // WFP state: filters and sublayers. `netsh wfp show state` needs an
        // elevated shell; captured as an error facet otherwise.
        snap.insert(
            "firewall_wfp",
            command_facet(
                "netsh wfp show filters",
                "netsh",
                &["wfp", "show", "filters", "dir=out"],
            ),
        );

        // Routing table.
        snap.insert("routes", command_facet("route print", "route", &["print"]));

        // DNS client configuration per interface.
        snap.insert(
            "dns",
            command_facet(
                "netsh interface ip show dns",
                "netsh",
                &["interface", "ip", "show", "dnsservers"],
            ),
        );

        // Services matching our namespace, via PowerShell for stable output.
        snap.insert(
            "services",
            command_facet(
                "Get-Service NetworkHub*",
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "Get-Service -Name 'NetworkHub*' | Format-Table -HideTableHeaders Name,Status",
                ],
            ),
        );

        // Files under watched directories.
        snap.insert("files", files_facet(config));

        apply_ignores(&mut snap, config);
        snap
    }
}
