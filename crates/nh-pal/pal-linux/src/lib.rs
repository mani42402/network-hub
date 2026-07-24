//! Linux PAL backend: netlink, nftables, tc, hostapd, NetworkManager.
//!
//! **Status:** the network-feature backends are stubs — see
//! `docs/architecture/EXECUTION_PLAN.md` for the work packages that fill them
//! in. What *is* implemented is the WP-M0.8 capability probe ([`probe`]), which
//! gathers the OS, privilege, and interface facts the daemon publishes.
//!
//! The probe exists only when compiling for Linux; on other platforms the crate
//! is an empty stub so the workspace still builds everywhere.

#[cfg(target_os = "linux")]
pub mod interfaces;
#[cfg(target_os = "linux")]
pub mod probe;

#[cfg(target_os = "linux")]
pub use interfaces::LinuxInterfaces;
#[cfg(target_os = "linux")]
pub use probe::LinuxProbe;
