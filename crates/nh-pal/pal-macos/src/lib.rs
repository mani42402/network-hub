//! macOS PAL backend: PF anchors, SystemConfiguration, utun.
//!
//! **Status:** the network-feature backends are stubs — see
//! `docs/architecture/EXECUTION_PLAN.md` for the work packages that fill them
//! in. What *is* implemented is the WP-M0.8 capability probe ([`probe`]), which
//! gathers the OS, privilege, and interface facts the daemon publishes.
//!
//! The probe exists only when compiling for macOS; on other platforms the crate
//! is an empty stub so the workspace still builds everywhere.

#[cfg(target_os = "macos")]
pub mod interfaces;
#[cfg(target_os = "macos")]
pub mod probe;

#[cfg(target_os = "macos")]
pub use interfaces::MacosInterfaces;
#[cfg(target_os = "macos")]
pub use probe::MacosProbe;
