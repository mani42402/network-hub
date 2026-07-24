//! Firewall backend: atomic application of a compiled rule program.
//!
//! The split (per §5.1, §8.3): `nh-firewall` owns the canonical rule model and
//! compiles a rule-set revision into a **platform-neutral** [`FirewallProgram`];
//! the PAL backend translates that to the OS mechanism (nftables / WFP / PF) and
//! applies it **atomically** — swap the whole ruleset or nothing. Keeping the
//! trait input neutral is what lets business logic stay OS-agnostic (AD-7).
//!
//! The concrete rule types are defined in WP-M2.3 when `nh-firewall` lands; the
//! trait shape is fixed here so the three backends can be built against it.

use serde::{Deserialize, Serialize};

use crate::error::PalResult;
use crate::guard::Guard;

/// What the firewall backend can do on this machine, surfaced into the
/// capability matrix (AD-6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallCaps {
    /// Per-device rules resolvable to a current IP+MAC.
    pub per_device: bool,
    /// IPv6 rule support (a v4-only backend would be a bypass risk, E-17).
    pub ipv6: bool,
    /// Whether the backend can report per-rule hit counters.
    pub counters: bool,
}

/// A platform-neutral, compiled firewall program: the complete NetworkHub-owned
/// rule set as one atomic unit. Produced by `nh-firewall`'s compiler.
///
/// Minimal for now — a versioned container. WP-M2.3 fills in the rule
/// representation; the point today is that `apply` takes one self-contained
/// program and swaps it atomically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallProgram {
    /// The rule-set revision this program was compiled from, for audit and
    /// rollback correlation (§7.2).
    pub revision: u64,
    // Rule contents land in WP-M2.3.
}

/// Applies NetworkHub's firewall program to the OS, atomically.
#[async_trait::async_trait]
pub trait FirewallBackend: Send + Sync {
    /// What this backend supports on this machine.
    fn capabilities(&self) -> FirewallCaps;

    /// Atomically replaces the NetworkHub-owned firewall program. The returned
    /// [`Guard`] restores the previously-active program. Foreign firewall state
    /// (firewalld, Defender, Apple's pf.conf) is never touched (AD-11).
    async fn apply(&self, program: &FirewallProgram) -> PalResult<Guard>;

    /// Removes all NetworkHub-owned firewall state. Used by uninstall and the
    /// boot-time deadman cleanup (F-2). Idempotent: clearing when nothing is
    /// present succeeds.
    async fn clear(&self) -> PalResult<()>;
}
