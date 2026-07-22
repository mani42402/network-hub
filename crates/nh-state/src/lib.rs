//! # nh-state — the engine's heart
//!
//! The generic machinery that makes NetworkHub robust (AD-4, §5.3–§5.5),
//! independent of any specific feature:
//!
//! - [`store`] — the **desired-state store**: typed, revisioned documents. API
//!   mutations write desired state; reconcilers converge the OS toward it.
//! - [`reconciler`] — the **reconciler framework**: drives convergence with
//!   backoff, a circuit breaker (`Degraded`), and interference detection
//!   (`Conflicted`) so it never fights another agent forever.
//! - [`journal`] — the **intent journal**: write-ahead records of OS mutations
//!   with prior values, so a crash mid-change is recoverable (AD-5, F-3, F-9).
//! - [`event_bus`] — the **domain event bus**: sequenced, with
//!   drop-for-slow-consumers so a stalled client can never back-pressure the
//!   engine (§5.4).
//! - [`supervisor`] — **task supervision**: named tasks with health and restart
//!   policy, backing `debug tasks`.
//!
//! This crate is a leaf framework: it depends on `nh-types` but not on `nh-pal`.
//! Feature crates combine this framework with PAL backends to build concrete
//! reconcilers; the framework itself is tested against toy reconcilers and (in
//! integration tests) against `pal-mock`.

pub mod backoff;
pub mod event_bus;
mod interference;
pub mod journal;
pub mod reconciler;
pub mod store;
pub mod supervisor;

// Most-used items at the crate root.
pub use event_bus::{EventBus, Received, Sequenced, Subscription};
pub use journal::{InMemoryJournal, Intent, IntentState, JournalStore};
pub use reconciler::{
    ReconcileOutcome, Reconciler, ReconcilerConfig, ReconcilerHandle, ReconcilerStatus,
    TriggerReason,
};
pub use store::{StateKey, StateStore, Versioned};
pub use supervisor::{RestartPolicy, Supervisor, TaskHealth};
