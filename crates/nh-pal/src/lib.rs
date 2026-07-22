//! # nh-pal — the Platform Adapter Layer
//!
//! The seam between NetworkHub's OS-agnostic logic and the very different
//! mechanisms each operating system provides (AD-7, §5.2). This crate defines
//! **traits only** — one per platform concern — plus a mock backend for tests.
//! The three real backends (`pal-linux`, `pal-windows`, `pal-macos`) implement
//! these traits; feature crates depend on the traits and never on a concrete
//! backend, so business logic stays testable against [`mock`] with no root and
//! no hardware. `cargo xtask check-layers` enforces that rule.
//!
//! ## The concerns
//!
//! - [`interface::InterfaceInfo`] — enumerate interfaces and watch changes
//!   (the one M0 implements for real).
//! - [`firewall::FirewallBackend`] — apply a compiled firewall program atomically.
//! - [`routing::RoutingNat`] — IP forwarding and NAT for connection sharing.
//! - [`wifi::WifiApControl`] — host a Wi-Fi access point (hardware-variable).
//! - [`shaper::TrafficShaper`] — per-device bandwidth limits.
//! - [`dns::DnsSystemConfig`] — point interfaces at our resolver, reversibly.
//! - [`process::ProcessAttrib`] — attribute flows to processes.
//!
//! ## Two conventions every backend follows (§5.2)
//!
//! 1. **Every mutating operation returns a [`guard::Guard`]** that undoes it, so
//!    a failed multi-step operation rolls back cleanly (via [`guard::Transaction`])
//!    and the durable intent journal (nh-state) can recover a crash (AD-5).
//! 2. **Operations are idempotent where the OS allows** — "ensure this exists",
//!    not "add this" — so a reconcile that runs twice does not double-apply.

pub mod dns;
pub mod error;
pub mod firewall;
pub mod guard;
pub mod interface;
pub mod process;
pub mod routing;
pub mod shaper;
pub mod wifi;

#[cfg(feature = "mock")]
pub mod mock;

// The most-used items at the crate root.
pub use error::{PalError, PalResult};
pub use guard::{Guard, Revert, Transaction};
pub use interface::{Interface, InterfaceEvent, InterfaceId, InterfaceInfo, InterfaceKind, IpCidr};
