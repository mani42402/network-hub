//! Per-device bandwidth shaping and quota enforcement.
//!
//! Fully supported on Linux (tc/CAKE), unavailable on Windows without a signed
//! kernel driver (R-3), partial on macOS. So, like Wi-Fi, the trait leads with a
//! capability report and the daemon offers shaping only where it works — and
//! reports accounting-only honestly where it does not (§3.4).
//!
//! Implemented in P2.2; shape fixed here.

use nh_types::BitRate;
use serde::{Deserialize, Serialize};

use crate::error::PalResult;
use crate::guard::Guard;
use crate::interface::InterfaceId;

/// What shaping this machine supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapingCaps {
    /// Whether per-device rate limits can be enforced (Linux: yes; Windows v1:
    /// no — accounting only).
    pub per_device_limits: bool,
    /// Whether ingress (download) shaping is possible, not just egress.
    pub ingress: bool,
}

/// Shapes traffic for individual devices.
#[async_trait::async_trait]
pub trait TrafficShaper: Send + Sync {
    /// What shaping is possible here.
    fn capabilities(&self) -> ShapingCaps;

    /// Applies a rate limit to a device (identified by its current address on
    /// the given interface), returning a [`Guard`] that removes the limit. The
    /// daemon re-applies when the device's address changes.
    async fn set_device_limit(
        &self,
        interface: &InterfaceId,
        device_addr: std::net::IpAddr,
        limit: RateLimit,
    ) -> PalResult<Guard>;
}

/// A bidirectional rate limit. `None` on a direction means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    /// Download (ingress) cap; `None` means unlimited.
    pub download: Option<BitRate>,
    /// Upload (egress) cap; `None` means unlimited.
    pub upload: Option<BitRate>,
}
