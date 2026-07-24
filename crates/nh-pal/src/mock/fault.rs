//! Fault injection for the mock backend.
//!
//! The whole point of the mock is to make the OS misbehave *on command*, so the
//! fail-open paths (AD-5) can be tested without a real OS to break. A test arms a
//! [`FaultPlan`] before running the code under test; the mock consults the plan
//! on every operation.

// This is test-support code. Its methods lock a Mutex and panic on poison; a
// poisoned lock means a test thread already panicked, so documenting it as a
// `# Panics` case on every setter would be noise without value.
#![allow(clippy::missing_panics_doc)]

use std::sync::Mutex;
use std::time::Duration;

use crate::error::PalError;

/// How the mock should fail (or delay) operations.
#[derive(Debug, Clone, Default)]
pub struct FaultPlan {
    /// If set, operations whose label contains this substring fail with the
    /// given error. Used to fail a specific step of a multi-step operation — the
    /// input to the "partial failure then rollback" test (AD-5).
    pub fail_ops_matching: Option<(String, FaultKind)>,
    /// If set, the Nth mutating operation (1-based) fails, whatever it is. Used
    /// to walk a compound operation and fail at each step in turn.
    pub fail_after_n: Option<(u32, FaultKind)>,
    /// Latency added to every operation, to exercise timeouts and cancellation.
    pub latency: Option<Duration>,
}

/// The kind of failure to inject, mapped to a [`PalError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    /// Simulates a transient IO failure.
    Io,
    /// Simulates the resource being held by something else (§5.3 interference).
    Conflict,
    /// Simulates a dropped privilege.
    PermissionDenied,
}

impl FaultKind {
    pub(crate) fn to_error(self, op: &str) -> PalError {
        match self {
            Self::Io => PalError::Io(std::io::Error::other(format!("injected io fault at {op}"))),
            Self::Conflict => PalError::Conflict(format!("injected conflict at {op}")),
            Self::PermissionDenied => {
                PalError::PermissionDenied(format!("injected permission fault at {op}"))
            }
        }
    }
}

/// Holds the current fault plan and the operation counter.
#[derive(Default)]
pub struct FaultInjector {
    plan: Mutex<FaultPlan>,
    op_count: Mutex<u32>,
}

impl FaultInjector {
    /// Replaces the fault plan.
    pub fn arm(&self, plan: FaultPlan) {
        *self.plan.lock().expect("fault plan mutex") = plan;
    }

    /// Arms a failure for operations whose label contains `needle`.
    pub fn fail_ops_matching(&self, needle: impl Into<String>, kind: FaultKind) {
        let mut plan = self.plan.lock().expect("fault plan mutex");
        plan.fail_ops_matching = Some((needle.into(), kind));
    }

    /// Arms a failure of the Nth mutating operation.
    pub fn fail_after_n(&self, n: u32, kind: FaultKind) {
        let mut plan = self.plan.lock().expect("fault plan mutex");
        plan.fail_after_n = Some((n, kind));
    }

    /// Clears all armed faults and resets the counter.
    pub fn clear(&self) {
        *self.plan.lock().expect("fault plan mutex") = FaultPlan::default();
        *self.op_count.lock().expect("op count mutex") = 0;
    }

    /// Consulted by the mock at the start of every operation. Applies latency if
    /// armed and returns an error if this operation should fail.
    pub(crate) async fn check(&self, op: &str) -> Result<(), PalError> {
        let (latency, matching, after_n) = {
            let plan = self.plan.lock().expect("fault plan mutex");
            (
                plan.latency,
                plan.fail_ops_matching.clone(),
                plan.fail_after_n,
            )
        };

        if let Some(d) = latency {
            tokio::time::sleep(d).await;
        }

        let n = {
            let mut count = self.op_count.lock().expect("op count mutex");
            *count += 1;
            *count
        };

        if let Some((needle, kind)) = matching
            && op.contains(&needle)
        {
            return Err(kind.to_error(op));
        }
        if let Some((target, kind)) = after_n
            && n == target
        {
            return Err(kind.to_error(op));
        }
        Ok(())
    }
}

impl std::fmt::Debug for FaultInjector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FaultInjector").finish_non_exhaustive()
    }
}
