//! Wi-Fi access-point control — the most hardware-variable PAL concern.
//!
//! Whether a machine can host an access point, which bands, and whether it can
//! run AP and client mode at once are functions of the chipset and driver, not
//! the OS (§3, R-1). So the trait leads with a **probe**: the daemon reads
//! capabilities and only offers what the hardware actually supports, rather than
//! trying and failing (AD-6). macOS reports this unsupported entirely (AD-12).
//!
//! Implemented in WP-M2.7/2.8/2.9; shape fixed here.

use nh_types::Ssid;
use serde::{Deserialize, Serialize};

use crate::error::PalResult;
use crate::guard::Guard;

/// What access-point features this machine's Wi-Fi hardware supports. Probed at
/// startup and on hardware change; feeds the capability matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApCapabilities {
    /// Whether AP mode is possible at all. `false` on macOS and on adapters
    /// whose driver does not support it.
    pub ap_mode: bool,
    /// Bands the hardware can host an AP on.
    pub bands: Vec<Band>,
    /// Whether AP and client mode can run simultaneously (repeater/STA+AP),
    /// which depends on the chipset's interface-combination matrix (E-3).
    pub concurrent_sta_ap: bool,
    /// Whether the backend can isolate clients from each other (guest safety).
    pub client_isolation: bool,
}

/// A Wi-Fi band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    /// 2.4 GHz.
    Band2_4GHz,
    /// 5 GHz.
    Band5GHz,
    /// 6 GHz (Wi-Fi 6E).
    Band6GHz,
}

/// Configuration for starting an access point. Expanded in WP-M2.6 (channel
/// selection, regulatory handling, WPA2/WPA3 options); the essentials are here.
///
/// No `PartialEq`/`Eq`: it holds a [`nh_types::Secret`], which has no equality by
/// design (comparing secrets by value invites timing attacks). `Debug` is safe —
/// the secret renders as `<redacted>`.
#[derive(Debug, Clone)]
pub struct ApConfig {
    /// The network name.
    pub ssid: Ssid,
    /// The pre-shared key. Held as a secret elsewhere; the backend receives it
    /// only at start time and does not retain it.
    pub passphrase: nh_types::Secret<String>,
    /// The band to host on.
    pub band: Band,
    /// Whether to isolate clients from one another.
    pub isolate_clients: bool,
}

/// Hosts a Wi-Fi access point.
#[async_trait::async_trait]
pub trait WifiApControl: Send + Sync {
    /// Probes what access-point features the hardware supports. Cheap and
    /// synchronous where possible; the daemon caches the result.
    fn probe(&self) -> ApCapabilities;

    /// Starts an access point with the given configuration, returning a [`Guard`]
    /// that stops it and restores interface ownership (e.g. hands the interface
    /// back to NetworkManager, AD-11).
    async fn start(&self, config: &ApConfig) -> PalResult<Guard>;
}
