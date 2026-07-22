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
