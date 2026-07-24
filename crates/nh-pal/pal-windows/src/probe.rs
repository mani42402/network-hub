//! The Windows system probe (WP-M0.8).
//!
//! Gathers [`SystemFacts`] via the Win32 API: the OS version from
//! `RtlGetVersion` (accurate, unlike `GetVersionEx` without a manifest), the
//! elevation state from the process token, and the interface list from IP
//! Helper's `GetAdaptersAddresses`. This crate is the designated FFI boundary
//! (`unsafe_code = "deny"`, opted into per block with a SAFETY note). The richer,
//! watched interface model lands in WP-M0.9.

use nh_pal::probe::{InterfaceKind, Os, ProbeError, ProbedInterface, SystemFacts, SystemProbe};

// IANA ifType values (ipifcons.h) used to classify adapters.
const IF_TYPE_ETHERNET_CSMACD: u32 = 6;
const IF_TYPE_PPP: u32 = 23;
const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
const IF_TYPE_IEEE80211: u32 = 71;
const IF_TYPE_TUNNEL: u32 = 131;
// IF_OPER_STATUS: IfOperStatusUp.
const IF_OPER_STATUS_UP: i32 = 1;

/// The Windows implementation of [`SystemProbe`].
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsProbe;

impl SystemProbe for WindowsProbe {
    fn facts(&self) -> Result<SystemFacts, ProbeError> {
        Ok(SystemFacts {
            os: Os::Windows,
            os_version: os_version(),
            arch: std::env::consts::ARCH.to_owned(),
            privileged: privileged(),
            interfaces: interfaces()?,
        })
    }
}

/// The OS version from `RtlGetVersion` (build number included, so it is exact).
#[allow(unsafe_code)]
fn os_version() -> String {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

    let size = u32::try_from(std::mem::size_of::<OSVERSIONINFOW>()).unwrap_or(0);
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: size,
        dwMajorVersion: 0,
        dwMinorVersion: 0,
        dwBuildNumber: 0,
        dwPlatformId: 0,
        szCSDVersion: [0u16; 128],
    };
    // SAFETY: `info` is a valid OSVERSIONINFOW with its size field set;
    // RtlGetVersion only writes into it and returns STATUS_SUCCESS (0) on success.
    let status = unsafe { RtlGetVersion(&raw mut info) };
    if status != 0 {
        return "Windows (version unavailable)".to_owned();
    }
    let (major, minor, build) = (info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber);
    // Build 22000 is the Windows 10 → 11 boundary; both report major version 10.
    let name = if major == 10 && build >= 22000 {
        "Windows 11"
    } else if major == 10 {
        "Windows 10"
    } else {
        "Windows"
    };
    format!("{name} {major}.{minor} (build {build})")
}

/// Whether the process token is elevated (the Administrator analogue of root).
#[allow(unsafe_code)]
fn privileged() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo-handle (must not be closed);
    // OpenProcessToken writes the process token into `token`, returning nonzero
    // on success.
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) };
    if opened == 0 {
        return false;
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut ret_len: u32 = 0;
    let size = u32::try_from(std::mem::size_of::<TOKEN_ELEVATION>()).unwrap_or(0);
    // SAFETY: `token` is a valid token handle; GetTokenInformation writes a
    // TOKEN_ELEVATION into `elevation` (buffer sized to it) and the byte count
    // into `ret_len`.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&raw mut elevation).cast(),
            size,
            &raw mut ret_len,
        )
    };
    // SAFETY: `token` came from OpenProcessToken and is not the pseudo-handle.
    let _ = unsafe { CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
}

/// Enumerates network adapters via IP Helper.
#[allow(unsafe_code)]
fn interfaces() -> Result<Vec<ProbedInterface>, ProbeError> {
    use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };

    const AF_UNSPEC: u32 = 0;
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let struct_size = std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>().max(1);

    // Start with the 15 KB buffer Microsoft recommends; grow to the reported size
    // on overflow. Bounded so a pathological driver cannot spin us forever.
    let mut size: u32 = 15 * 1024;
    let mut buffer: Vec<IP_ADAPTER_ADDRESSES_LH> = Vec::new();
    let mut filled = false;
    for _ in 0..4 {
        let count = (size as usize / struct_size) + 1;
        buffer = Vec::with_capacity(count);
        let mut io_size = size;
        // SAFETY: `buffer` has room for `count` structs (>= `size` bytes) and is
        // aligned to IP_ADAPTER_ADDRESSES_LH; GetAdaptersAddresses fills it and
        // writes the used/required byte count into `io_size`.
        let ret = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC,
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
        return Err(ProbeError::Io(format!(
            "GetAdaptersAddresses failed with code {ret}"
        )));
    }
    if !filled {
        return Err(ProbeError::Io(
            "GetAdaptersAddresses did not converge on a buffer size".to_owned(),
        ));
    }

    let mut out = Vec::new();
    let mut cur: *const IP_ADAPTER_ADDRESSES_LH = buffer.as_ptr();
    while !cur.is_null() {
        // SAFETY: `cur` points at a live IP_ADAPTER_ADDRESSES_LH node that
        // GetAdaptersAddresses linked; we only read its fields.
        let adapter = unsafe { &*cur };
        // SAFETY: FriendlyName is a NUL-terminated wide string owned by the buffer.
        let name = unsafe { wide_to_string(adapter.FriendlyName) };
        out.push(ProbedInterface {
            name,
            kind: classify_iftype(adapter.IfType),
            up: adapter.OperStatus == IF_OPER_STATUS_UP,
        });
        cur = adapter.Next;
    }
    Ok(out)
}

fn classify_iftype(if_type: u32) -> InterfaceKind {
    match if_type {
        IF_TYPE_SOFTWARE_LOOPBACK => InterfaceKind::Loopback,
        IF_TYPE_IEEE80211 => InterfaceKind::Wireless,
        IF_TYPE_ETHERNET_CSMACD => InterfaceKind::Ethernet,
        IF_TYPE_TUNNEL | IF_TYPE_PPP => InterfaceKind::Virtual,
        _ => InterfaceKind::Other,
    }
}

/// Reads a NUL-terminated UTF-16 string into a `String`.
///
/// # Safety
/// `ptr` must be null or point to a valid NUL-terminated wide string.
#[allow(unsafe_code)]
unsafe fn wide_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    // SAFETY: the caller guarantees a NUL-terminated wide string, so scanning to
    // the terminator stays in bounds.
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `ptr` is valid for `len` u16 elements per the scan above.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_windows_and_some_interface() {
        let facts = WindowsProbe.facts().unwrap();
        assert_eq!(facts.os, Os::Windows);
        assert!(facts.os_version.starts_with("Windows"));
        assert!(!facts.arch.is_empty());
        // Every Windows host has at least a loopback adapter.
        assert!(
            !facts.interfaces.is_empty(),
            "expected at least one interface"
        );
    }

    #[test]
    fn iftype_classification() {
        assert_eq!(
            classify_iftype(IF_TYPE_SOFTWARE_LOOPBACK),
            InterfaceKind::Loopback
        );
        assert_eq!(classify_iftype(IF_TYPE_IEEE80211), InterfaceKind::Wireless);
        assert_eq!(
            classify_iftype(IF_TYPE_ETHERNET_CSMACD),
            InterfaceKind::Ethernet
        );
        assert_eq!(classify_iftype(IF_TYPE_TUNNEL), InterfaceKind::Virtual);
        assert_eq!(classify_iftype(9999), InterfaceKind::Other);
    }
}
