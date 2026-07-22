//! System DNS configuration — pointing an interface (or the host) at our
//! resolver.
//!
//! Setting per-interface DNS is how the embedded resolver (DoH/DoT, filtering)
//! takes effect. It must be reversible with a restore-on-teardown guard and a
//! boot-time restore, or the machine is left pointing at a resolver that is no
//! longer running (§8.4). On Linux this integrates with systemd-resolved rather
//! than fighting the stub listener (AD-11).
//!
//! Implemented in WP-M2.5 / P2.3; shape fixed here.

use std::net::IpAddr;

use crate::error::PalResult;
use crate::guard::Guard;
use crate::interface::InterfaceId;

/// Configures which DNS servers an interface (or the host) uses.
#[async_trait::async_trait]
pub trait DnsSystemConfig: Send + Sync {
    /// Sets the DNS servers for one interface, returning a [`Guard`] that
    /// restores the previous configuration exactly. Used to point hotspot
    /// clients, or the host itself, at NetworkHub's resolver.
    async fn set_interface_dns(
        &self,
        interface: &InterfaceId,
        servers: &[IpAddr],
    ) -> PalResult<Guard>;

    /// Reports the current DNS servers for an interface, so the daemon can
    /// detect external changes it needs to reconcile.
    async fn get_interface_dns(&self, interface: &InterfaceId) -> PalResult<Vec<IpAddr>>;
}
