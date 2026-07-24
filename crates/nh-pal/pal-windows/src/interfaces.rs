//! Windows [`InterfaceInfo`] + [`InterfaceStats`] with a `NotifyIpInterfaceChange`
//! watcher (WP-M0.9).
//!
//! Enumeration, addresses, MAC, and the stable-id GUID come from
//! `GetAdaptersAddresses` (the same call the M0.8 probe uses); 64-bit counters
//! from `GetIfEntry2`; and change notifications from `NotifyIpInterfaceChange`,
//! whose callback wakes the shared watch loop
//! ([`nh_pal::interface::watch`]) — notification-as-hint, enumeration-as-truth.
//! All raw FFI, isolated to this designated boundary crate.

use std::ffi::CStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use nh_pal::error::{PalError, PalResult};
use nh_pal::interface::{
    Interface, InterfaceCounters, InterfaceEvent, InterfaceEventStream, InterfaceId, InterfaceInfo,
    InterfaceKind, InterfaceStats, IpCidr, resolve_id, watch,
};
use nh_types::MacAddr;
use tokio::sync::broadcast;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, HANDLE, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
    GetIfEntry2, IP_ADAPTER_ADDRESSES_LH, MIB_IF_ROW2, MIB_IPINTERFACE_ROW, MIB_NOTIFICATION_TYPE,
    NotifyIpInterfaceChange,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR};

const EVENT_CAPACITY: usize = 256;
const BACKSTOP: Duration = Duration::from_secs(30);
const POLL_BACKSTOP: Duration = Duration::from_secs(2);

// IANA ifType values used to classify adapters.
const IF_TYPE_ETHERNET_CSMACD: u32 = 6;
const IF_TYPE_PPP: u32 = 23;
const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
const IF_TYPE_TUNNEL: u32 = 131;
const IF_TYPE_IEEE80211: u32 = 71;
const IF_OPER_STATUS_UP: i32 = 1;

/// The Windows interface backend.
pub struct WindowsInterfaces {
    events: broadcast::Sender<InterfaceEvent>,
}

impl std::fmt::Debug for WindowsInterfaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsInterfaces").finish_non_exhaustive()
    }
}

impl WindowsInterfaces {
    /// Builds the backend and starts the watch loop on the current runtime.
    ///
    /// `NotifyIpInterfaceChange` gives near-instant change wakes; if registration
    /// fails, the loop falls back to a short-interval backstop poll.
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
        match ChangeWake::open() {
            Ok(wake) => {
                tokio::spawn(watch::run(info, Box::new(wake), BACKSTOP, events));
            }
            Err(e) => {
                tracing::warn!("NotifyIpInterfaceChange unavailable, polling instead: {e}");
                tokio::spawn(watch::run(info, Box::new(NeverWake), POLL_BACKSTOP, events));
            }
        }
        Ok(this)
    }
}

#[async_trait::async_trait]
impl InterfaceInfo for WindowsInterfaces {
    async fn list(&self) -> PalResult<Vec<Interface>> {
        Ok(scan()?.into_iter().map(|s| s.interface).collect())
    }

    async fn get(&self, id: &InterfaceId) -> PalResult<Option<Interface>> {
        Ok(scan()?
            .into_iter()
            .map(|s| s.interface)
            .find(|i| &i.id == id))
    }

    fn watch(&self) -> InterfaceEventStream {
        self.events.subscribe()
    }
}

#[async_trait::async_trait]
impl InterfaceStats for WindowsInterfaces {
    async fn counters(&self, id: &InterfaceId) -> PalResult<Option<InterfaceCounters>> {
        let Some(scanned) = scan()?.into_iter().find(|s| &s.interface.id == id) else {
            return Ok(None);
        };
        Ok(Some(read_counters(scanned.luid)))
    }
}

// --- enumeration (GetAdaptersAddresses) ------------------------------------

/// An enumerated adapter plus its LUID (kept for the counter lookup).
struct Scanned {
    interface: Interface,
    luid: u64,
}

/// Walks `GetAdaptersAddresses` into interfaces + their LUIDs.
#[allow(unsafe_code)]
fn scan() -> PalResult<Vec<Scanned>> {
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let struct_size = std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>().max(1);

    let mut size: u32 = 15 * 1024;
    let mut buffer: Vec<IP_ADAPTER_ADDRESSES_LH> = Vec::new();
    let mut filled = false;
    for _ in 0..4 {
        let count = (size as usize / struct_size) + 1;
        buffer = Vec::with_capacity(count);
        let mut io_size = size;
        // SAFETY: `buffer` is aligned to IP_ADAPTER_ADDRESSES_LH and holds
        // `count` structs (>= `size` bytes); the call fills it and updates size.
        let ret = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC),
                flags,
                std::ptr::null(),
                buffer.as_mut_ptr(),
                &raw mut io_size,
            )
        };
        if ret == NO_ERROR {
            filled = true;
            break;
        }
        if ret == ERROR_BUFFER_OVERFLOW {
            size = io_size;
            continue;
        }
        return Err(PalError::Platform(format!(
            "GetAdaptersAddresses failed with code {ret}"
        )));
    }
    if !filled {
        return Err(PalError::Platform(
            "GetAdaptersAddresses did not converge on a buffer size".to_owned(),
        ));
    }

    let mut out = Vec::new();
    let mut cur: *const IP_ADAPTER_ADDRESSES_LH = buffer.as_ptr();
    while !cur.is_null() {
        // SAFETY: `cur` is a live node GetAdaptersAddresses linked; read-only.
        let adapter = unsafe { &*cur };
        out.push(scan_adapter(adapter));
        cur = adapter.Next;
    }
    Ok(out)
}

#[allow(unsafe_code)]
fn scan_adapter(adapter: &IP_ADAPTER_ADDRESSES_LH) -> Scanned {
    // SAFETY: FriendlyName is a NUL-terminated wide string; AdapterName an ASCII one.
    let name = unsafe { wide_to_string(adapter.FriendlyName) };
    let guid = unsafe { ascii_to_string(adapter.AdapterName) };
    let kind = classify(adapter.IfType);
    let mac = mac_from(adapter);
    let addresses = unsafe { addresses_of(adapter) };
    // SAFETY: NET_LUID_LH is a union of integers; its raw u64 value is stable.
    let luid = unsafe { adapter.Luid.Value };

    let interface = Interface {
        id: resolve_id(kind, mac, Some(&guid), &name),
        is_up: adapter.OperStatus == IF_OPER_STATUS_UP,
        mac,
        addresses,
        kind,
        name,
    };
    Scanned { interface, luid }
}

fn classify(if_type: u32) -> InterfaceKind {
    match if_type {
        IF_TYPE_SOFTWARE_LOOPBACK => InterfaceKind::Loopback,
        IF_TYPE_IEEE80211 => InterfaceKind::Wifi,
        IF_TYPE_ETHERNET_CSMACD => InterfaceKind::Ethernet,
        IF_TYPE_TUNNEL | IF_TYPE_PPP => InterfaceKind::Tunnel,
        _ => InterfaceKind::Other,
    }
}

fn mac_from(adapter: &IP_ADAPTER_ADDRESSES_LH) -> Option<MacAddr> {
    if adapter.PhysicalAddressLength != 6 {
        return None;
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&adapter.PhysicalAddress[..6]);
    if mac == [0u8; 6] {
        None
    } else {
        Some(MacAddr::new(mac))
    }
}

/// Walks the adapter's unicast address list into [`IpCidr`]s (sorted).
#[allow(unsafe_code)]
unsafe fn addresses_of(adapter: &IP_ADAPTER_ADDRESSES_LH) -> Vec<IpCidr> {
    let mut addrs = Vec::new();
    let mut ua = adapter.FirstUnicastAddress;
    while !ua.is_null() {
        // SAFETY: `ua` is a live unicast-address node; read-only.
        let entry = unsafe { &*ua };
        // SAFETY: lpSockaddr points to a valid sockaddr for this address family.
        if let Some(addr) = unsafe { ip_from_sockaddr(entry.Address.lpSockaddr) } {
            addrs.push(IpCidr {
                addr,
                prefix_len: entry.OnLinkPrefixLength,
            });
        }
        ua = entry.Next;
    }
    addrs.sort_by(|a, b| a.addr.cmp(&b.addr).then(a.prefix_len.cmp(&b.prefix_len)));
    addrs
}

/// Reads an [`IpAddr`] from a `SOCKADDR`, taking the address octets at their
/// fixed offsets rather than through the `IN_ADDR`/`IN6_ADDR` unions.
#[allow(unsafe_code)]
unsafe fn ip_from_sockaddr(sa: *const SOCKADDR) -> Option<IpAddr> {
    if sa.is_null() {
        return None;
    }
    // SAFETY: sa_family is the sockaddr discriminant.
    let family = unsafe { (*sa).sa_family };
    let base = sa.cast::<u8>();
    if family == AF_INET {
        let mut b = [0u8; 4];
        // SAFETY: sin_addr is at offset 4 (sin_family:u16 + sin_port:u16).
        for (i, byte) in b.iter_mut().enumerate() {
            *byte = unsafe { *base.add(4 + i) };
        }
        Some(IpAddr::V4(Ipv4Addr::from(b)))
    } else if family == AF_INET6 {
        let mut b = [0u8; 16];
        // SAFETY: sin6_addr is at offset 8 (family:u16 + port:u16 + flowinfo:u32).
        for (i, byte) in b.iter_mut().enumerate() {
            *byte = unsafe { *base.add(8 + i) };
        }
        Some(IpAddr::V6(Ipv6Addr::from(b)))
    } else {
        None
    }
}

/// Reads a NUL-terminated UTF-16 string.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated wide string.
#[allow(unsafe_code)]
unsafe fn wide_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    // SAFETY: caller guarantees NUL termination, so the scan stays in bounds.
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `ptr` is valid for `len` u16s per the scan.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

/// Reads a NUL-terminated ASCII string (the adapter GUID).
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
#[allow(unsafe_code)]
unsafe fn ascii_to_string(ptr: *const u8) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: caller guarantees a valid NUL-terminated C string.
    unsafe { CStr::from_ptr(ptr.cast()) }
        .to_string_lossy()
        .into_owned()
}

// --- counters (GetIfEntry2) ------------------------------------------------

#[allow(unsafe_code)]
fn read_counters(luid: u64) -> InterfaceCounters {
    // SAFETY: MIB_IF_ROW2 is all-POD; a zeroed value is valid, and we set the
    // LUID key GetIfEntry2 looks up.
    let mut row: MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
    row.InterfaceLuid.Value = luid;
    // SAFETY: `row` is a valid MIB_IF_ROW2 with its LUID set; GetIfEntry2 fills it.
    if unsafe { GetIfEntry2(&raw mut row) } != NO_ERROR {
        return InterfaceCounters::default();
    }
    InterfaceCounters {
        rx_bytes: row.InOctets,
        tx_bytes: row.OutOctets,
        rx_packets: row.InUcastPkts.saturating_add(row.InNUcastPkts),
        tx_packets: row.OutUcastPkts.saturating_add(row.OutNUcastPkts),
        rx_errors: row.InErrors,
        tx_errors: row.OutErrors,
        rx_dropped: row.InDiscards,
        tx_dropped: row.OutDiscards,
    }
}

// --- NotifyIpInterfaceChange watch source ----------------------------------

/// A [`watch::WakeSource`] fed by the `NotifyIpInterfaceChange` callback.
struct ChangeWake {
    rx: tokio::sync::mpsc::Receiver<()>,
}

/// The C callback Windows invokes on an interface change. `context` is the
/// leaked `Sender` we registered.
#[allow(unsafe_code)]
unsafe extern "system" fn on_change(
    context: *const core::ffi::c_void,
    _row: *const MIB_IPINTERFACE_ROW,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    if context.is_null() {
        return;
    }
    // SAFETY: `context` is the Sender pointer passed to NotifyIpInterfaceChange,
    // leaked so it outlives every callback. try_send is safe from any thread.
    let tx = unsafe { &*(context.cast::<tokio::sync::mpsc::Sender<()>>()) };
    let _ = tx.try_send(());
}

impl ChangeWake {
    #[allow(unsafe_code)]
    fn open() -> PalResult<Self> {
        // Capacity-1 coalesces a burst of change callbacks into one pending wake.
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        // Leak the sender so the raw context pointer stays valid for every
        // callback (the registration lives for the daemon's lifetime).
        let ctx = Box::into_raw(Box::new(tx));
        let mut handle: HANDLE = std::ptr::null_mut();
        // SAFETY: a well-formed registration; `on_change` matches the callback
        // ABI and `ctx` outlives it.
        let rc = unsafe {
            NotifyIpInterfaceChange(
                AF_UNSPEC,
                Some(on_change),
                ctx.cast(),
                false,
                &raw mut handle,
            )
        };
        if rc != NO_ERROR {
            // Registration failed: reclaim the leaked sender.
            // SAFETY: `ctx` came from Box::into_raw and was not handed to Windows.
            drop(unsafe { Box::from_raw(ctx) });
            return Err(PalError::Platform(format!(
                "NotifyIpInterfaceChange failed with code {rc}"
            )));
        }
        // The registration `handle` is only needed to cancel the notification;
        // a singleton daemon never does, so it is intentionally dropped (the
        // registration lives until the process exits).
        let _ = handle;
        Ok(Self { rx })
    }
}

#[async_trait::async_trait]
impl watch::WakeSource for ChangeWake {
    async fn next(&mut self) -> PalResult<()> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| PalError::Platform("interface-change watcher ended".to_owned()))
    }
}

/// A wake source that never fires; the watch loop then relies solely on its
/// backstop timer (used when the change notification cannot be registered).
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
        let ifaces = WindowsInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        // Every Windows host has a loopback pseudo-interface.
        let lo = list
            .iter()
            .find(|i| i.kind == InterfaceKind::Loopback)
            .expect("a loopback interface should be present");
        let again = ifaces.list().await.unwrap();
        assert!(again.iter().any(|i| i.id == lo.id));
    }

    #[tokio::test]
    async fn reports_counters_for_loopback() {
        let ifaces = WindowsInterfaces::new().unwrap();
        let list = ifaces.list().await.unwrap();
        let lo = list
            .iter()
            .find(|i| i.kind == InterfaceKind::Loopback)
            .unwrap();
        assert!(ifaces.counters(&lo.id).await.unwrap().is_some());
    }
}
