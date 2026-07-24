//! macOS [`InterfaceInfo`] + [`InterfaceStats`] with a `PF_ROUTE` change watcher
//! (WP-M0.9).
//!
//! Enumeration and MAC/address extraction come from `getifaddrs`; 64-bit traffic
//! counters from the `NET_RT_IFLIST2` sysctl (what `netstat -ib` uses); and
//! change notifications from a `PF_ROUTE` socket. The watcher follows the shared
//! "notification-as-hint, enumeration-as-truth" model: a routing message merely
//! wakes a re-enumeration, which is diffed against the previous snapshot
//! ([`nh_pal::interface::watch`]). All raw FFI, which is why this crate is a
//! designated `unsafe` boundary.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use nh_pal::error::{PalError, PalResult};
use nh_pal::interface::{
    Interface, InterfaceCounters, InterfaceEvent, InterfaceEventStream, InterfaceId, InterfaceInfo,
    InterfaceKind, InterfaceStats, IpCidr, resolve_id, watch,
};
use tokio::sync::broadcast;

/// Broadcast backlog; a lagging subscriber resyncs from `list()` (§5.4).
const EVENT_CAPACITY: usize = 256;
/// Backstop re-enumeration when the routing socket is delivering wakes.
const BACKSTOP: Duration = Duration::from_secs(30);
/// Backstop when the routing socket is unavailable, so it doubles as the poll.
const POLL_BACKSTOP: Duration = Duration::from_secs(2);

/// The macOS interface backend.
pub struct MacosInterfaces {
    events: broadcast::Sender<InterfaceEvent>,
}

impl std::fmt::Debug for MacosInterfaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacosInterfaces").finish_non_exhaustive()
    }
}

impl MacosInterfaces {
    /// Builds the backend and starts the watch loop on the current runtime.
    ///
    /// The `PF_ROUTE` socket gives near-instant change wakes; if it cannot be
    /// opened, the loop falls back to a short-interval backstop poll so watching
    /// still works (just less promptly).
    ///
    /// # Errors
    /// Returns [`PalError`] only if the initial enumeration fails outright.
    ///
    /// # Panics
    /// Must be called from within a Tokio runtime (it spawns the watch task).
    pub fn new() -> PalResult<Arc<Self>> {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        let this = Arc::new(Self {
            events: events.clone(),
        });
        let info: Arc<dyn InterfaceInfo> = this.clone();
        match RouteWake::open() {
            Ok(wake) => {
                tokio::spawn(watch::run(info, Box::new(wake), BACKSTOP, events));
            }
            Err(e) => {
                tracing::warn!("PF_ROUTE watcher unavailable, polling instead: {e}");
                tokio::spawn(watch::run(info, Box::new(NeverWake), POLL_BACKSTOP, events));
            }
        }
        Ok(this)
    }
}

#[async_trait::async_trait]
impl InterfaceInfo for MacosInterfaces {
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
impl InterfaceStats for MacosInterfaces {
    async fn counters(&self, id: &InterfaceId) -> PalResult<Option<InterfaceCounters>> {
        let Some(iface) = enumerate()?.into_iter().find(|i| &i.id == id) else {
            return Ok(None);
        };
        let index = if_index(&iface.name);
        if index == 0 {
            return Ok(None);
        }
        Ok(counters_by_index()?.get(&index).copied())
    }
}

// --- enumeration -----------------------------------------------------------

/// One interface accumulated across the `getifaddrs` entries that share a name.
#[derive(Default)]
struct Acc {
    flags: u32,
    mac: Option<nh_types::MacAddr>,
    addresses: Vec<IpCidr>,
}

/// Enumerates interfaces via `getifaddrs`.
#[allow(unsafe_code)]
fn enumerate() -> PalResult<Vec<Interface>> {
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs writes a linked list into `ifap`; we free it below.
    if unsafe { libc::getifaddrs(&raw mut ifap) } != 0 {
        return Err(PalError::Io(std::io::Error::last_os_error()));
    }

    let mut by_name: BTreeMap<String, Acc> = BTreeMap::new();
    let mut cur = ifap;
    while !cur.is_null() {
        // SAFETY: `cur` is a valid node in the getifaddrs list until we hit null.
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_name.is_null() {
            continue;
        }
        // SAFETY: ifa_name is a valid C string for the life of the list.
        let name = unsafe { CStr::from_ptr(ifa.ifa_name) }
            .to_string_lossy()
            .into_owned();
        let acc = by_name.entry(name).or_default();
        acc.flags = ifa.ifa_flags;

        if ifa.ifa_addr.is_null() {
            continue;
        }
        // SAFETY: sa_family is the first-word discriminant of a valid sockaddr.
        let family = i32::from(unsafe { (*ifa.ifa_addr).sa_family });
        match family {
            libc::AF_LINK => {
                if let Some(mac) = unsafe { mac_from_link(ifa.ifa_addr) } {
                    acc.mac = Some(mac);
                }
            }
            libc::AF_INET | libc::AF_INET6 => {
                if let Some(addr) = unsafe { ip_from_sockaddr(ifa.ifa_addr) } {
                    let prefix_len = if ifa.ifa_netmask.is_null() {
                        default_prefix(&addr)
                    } else {
                        unsafe { prefix_from_netmask(ifa.ifa_netmask) }.unwrap_or(0)
                    };
                    acc.addresses.push(IpCidr { addr, prefix_len });
                }
            }
            _ => {}
        }
    }
    // SAFETY: `ifap` came from getifaddrs and has not been freed yet.
    unsafe { libc::freeifaddrs(ifap) };

    Ok(by_name
        .into_iter()
        .map(|(name, mut acc)| {
            acc.addresses
                .sort_by(|a, b| a.addr.cmp(&b.addr).then(a.prefix_len.cmp(&b.prefix_len)));
            let kind = classify(&name, acc.flags);
            Interface {
                id: resolve_id(kind, acc.mac, None, &name),
                is_up: acc.flags & (libc::IFF_UP as u32) != 0,
                mac: acc.mac,
                addresses: acc.addresses,
                kind,
                name,
            }
        })
        .collect())
}

fn classify(name: &str, flags: u32) -> InterfaceKind {
    if flags & (libc::IFF_LOOPBACK as u32) != 0 {
        return InterfaceKind::Loopback;
    }
    // Precise Wi-Fi/Ethernet typing needs SystemConfiguration; `enN` is reported
    // as Ethernet for now (see the M0.8 probe note). Tunnels and virtual links
    // are named distinctly enough to classify by prefix.
    for p in ["utun", "ipsec", "ppp", "gif", "stf", "wg"] {
        if name.starts_with(p) {
            return InterfaceKind::Tunnel;
        }
    }
    if name.starts_with("awdl") || name.starts_with("llw") {
        return InterfaceKind::Wifi;
    }
    if name.starts_with("en") {
        return InterfaceKind::Ethernet;
    }
    for p in ["bridge", "ap", "feth", "vlan"] {
        if name.starts_with(p) {
            return InterfaceKind::Virtual;
        }
    }
    InterfaceKind::Other
}

/// Reads the MAC out of an `AF_LINK` `sockaddr_dl`.
#[allow(unsafe_code)]
unsafe fn mac_from_link(sa: *const libc::sockaddr) -> Option<nh_types::MacAddr> {
    // SAFETY: caller guarantees `sa` points to an AF_LINK sockaddr_dl.
    let dl: libc::sockaddr_dl = unsafe { std::ptr::read_unaligned(sa.cast()) };
    let nlen = dl.sdl_nlen as usize;
    if usize::from(dl.sdl_alen) != 6 {
        return None;
    }
    // The address bytes follow the name bytes inside sdl_data.
    let base = sa.cast::<u8>();
    let mut mac = [0u8; 6];
    for (i, b) in mac.iter_mut().enumerate() {
        // SAFETY: sdl_data starts at offset 8 in sockaddr_dl; name then address
        // bytes live within the allocation getifaddrs made.
        *b = unsafe { *base.add(8 + nlen + i) };
    }
    if mac == [0u8; 6] {
        None
    } else {
        Some(nh_types::MacAddr::new(mac))
    }
}

/// Extracts an [`IpAddr`] from an `AF_INET`/`AF_INET6` sockaddr.
#[allow(unsafe_code)]
unsafe fn ip_from_sockaddr(sa: *const libc::sockaddr) -> Option<IpAddr> {
    // SAFETY: caller checked sa_family before dispatching here.
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

/// Counts the prefix length from a netmask sockaddr.
#[allow(unsafe_code)]
unsafe fn prefix_from_netmask(sa: *const libc::sockaddr) -> Option<u8> {
    // SAFETY: caller passes the ifa_netmask sockaddr, valid for reading.
    match i32::from(unsafe { (*sa).sa_family }) {
        libc::AF_INET => {
            let sin: libc::sockaddr_in = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(count_ones_u8(&sin.sin_addr.s_addr.to_ne_bytes()))
        }
        libc::AF_INET6 => {
            let sin6: libc::sockaddr_in6 = unsafe { std::ptr::read_unaligned(sa.cast()) };
            Some(count_ones_u8(&sin6.sin6_addr.s6_addr))
        }
        // A zero-length netmask sockaddr (sa_family 0) means a full prefix.
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

/// The kernel interface index for a name, or 0 if unknown.
#[allow(unsafe_code)]
fn if_index(name: &str) -> u32 {
    let Ok(cname) = std::ffi::CString::new(name) else {
        return 0;
    };
    // SAFETY: cname is a valid NUL-terminated C string.
    unsafe { libc::if_nametoindex(cname.as_ptr()) }
}

// --- counters (NET_RT_IFLIST2) ---------------------------------------------

/// Reads 64-bit traffic counters for every interface, keyed by kernel index.
#[allow(unsafe_code)]
fn counters_by_index() -> PalResult<BTreeMap<u32, InterfaceCounters>> {
    let mut mib = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
    let mib_len = u32::try_from(mib.len()).unwrap_or(6);
    let mut len: libc::size_t = 0;
    // SAFETY: querying the required buffer size with a null output pointer.
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib_len,
            std::ptr::null_mut(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(PalError::Io(std::io::Error::last_os_error()));
    }
    let mut buf = vec![0u8; len];
    // SAFETY: buf has `len` bytes; sysctl fills it and updates `len`.
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib_len,
            buf.as_mut_ptr().cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(PalError::Io(std::io::Error::last_os_error()));
    }
    buf.truncate(len);

    let mut out = BTreeMap::new();
    let mut off = 0usize;
    while off + 4 <= buf.len() {
        let msglen = u16::from_ne_bytes([buf[off], buf[off + 1]]) as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }
        // ifm_type is the 4th byte (after u16 msglen + u8 version).
        if i32::from(buf[off + 3]) == libc::RTM_IFINFO2 {
            // SAFETY: read_unaligned copies the header out of the byte buffer
            // regardless of alignment; the RTM_IFINFO2 message is >= its size.
            let hdr: libc::if_msghdr2 =
                unsafe { std::ptr::read_unaligned(buf.as_ptr().add(off).cast()) };
            let d = hdr.ifm_data;
            out.insert(
                u32::from(hdr.ifm_index),
                InterfaceCounters {
                    rx_bytes: d.ifi_ibytes,
                    tx_bytes: d.ifi_obytes,
                    rx_packets: d.ifi_ipackets,
                    tx_packets: d.ifi_opackets,
                    rx_errors: d.ifi_ierrors,
                    tx_errors: d.ifi_oerrors,
                    rx_dropped: d.ifi_iqdrops,
                    tx_dropped: 0,
                },
            );
        }
        off += msglen;
    }
    Ok(out)
}

// --- PF_ROUTE watch source -------------------------------------------------

/// A [`watch::WakeSource`] that fires whenever the `PF_ROUTE` socket delivers a
/// routing message (link up/down, address change, adapter add/remove).
struct RouteWake {
    rx: tokio::sync::mpsc::Receiver<()>,
}

impl RouteWake {
    #[allow(unsafe_code)]
    fn open() -> PalResult<Self> {
        // SAFETY: a plain socket() call; the returned fd is checked below.
        let fd = unsafe { libc::socket(libc::PF_ROUTE, libc::SOCK_RAW, 0) };
        if fd < 0 {
            return Err(PalError::Io(std::io::Error::last_os_error()));
        }
        // A capacity-1 channel coalesces bursts: many routing messages collapse
        // into one pending wake, which is all the re-enumeration needs.
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                // SAFETY: reading into a local buffer from our own socket fd.
                let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
                if n <= 0 {
                    if n < 0
                        && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
                    {
                        continue;
                    }
                    break;
                }
                if tx.try_send(()).is_err() && tx.is_closed() {
                    break;
                }
            }
            // SAFETY: closing our own fd as the thread exits.
            unsafe { libc::close(fd) };
        });
        Ok(Self { rx })
    }
}

#[async_trait::async_trait]
impl watch::WakeSource for RouteWake {
    async fn next(&mut self) -> PalResult<()> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| PalError::Platform("PF_ROUTE watcher ended".to_owned()))
    }
}

/// A wake source that never fires; the watch loop then relies solely on its
/// backstop timer (used when `PF_ROUTE` is unavailable).
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
        let ifaces = MacosInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        let lo = list
            .iter()
            .find(|i| i.name == "lo0")
            .expect("lo0 should be present");
        assert_eq!(lo.kind, InterfaceKind::Loopback);
        assert!(lo.is_up);
        // A second enumeration yields the same stable id (F-13).
        let again = ifaces.list().await.unwrap();
        assert!(again.iter().any(|i| i.id == lo.id));
    }

    #[tokio::test]
    async fn reports_counters_for_loopback() {
        let ifaces = MacosInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        let lo = list.iter().find(|i| i.name == "lo0").unwrap();
        // Loopback exists, so counters resolve (even if traffic is zero).
        let c = ifaces.counters(&lo.id).await.unwrap();
        assert!(c.is_some(), "loopback should report counters");
    }
}
