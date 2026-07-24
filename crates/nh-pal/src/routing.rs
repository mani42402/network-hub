//! IP forwarding and NAT — the plumbing behind connection sharing.
//!
//! Enabling forwarding and adding NAT is how a hotspot shares an upstream
//! connection. Both are reversible and journaled: the `sysctl` `ip_forward`
//! change in particular must be restored to its prior value on teardown and
//! uninstall, or the host silently keeps acting as a router (F-9).
//!
//! Implemented in WP-M2.7 (Linux first); shape fixed here.

use crate::error::PalResult;
use crate::guard::Guard;
use crate::interface::InterfaceId;

/// Enables IP forwarding and manages NAT for connection sharing.
#[async_trait::async_trait]
pub trait RoutingNat: Send + Sync {
    /// Enables IP forwarding for the given family, returning a [`Guard`] that
    /// restores the previous kernel setting exactly (F-9) — not merely "off",
    /// since the user may have had it on already.
    async fn enable_forwarding(&self, family: IpFamily) -> PalResult<Guard>;

    /// Adds source NAT (masquerade) from an internal network out through an
    /// upstream interface, returning a [`Guard`] that removes it.
    async fn add_nat(&self, spec: &NatSpec) -> PalResult<Guard>;
}

/// Which IP family an operation applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpFamily {
    /// IPv4.
    V4,
    /// IPv6.
    V6,
}

/// A source-NAT rule: masquerade traffic from `internal` out via `upstream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatSpec {
    /// The interface facing the shared clients (the hotspot/bridge interface).
    pub internal: InterfaceId,
    /// The interface facing the internet, which the upstream may change to as
    /// routes change (E-1) — the daemon re-adds NAT when it does.
    pub upstream: InterfaceId,
}
