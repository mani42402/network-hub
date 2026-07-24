//! Per-process attribution of network flows.
//!
//! Answers "which application owns this connection?" for per-app monitoring.
//! Availability varies (Windows: IP Helper/ETW; Linux: eBPF, kernel-dependent;
//! macOS: deferred — §3.4), so this is capability-gated like the rest.
//!
//! Implemented from M2 onward where the platform allows; shape fixed here.

use std::net::SocketAddr;

use crate::error::PalResult;

/// Attributes network flows to the processes that own them.
#[async_trait::async_trait]
pub trait ProcessAttrib: Send + Sync {
    /// Whether per-process attribution works on this machine at all.
    fn is_available(&self) -> bool;

    /// Resolves the process owning a local socket endpoint, if it can be
    /// determined. `None` is a normal answer (the socket may have closed, or the
    /// owner may be a kernel flow), not an error.
    async fn owner_of(
        &self,
        local: SocketAddr,
        protocol: Protocol,
    ) -> PalResult<Option<ProcessInfo>>;
}

/// Transport protocol of a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
}

/// The process owning a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    /// The process id.
    pub pid: u32,
    /// The executable name, for display. Never a full path with a username in
    /// it, to avoid leaking home-directory paths into analytics.
    pub name: String,
}
