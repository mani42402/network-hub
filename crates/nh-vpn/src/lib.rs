//! VPN profiles, WireGuard engine, OpenVPN process control, kill switch, and
//! leak prevention.
//!
//! **Status (WP-M1.2):** the profile layer is landing here — the parsers and the
//! hostile-input hardening that make import a real security boundary (§8.2). A
//! VPN profile arrives from a file, a paste, or a QR code — all untrusted — so
//! every parser enforces size/encoding caps ([`limits`]) before doing any work,
//! and the OpenVPN importer additionally *sanitises* the config, refusing any
//! directive that can execute code as root.
//!
//! Secret material (`PrivateKey`, `PresharedKey`, inline OpenVPN keys) is held in
//! [`nh_types::Secret`] so it never reaches a log or `Debug`; the daemon extracts
//! it to the keychain on import (WP-M1.1).
//!
//! The WireGuard engine, OpenVPN controller, kill switch, and leak prevention
//! land in later work packages (WP-M1.3+).

pub mod error;
pub mod limits;
pub mod types;
pub mod wireguard;

pub use error::{VpnError, VpnResult};
pub use types::{Cidr, Endpoint};
pub use wireguard::{WgInterface, WgPeer, WireguardProfile};
