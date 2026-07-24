//! Mock implementation of [`RoutingNat`], for exercising the Guard/Transaction
//! rollback pattern under injected failure.
//!
//! This is the concrete demonstration of the fail-open invariant at the PAL
//! level (AD-5): a compound operation applies forwarding then NAT, and if the
//! second step fails, rolling back the transaction restores the machine exactly
//! as it was — verified in the tests by inspecting the in-memory state.

// Test-support code: methods lock a Mutex and panic on poison. See mock/fault.rs.
#![allow(clippy::missing_panics_doc)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::error::PalResult;
use crate::guard::{Guard, Revert};
use crate::mock::fault::FaultInjector;
use crate::routing::{IpFamily, NatSpec, RoutingNat};

/// In-memory routing state the mock manipulates and reverts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RoutingState {
    /// Whether IPv4 forwarding is enabled.
    pub forwarding_v4: bool,
    /// Whether IPv6 forwarding is enabled.
    pub forwarding_v6: bool,
    /// Currently-installed NAT rules.
    pub nats: Vec<NatSpec>,
}

/// A mock [`RoutingNat`] backend.
pub struct MockRouting {
    state: Arc<Mutex<RoutingState>>,
    faults: FaultInjector,
}

impl MockRouting {
    /// A new backend with forwarding off and no NAT.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RoutingState::default())),
            faults: FaultInjector::default(),
        }
    }

    /// The fault injector, to arm failures/latency.
    #[must_use]
    pub fn faults(&self) -> &FaultInjector {
        &self.faults
    }

    /// A snapshot of the current routing state, for test assertions.
    #[must_use]
    pub fn snapshot(&self) -> RoutingState {
        self.state.lock().expect("routing state mutex").clone()
    }
}

impl Default for MockRouting {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MockRouting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockRouting")
            .field("state", &self.snapshot())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RoutingNat for MockRouting {
    async fn enable_forwarding(&self, family: IpFamily) -> PalResult<Guard> {
        self.faults.check("routing.enable_forwarding").await?;

        let prior = {
            let mut state = self.state.lock().expect("routing state mutex");
            let slot = match family {
                IpFamily::V4 => &mut state.forwarding_v4,
                IpFamily::V6 => &mut state.forwarding_v6,
            };
            let prior = *slot;
            *slot = true;
            prior
        };

        Ok(Guard::new(
            "routing.forwarding",
            Box::new(ForwardingRevert {
                state: self.state.clone(),
                family,
                prior,
            }),
        ))
    }

    async fn add_nat(&self, spec: &NatSpec) -> PalResult<Guard> {
        self.faults.check("routing.add_nat").await?;

        self.state
            .lock()
            .expect("routing state mutex")
            .nats
            .push(spec.clone());

        Ok(Guard::new(
            "routing.nat",
            Box::new(NatRevert {
                state: self.state.clone(),
                spec: spec.clone(),
            }),
        ))
    }
}

/// Restores forwarding to its exact prior value (F-9: not merely "off").
struct ForwardingRevert {
    state: Arc<Mutex<RoutingState>>,
    family: IpFamily,
    prior: bool,
}

#[async_trait]
impl Revert for ForwardingRevert {
    async fn revert(&self) -> PalResult<()> {
        let mut state = self.state.lock().expect("routing state mutex");
        match self.family {
            IpFamily::V4 => state.forwarding_v4 = self.prior,
            IpFamily::V6 => state.forwarding_v6 = self.prior,
        }
        Ok(())
    }
}

/// Removes a previously-added NAT rule.
struct NatRevert {
    state: Arc<Mutex<RoutingState>>,
    spec: NatSpec,
}

#[async_trait]
impl Revert for NatRevert {
    async fn revert(&self) -> PalResult<()> {
        let mut state = self.state.lock().expect("routing state mutex");
        // Remove the first matching rule; idempotent if already gone.
        if let Some(pos) = state.nats.iter().position(|n| *n == self.spec) {
            state.nats.remove(pos);
        }
        Ok(())
    }
}
