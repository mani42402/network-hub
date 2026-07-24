//! Windows PAL backend: WFP, WinRT tethering, IP Helper.
//!
//! **Status:** the network-feature backends (WFP firewall, WinRT tethering, IP
//! Helper) are stubs — see `docs/architecture/EXECUTION_PLAN.md` for the work
//! package that fills them in.
//!
//! What *is* implemented here is the Windows platform glue the daemon composes
//! in at its edges, because it needs FFI and this crate is the designated FFI
//! boundary (ADR-0002, §5.1):
//!
//! - [`ipc`] — gRPC over a named pipe, the Windows analogue of a Unix domain
//!   socket, with peer identification via `GetNamedPipeClientProcessId`.
//! - [`service`] — Windows Service (SCM) registration, start/stop dispatch,
//!   crash-recovery actions, and the on-start orphan check (the F-2 analogue).
//! - [`probe`] — the WP-M0.8 capability probe: OS version, privilege elevation,
//!   and interface enumeration via IP Helper.
//!
//! These modules exist only when compiling for Windows; on other platforms the
//! crate is an empty stub so the workspace still builds everywhere.

#[cfg(windows)]
pub mod interfaces;
#[cfg(windows)]
pub mod ipc;
#[cfg(windows)]
pub mod probe;
#[cfg(windows)]
pub mod service;

#[cfg(windows)]
pub use interfaces::WindowsInterfaces;
#[cfg(windows)]
pub use probe::WindowsProbe;
