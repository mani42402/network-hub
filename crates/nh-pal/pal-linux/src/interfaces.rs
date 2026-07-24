//! Linux [`InterfaceInfo`] + [`InterfaceStats`] with an RTNETLINK change watcher
//! (WP-M0.9).
//!
//! Interface identity, kind, link state, and counters come from `/sys/class/net`
//! (authoritative and cheap); IP addresses from `getifaddrs`; and change
//! notifications from an `AF_NETLINK` route socket subscribed to the link/addr
//! multicast groups. The watcher follows the shared "notification-as-hint,
//! enumeration-as-truth" model ([`nh_pal::interface::watch`]): a netlink message
//! merely wakes a re-enumeration that is diffed against the previous snapshot.
//! Only the address read touches raw FFI (getifaddrs); the netlink socket uses
//! safe `nix` wrappers.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use nh_pal::error::{PalError, PalResult};
use nh_pal::interface::{
    Interface, InterfaceCounters, InterfaceEvent, InterfaceEventStream, InterfaceId, InterfaceInfo,
    InterfaceKind, InterfaceStats, IpCidr, resolve_id, watch,
};
use nh_types::MacAddr;
use tokio::sync::broadcast;

const EVENT_CAPACITY: usize = 256;
const BACKSTOP: Duration = Duration::from_secs(30);
const POLL_BACKSTOP: Duration = Duration::from_secs(2);
const SYS_CLASS_NET: &str = "/sys/class/net";
/// `ARPHRD_LOOPBACK` — the `/sys/.../type` value for the loopback interface.
const ARPHRD_LOOPBACK: u32 = 772;
/// `ARPHRD_NONE` — used by tun devices and WireGuard.
const ARPHRD_NONE: u32 = 65534;

/// The Linux interface backend.
pub struct LinuxInterfaces {
    events: broadcast::Sender<InterfaceEvent>,
}

impl std::fmt::Debug for LinuxInterfaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinuxInterfaces").finish_non_exhaustive()
    }
}

impl LinuxInterfaces {
    /// Builds the backend and starts the watch loop on the current runtime.
    ///
    /// A netlink route socket gives near-instant change wakes; if it cannot be
    /// opened, the loop falls back to a short-interval backstop poll.
    ///
    /// # Errors
    /// Never fails today, but returns [`PalError`] to leave room for a hard
    /// failure in a future revision.
    ///
    /// # Panics
    /// Must be called from within a Tokio runtime (it spawns the watch task).
    pub fn new() -> PalResult<Arc<Self>> {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        let this = Arc::new(Self {
            events: events.clone(),
        });
        let info: Arc<dyn InterfaceInfo> = this.clone();
        match NetlinkWake::open() {
            Ok(wake) => {
                tokio::spawn(watch::run(info, Box::new(wake), BACKSTOP, events));
            }
            Err(e) => {
                tracing::warn!("RTNETLINK watcher unavailable, polling instead: {e}");
                tokio::spawn(watch::run(info, Box::new(NeverWake), POLL_BACKSTOP, events));
            }
        }
        Ok(this)
    }
}

#[async_trait::async_trait]
impl InterfaceInfo for LinuxInterfaces {
    async fn list(&self) -> PalResult<Vec<Interface>> {
        enumerate()
    }

    async fn get(&self, id: &InterfaceId) -> PalResult<Option<Interface>> {
        Ok(enumerate()?.into_iter().find(|i| &i.id == id))
    }

    fn watch(&self) -> InterfaceEventStream {
        self.events.subscribe()
    }
}

#[async_trait::async_trait]
impl InterfaceStats for LinuxInterfaces {
    async fn counters(&self, id: &InterfaceId) -> PalResult<Option<InterfaceCounters>> {
        let Some(iface) = enumerate()?.into_iter().find(|i| &i.id == id) else {
            return Ok(None);
        };
        Ok(Some(read_counters(&iface.name)))
    }
}

// --- enumeration -----------------------------------------------------------

fn enumerate() -> PalResult<Vec<Interface>> {
    let addrs = addresses_by_name()?;
    let mut out = Vec::new();
    let entries = std::fs::read_dir(SYS_CLASS_NET).map_err(PalError::Io)?;
    for entry in entries {
        let entry = entry.map_err(PalError::Io)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let base = Path::new(SYS_CLASS_NET).join(&name);
        let kind = classify(&base, &name);
        let mac = read_mac(&base);
        let hint = device_hint(&base);
        let mut addresses = addrs.get(&name).cloned().unwrap_or_default();
        addresses.sort_by(|a, b| a.addr.cmp(&b.addr).then(a.prefix_len.cmp(&b.prefix_len)));
        out.push(Interface {
            id: resolve_id(kind, mac, hint.as_deref(), &name),
            is_up: link_up(&base, kind),
            mac,
            addresses,
            kind,
            name,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn classify(base: &Path, name: &str) -> InterfaceKind {
    let iftype = read_u32(&base.join("type"));
    if iftype == Some(ARPHRD_LOOPBACK) {
        return InterfaceKind::Loopback;
    }
    if base.join("wireless").is_dir() || base.join("phy80211").exists() {
        return InterfaceKind::Wifi;
    }
    if iftype == Some(ARPHRD_NONE)
        || ["tun", "tap", "wg", "ppp"]
            .iter()
            .any(|p| name.starts_with(p))
    {
        return InterfaceKind::Tunnel;
    }
    // A `device` symlink means a backing hardware device; its absence means a
    // purely virtual interface (bridge, veth, docker0).
    if base.join("device").exists() {
        InterfaceKind::Ethernet
    } else {
        InterfaceKind::Virtual
    }
}

fn read_mac(base: &Path) -> Option<MacAddr> {
    let text = std::fs::read_to_string(base.join("address")).ok()?;
    let mut bytes = [0u8; 6];
    let mut parts = text.trim().split(':');
    for byte in &mut bytes {
        *byte = u8::from_str_radix(parts.next()?, 16).ok()?;
    }
    if parts.next().is_some() || bytes == [0u8; 6] {
        None
    } else {
        Some(MacAddr::new(bytes))
    }
}

/// The backing device's bus id (PCI/USB path), a stable identity hint (F-13).
fn device_hint(base: &Path) -> Option<String> {
    let target = std::fs::read_link(base.join("device")).ok()?;
    target.file_name().map(|n| n.to_string_lossy().into_owned())
}

fn link_up(base: &Path, kind: InterfaceKind) -> bool {
    if kind == InterfaceKind::Loopback {
        return true;
    }
    match std::fs::read_to_string(base.join("operstate")) {
        Ok(s) if s.trim() == "up" => true,
        // "unknown" is common for virtual links that are in fact up; consult the
        // carrier file, which reads 1 when the link has carrier.
        Ok(s) if s.trim() == "unknown" => read_u32(&base.join("carrier")) == Some(1),
        _ => false,
    }
}

fn read_u32(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_u64(path: &Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn read_counters(name: &str) -> InterfaceCounters {
    let stats = Path::new(SYS_CLASS_NET).join(name).join("statistics");
    InterfaceCounters {
        rx_bytes: read_u64(&stats.join("rx_bytes")),
        tx_bytes: read_u64(&stats.join("tx_bytes")),
        rx_packets: read_u64(&stats.join("rx_packets")),
        tx_packets: read_u64(&stats.join("tx_packets")),
        rx_errors: read_u64(&stats.join("rx_errors")),
        tx_errors: read_u64(&stats.join("tx_errors")),
        rx_dropped: read_u64(&stats.join("rx_dropped")),
        tx_dropped: read_u64(&stats.join("tx_dropped")),
    }
}

// --- addresses (getifaddrs) ------------------------------------------------

/// Collects IPv4/IPv6 addresses per interface name via `getifaddrs`.
#[allow(unsafe_code)]
fn addresses_by_name() -> PalResult<BTreeMap<String, Vec<IpCidr>>> {
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs writes a linked list into `ifap`, freed below.
    if unsafe { libc::getifaddrs(&raw mut ifap) } != 0 {
        return Err(PalError::Io(std::io::Error::last_os_error()));
    }
    let mut map: BTreeMap<String, Vec<IpCidr>> = BTreeMap::new();
    let mut cur = ifap;
    while !cur.is_null() {
        // SAFETY: `cur` is a valid list node until null.
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_name.is_null() || ifa.ifa_addr.is_null() {
            continue;
        }
        // SAFETY: ifa_name is a valid C string for the life of the list.
        let name = unsafe { CStr::from_ptr(ifa.ifa_name) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: ifa_addr points to a valid sockaddr; we read the family first.
        if let Some(addr) = unsafe { ip_from_sockaddr(ifa.ifa_addr) } {
            let prefix_len = if ifa.ifa_netmask.is_null() {
                default_prefix(&addr)
            } else {
                unsafe { prefix_from_netmask(ifa.ifa_netmask) }.unwrap_or(0)
            };
            map.entry(name)
                .or_default()
                .push(IpCidr { addr, prefix_len });
        }
    }
    // SAFETY: `ifap` came from getifaddrs and is freed exactly once.
    unsafe { libc::freeifaddrs(ifap) };
    Ok(map)
}

#[allow(unsafe_code)]
unsafe fn ip_from_sockaddr(sa: *const libc::sockaddr) -> Option<IpAddr> {
    // SAFETY: sa_family is the sockaddr discriminant, valid to read.
    match i32::from(unsafe { (*sa).sa_family }) {
        libc::AF_INET => {
            let sin: libc::sockaddr_in = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                sin.sin_addr.s_addr,
            ))))
        }
        libc::AF_INET6 => {
            let sin6: libc::sockaddr_in6 = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.s6_addr)))
        }
        _ => None,
    }
}

#[allow(unsafe_code)]
unsafe fn prefix_from_netmask(sa: *const libc::sockaddr) -> Option<u8> {
    // SAFETY: the ifa_netmask sockaddr is valid for reading its family + payload.
    match i32::from(unsafe { (*sa).sa_family }) {
        libc::AF_INET => {
            let sin: libc::sockaddr_in = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(count_ones_u8(&sin.sin_addr.s_addr.to_ne_bytes()))
        }
        libc::AF_INET6 => {
            let sin6: libc::sockaddr_in6 = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(count_ones_u8(&sin6.sin6_addr.s6_addr))
        }
        _ => None,
    }
}

fn count_ones_u8(bytes: &[u8]) -> u8 {
    let bits: u32 = bytes.iter().map(|b| b.count_ones()).sum();
    u8::try_from(bits).unwrap_or(0)
}

fn default_prefix(addr: &IpAddr) -> u8 {
    match addr {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

// --- RTNETLINK watch source ------------------------------------------------

/// A [`watch::WakeSource`] that fires on any link/address netlink notification.
struct NetlinkWake {
    rx: tokio::sync::mpsc::Receiver<()>,
}

impl NetlinkWake {
    fn open() -> PalResult<Self> {
        use nix::sys::socket::{
            AddressFamily, MsgFlags, NetlinkAddr, SockFlag, SockProtocol, SockType, bind, recv,
            socket,
        };

        // RTMGRP_* multicast groups: link state, IPv4 and IPv6 address changes.
        const RTMGRP_LINK: u32 = 1;
        const RTMGRP_IPV4_IFADDR: u32 = 0x10;
        const RTMGRP_IPV6_IFADDR: u32 = 0x100;

        let fd = socket(
            AddressFamily::Netlink,
            SockType::Raw,
            SockFlag::empty(),
            SockProtocol::NetlinkRoute,
        )
        .map_err(|e| PalError::Platform(format!("netlink socket: {e}")))?;
        bind(
            fd.as_raw_fd(),
            &NetlinkAddr::new(0, RTMGRP_LINK | RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR),
        )
        .map_err(|e| PalError::Platform(format!("netlink bind: {e}")))?;

        // Capacity-1 coalesces a burst of messages into one pending wake.
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        std::thread::spawn(move || {
            let owned = fd; // keep the socket open for the thread's lifetime
            let raw = owned.as_raw_fd();
            let mut buf = [0u8; 8192];
            loop {
                match recv(raw, &mut buf, MsgFlags::empty()) {
                    Ok(0) => break,
                    Ok(_) => {
                        if tx.try_send(()).is_err() && tx.is_closed() {
                            break;
                        }
                    }
                    Err(nix::errno::Errno::EINTR) => {}
                    Err(_) => break,
                }
            }
        });
        Ok(Self { rx })
    }
}

#[async_trait::async_trait]
impl watch::WakeSource for NetlinkWake {
    async fn next(&mut self) -> PalResult<()> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| PalError::Platform("RTNETLINK watcher ended".to_owned()))
    }
}

/// A wake source that never fires; the watch loop then relies solely on its
/// backstop timer (used when netlink is unavailable).
struct NeverWake;

#[async_trait::async_trait]
impl watch::WakeSource for NeverWake {
    async fn next(&mut self) -> PalResult<()> {
        std::future::pending().await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lists_loopback_with_stable_id() {
        let ifaces = LinuxInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        let lo = list
            .iter()
            .find(|i| i.name == "lo")
            .expect("lo should be present");
        assert_eq!(lo.kind, InterfaceKind::Loopback);
        assert!(lo.is_up);
        let again = ifaces.list().await.unwrap();
        assert!(again.iter().any(|i| i.id == lo.id));
    }

    #[tokio::test]
    async fn reports_counters_for_loopback() {
        let ifaces = LinuxInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        let lo = list.iter().find(|i| i.name == "lo").unwrap();
        assert!(ifaces.counters(&lo.id).await.unwrap().is_some());
    }
}
