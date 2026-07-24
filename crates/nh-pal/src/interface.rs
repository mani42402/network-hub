//! Network interface enumeration and change events.
//!
//! This is the first PAL concern implemented for real (WP-M0.9, per OS via
//! RTNETLINK / `NotifyIpInterfaceChange` / SCDynamicStore) and the one the
//! walking-skeleton UI renders. The mock (WP-M0.3) simulates it fully.

use std::net::IpAddr;

use nh_types::MacAddr;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::PalResult;

/// A stable identifier for an interface across its lifetime.
///
/// Deliberately **not** the OS name. Interface names change — a USB adapter
/// re-plugged gets a new name, predictable-naming schemes rename on hardware
/// changes (F-13) — so rules and profiles bound to a name would silently break.
/// The daemon resolves a stable identity (from MAC + kind + bus path) and keys
/// on this; the OS name is shown to the user but never relied on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InterfaceId(
    /// The stable identity string.
    pub String,
);

impl std::fmt::Display for InterfaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What kind of interface this is, which drives both UI presentation and which
/// operations are valid (you cannot host an access point on a loopback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    /// A wired Ethernet interface.
    Ethernet,
    /// A Wi-Fi interface.
    Wifi,
    /// The loopback interface.
    Loopback,
    /// A VPN tunnel interface (utun / wintun / wg).
    Tunnel,
    /// A virtual or bridge interface, e.g. a hotspot bridge.
    Virtual,
    /// Anything not otherwise classified.
    Other,
}

/// A network interface and its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    /// Stable identity (see [`InterfaceId`]).
    pub id: InterfaceId,
    /// Human-facing OS name, e.g. `en0`, `eth0`, `Wi-Fi`. For display only.
    pub name: String,
    /// What kind of interface this is.
    pub kind: InterfaceKind,
    /// Whether the link is up.
    pub is_up: bool,
    /// Hardware address, if the interface has one (loopback and some tunnels do
    /// not).
    pub mac: Option<MacAddr>,
    /// Assigned IP addresses with prefix lengths.
    pub addresses: Vec<IpCidr>,
}

/// An IP address with its prefix length, e.g. `192.168.1.5/24`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpCidr {
    /// The address.
    pub addr: IpAddr,
    /// The network prefix length in bits (0–32 for v4, 0–128 for v6).
    pub prefix_len: u8,
}

impl std::fmt::Display for IpCidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix_len)
    }
}

/// A change to the set of interfaces or to one interface's state, delivered on
/// the watch channel. The daemon translates these into domain events (§5.4) and
/// re-resolves any state bound to the affected interface (F-13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InterfaceEvent {
    /// A new interface appeared (adapter plugged in, tunnel created).
    Added {
        /// The newly-appeared interface.
        interface: Interface,
    },
    /// An interface disappeared (adapter unplugged, tunnel torn down).
    Removed {
        /// The interface that is gone.
        id: InterfaceId,
    },
    /// The link went up or down without the interface appearing/disappearing.
    LinkChanged {
        /// The affected interface.
        id: InterfaceId,
        /// The new link state.
        is_up: bool,
    },
    /// The set of addresses changed (DHCP lease, manual configuration).
    AddressesChanged {
        /// The affected interface.
        id: InterfaceId,
        /// The complete new address set.
        addresses: Vec<IpCidr>,
    },
    /// The OS name changed while the stable id stayed the same — the F-13 case
    /// that must not break anything bound to the interface.
    Renamed {
        /// The interface, whose stable id is unchanged.
        id: InterfaceId,
        /// The new OS name.
        new_name: String,
    },
}

/// A subscription to interface change events.
pub type InterfaceEventStream = broadcast::Receiver<InterfaceEvent>;

/// A snapshot of one interface's traffic counters.
///
/// Raw and monotonic as the OS reports them. Deriving rates — including the
/// wrap/reset handling that keeps a rate from ever going negative (E-25/E-26) —
/// is the monitor's job (WP-M0.10), not the PAL's; the PAL only reports the
/// current values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InterfaceCounters {
    /// Bytes received.
    pub rx_bytes: u64,
    /// Bytes transmitted.
    pub tx_bytes: u64,
    /// Packets received.
    pub rx_packets: u64,
    /// Packets transmitted.
    pub tx_packets: u64,
    /// Receive errors.
    pub rx_errors: u64,
    /// Transmit errors.
    pub tx_errors: u64,
    /// Inbound packets dropped.
    pub rx_dropped: u64,
    /// Outbound packets dropped.
    pub tx_dropped: u64,
}

/// Reads per-interface traffic counters (WP-M0.9).
///
/// Separate from [`InterfaceInfo`] because counters are volatile telemetry the
/// monitor samples on a timer, whereas interface identity/config changes are
/// event-driven — different lifecycles, different consumers.
#[async_trait::async_trait]
pub trait InterfaceStats: Send + Sync {
    /// Reads the current counters for one interface, or `None` if it is gone.
    ///
    /// # Errors
    /// Returns [`PalError`](crate::error::PalError) if the OS counter query fails.
    async fn counters(&self, id: &InterfaceId) -> PalResult<Option<InterfaceCounters>>;
}

/// Resolves the stable [`InterfaceId`] for an interface (F-13).
///
/// Identity must survive an OS rename or a re-plug, so it is derived from durable
/// attributes rather than the volatile name:
/// 1. a globally-administered hardware MAC — the strongest key;
/// 2. else a platform-supplied stable hint (PCI/USB path, adapter GUID);
/// 3. else a locally-administered/randomized MAC (weak, but stable per boot);
/// 4. else, as a last resort, the OS name.
///
/// The [`InterfaceKind`] is folded in so two interfaces that share a basis (a
/// bridge over an adapter, say) still get distinct ids.
#[must_use]
pub fn resolve_id(
    kind: InterfaceKind,
    mac: Option<MacAddr>,
    stable_hint: Option<&str>,
    name: &str,
) -> InterfaceId {
    let zero = MacAddr::new([0; 6]);
    let basis = match (mac, stable_hint) {
        (Some(m), _) if m != zero && !m.is_locally_administered() => format!("mac:{m}"),
        (_, Some(h)) if !h.is_empty() => format!("hint:{h}"),
        (Some(m), _) if m != zero => format!("mac-la:{m}"),
        _ => format!("name:{name}"),
    };
    InterfaceId(format!("{}:{basis}", kind_tag(kind)))
}

const fn kind_tag(kind: InterfaceKind) -> &'static str {
    match kind {
        InterfaceKind::Ethernet => "eth",
        InterfaceKind::Wifi => "wifi",
        InterfaceKind::Loopback => "lo",
        InterfaceKind::Tunnel => "tun",
        InterfaceKind::Virtual => "virt",
        InterfaceKind::Other => "other",
    }
}

/// Computes the [`InterfaceEvent`]s that carry `old` to `new`.
///
/// Order-independent (keyed on [`InterfaceId`]). A single interface can yield
/// several events at once — e.g. a rename plus a link change. Callers should
/// pass address lists in a canonical (sorted) order so an unordered reshuffle is
/// not mistaken for a change.
#[must_use]
pub fn diff(old: &[Interface], new: &[Interface]) -> Vec<InterfaceEvent> {
    // Interface counts are tiny (tens at most), so a linear scan per side is
    // clearer than building maps and cheaper than it looks.
    let mut events = Vec::new();

    for old_iface in old {
        if !new.iter().any(|n| n.id == old_iface.id) {
            events.push(InterfaceEvent::Removed {
                id: old_iface.id.clone(),
            });
        }
    }
    for new_iface in new {
        let Some(old_iface) = old.iter().find(|o| o.id == new_iface.id) else {
            events.push(InterfaceEvent::Added {
                interface: new_iface.clone(),
            });
            continue;
        };
        if old_iface.name != new_iface.name {
            events.push(InterfaceEvent::Renamed {
                id: new_iface.id.clone(),
                new_name: new_iface.name.clone(),
            });
        }
        if old_iface.is_up != new_iface.is_up {
            events.push(InterfaceEvent::LinkChanged {
                id: new_iface.id.clone(),
                is_up: new_iface.is_up,
            });
        }
        if old_iface.addresses != new_iface.addresses {
            events.push(InterfaceEvent::AddressesChanged {
                id: new_iface.id.clone(),
                addresses: new_iface.addresses.clone(),
            });
        }
    }
    events
}

/// The shared interface-watch engine.
///
/// Every real backend watches the same way: a cheap OS signal ("something
/// changed") wakes a re-enumeration, and the fresh list is diffed against the
/// last snapshot to produce precise [`InterfaceEvent`]s. Parsing the raw OS
/// notification payload is deliberately avoided — the notification is only a
/// hint; the enumeration is the truth — which is both simpler and less fragile.
/// A periodic backstop covers any missed signal. The engine lives here so the
/// tricky loop is written and tested once; each backend supplies only an
/// [`InterfaceInfo`] to enumerate through and a [`WakeSource`].
pub mod watch {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::broadcast;

    use super::{Interface, InterfaceEvent, InterfaceInfo, diff};
    use crate::error::PalResult;

    /// A source of "something may have changed" wake-ups, backed by an OS change
    /// notification (netlink socket, `PF_ROUTE` socket, `NotifyIpInterfaceChange`).
    #[async_trait::async_trait]
    pub trait WakeSource: Send {
        /// Resolves on the next change signal. Returning `Err` ends the watch
        /// loop (the daemon then relies on the periodic `list()` resync, §5.4).
        ///
        /// # Errors
        /// Returns [`PalError`](crate::error::PalError) if the underlying
        /// notification source fails unrecoverably.
        async fn next(&mut self) -> PalResult<()>;
    }

    /// Runs the watch loop until `wake` errors. Backends spawn this on their
    /// runtime and hand out `tx.subscribe()` from `watch()`.
    ///
    /// On each wake — or every `backstop` if no signal arrives — it re-enumerates
    /// via `info.list()`, diffs against the previous snapshot, and broadcasts the
    /// resulting events.
    pub async fn run(
        info: Arc<dyn InterfaceInfo>,
        mut wake: Box<dyn WakeSource>,
        backstop: Duration,
        tx: broadcast::Sender<InterfaceEvent>,
    ) {
        let mut snapshot: Vec<Interface> = info.list().await.unwrap_or_default();
        loop {
            tokio::select! {
                r = wake.next() => {
                    if let Err(e) = r {
                        tracing::warn!("interface wake source ended: {e}");
                        break;
                    }
                }
                () = tokio::time::sleep(backstop) => {}
            }
            match info.list().await {
                Ok(fresh) => {
                    for event in diff(&snapshot, &fresh) {
                        // A send error only means no live subscribers; the next
                        // subscriber resyncs from list(), so it is safe to drop.
                        let _ = tx.send(event);
                    }
                    snapshot = fresh;
                }
                Err(e) => tracing::warn!("interface re-enumeration failed: {e}"),
            }
        }
    }
}

/// Enumerates network interfaces and watches for changes.
///
/// The one PAL trait M0 implements for real. Enumeration is fast (a syscall), so
/// it is `async` only for uniformity with slower backends; watching returns a
/// broadcast receiver that the daemon subscribes to, matching the event-bus
/// model it uses internally.
#[async_trait::async_trait]
pub trait InterfaceInfo: Send + Sync {
    /// Lists the machine's interfaces and their current state.
    async fn list(&self) -> PalResult<Vec<Interface>>;

    /// Fetches one interface by stable id, or `None` if it is gone.
    async fn get(&self, id: &InterfaceId) -> PalResult<Option<Interface>>;

    /// Subscribes to change events. Each subscriber gets its own receiver;
    /// slow subscribers lag and lose events (bounded channel), which is
    /// acceptable because the daemon resynchronizes from a fresh `list()` on
    /// lag — it never lets a slow consumer back-pressure the watcher (§5.4).
    fn watch(&self) -> InterfaceEventStream;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::net::Ipv4Addr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;

    fn cidr(octets: [u8; 4], prefix_len: u8) -> IpCidr {
        IpCidr {
            addr: IpAddr::V4(Ipv4Addr::from(octets)),
            prefix_len,
        }
    }

    fn iface(id: &str, name: &str, is_up: bool, addresses: Vec<IpCidr>) -> Interface {
        Interface {
            id: InterfaceId(id.to_owned()),
            name: name.to_owned(),
            kind: InterfaceKind::Ethernet,
            is_up,
            mac: None,
            addresses,
        }
    }

    #[test]
    fn diff_detects_each_change_kind() {
        let base = iface("a", "eth0", true, vec![cidr([10, 0, 0, 1], 24)]);
        let base_slice = std::slice::from_ref(&base);

        assert!(matches!(
            diff(&[], base_slice).as_slice(),
            [InterfaceEvent::Added { .. }]
        ));
        assert!(matches!(
            diff(base_slice, &[]).as_slice(),
            [InterfaceEvent::Removed { .. }]
        ));

        let mut down = base.clone();
        down.is_up = false;
        assert!(matches!(
            diff(base_slice, std::slice::from_ref(&down)).as_slice(),
            [InterfaceEvent::LinkChanged { is_up: false, .. }]
        ));

        let mut readdr = base.clone();
        readdr.addresses = vec![cidr([10, 0, 0, 2], 24)];
        assert!(matches!(
            diff(base_slice, std::slice::from_ref(&readdr)).as_slice(),
            [InterfaceEvent::AddressesChanged { .. }]
        ));

        let mut renamed = base.clone();
        renamed.name = "eth1".to_owned();
        assert!(matches!(
            diff(base_slice, std::slice::from_ref(&renamed)).as_slice(),
            [InterfaceEvent::Renamed { .. }]
        ));
    }

    #[test]
    fn diff_combines_multiple_changes_for_one_interface() {
        let a = iface("id", "eth0", true, vec![]);
        let mut b = a.clone();
        b.name = "eth1".to_owned();
        b.is_up = false;
        // Same stable id, two attributes changed → two events (rename + link).
        assert_eq!(diff(&[a], &[b]).len(), 2);
    }

    #[test]
    fn resolve_id_prefers_global_mac_then_hint_then_name() {
        let global = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(!global.is_locally_administered());
        assert_eq!(
            resolve_id(
                InterfaceKind::Ethernet,
                Some(global),
                Some("pci-0000:00"),
                "eth0"
            )
            .0,
            format!("eth:mac:{global}")
        );

        // A randomized (locally-administered) MAC is weaker than a stable hint.
        let random = MacAddr::new([0x02, 0, 0, 0, 0, 1]);
        assert!(random.is_locally_administered());
        assert_eq!(
            resolve_id(InterfaceKind::Wifi, Some(random), Some("usb-1"), "wlan0").0,
            "wifi:hint:usb-1"
        );
        // No hint: the randomized MAC still beats the name (stable per boot).
        assert_eq!(
            resolve_id(InterfaceKind::Wifi, Some(random), None, "wlan0").0,
            format!("wifi:mac-la:{random}")
        );
        // Nothing durable: fall back to the name.
        assert_eq!(
            resolve_id(InterfaceKind::Tunnel, None, None, "utun3").0,
            "tun:name:utun3"
        );
    }

    /// An `InterfaceInfo` that returns a scripted sequence of snapshots.
    struct ScriptedInfo {
        snaps: Vec<Vec<Interface>>,
        idx: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl InterfaceInfo for ScriptedInfo {
        async fn list(&self) -> PalResult<Vec<Interface>> {
            let i = self.idx.fetch_add(1, Ordering::SeqCst);
            Ok(self.snaps[i.min(self.snaps.len() - 1)].clone())
        }
        async fn get(&self, _id: &InterfaceId) -> PalResult<Option<Interface>> {
            Ok(None)
        }
        fn watch(&self) -> InterfaceEventStream {
            broadcast::channel(1).0.subscribe()
        }
    }

    /// A wake source driven by a channel; closing it ends the watch loop.
    struct ChannelWake(tokio::sync::mpsc::Receiver<()>);

    #[async_trait::async_trait]
    impl watch::WakeSource for ChannelWake {
        async fn next(&mut self) -> PalResult<()> {
            self.0
                .recv()
                .await
                .ok_or_else(|| crate::error::PalError::Platform("wake closed".to_owned()))
        }
    }

    #[tokio::test]
    async fn watch_engine_emits_diffs_on_each_wake() {
        let a = iface("A", "a", true, vec![]);
        let b = iface("B", "b", true, vec![]);
        let info = Arc::new(ScriptedInfo {
            // initial [A], then add B, then remove A.
            snaps: vec![vec![a.clone()], vec![a.clone(), b.clone()], vec![b.clone()]],
            idx: AtomicUsize::new(0),
        });
        let (wake_tx, wake_rx) = tokio::sync::mpsc::channel(8);
        let (tx, mut rx) = broadcast::channel::<InterfaceEvent>(64);
        // A huge backstop so only explicit wakes drive the loop in the test.
        tokio::spawn(watch::run(
            info,
            Box::new(ChannelWake(wake_rx)),
            Duration::from_hours(1),
            tx,
        ));

        wake_tx.send(()).await.unwrap();
        assert!(
            matches!(rx.recv().await.unwrap(), InterfaceEvent::Added { interface } if interface.id.0 == "B")
        );

        wake_tx.send(()).await.unwrap();
        assert!(matches!(rx.recv().await.unwrap(), InterfaceEvent::Removed { id } if id.0 == "A"));
    }
}
