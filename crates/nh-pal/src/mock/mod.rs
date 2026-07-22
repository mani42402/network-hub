//! The mock platform backend.
//!
//! `pal-mock` implements PAL traits against in-memory state, so reconcilers,
//! state machines, and the daemon can be exercised with **no root and no
//! hardware** (AD-7). It is the foundation of the fault-injection suite: it can
//! be told to fail a specific operation, fail after N operations, or add
//! latency, which is how tests drive the fail-open invariant (AD-5) without
//! needing a real OS to misbehave.
//!
//! Interface event simulation is **driven by explicit method calls**, not
//! background timers — `simulate_unplug`, `simulate_rename`, and so on push
//! events synchronously, so tests control exactly when things happen and stay
//! deterministic.
//!
//! For M0 the mock implements the two traits M0 needs to prove the patterns:
//! [`MockInterfaces`] (the read-and-watch concern M0 ships) and [`MockRouting`]
//! (a mutating concern, to exercise Guard/Transaction rollback under injected
//! failure). The remaining backends are mocked as their feature milestones land.
//!
//! Enabled by the `mock` feature; never linked into a production build.

mod fault;
mod interfaces;
mod routing;

pub use fault::{FaultInjector, FaultKind, FaultPlan};
pub use interfaces::MockInterfaces;
pub use routing::{MockRouting, RoutingState};
