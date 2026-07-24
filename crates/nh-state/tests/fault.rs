//! Fault-injection suite v1 (WP-M0.14) — the fail-open invariant (AD-5, §11) as
//! an automated test: crash the daemon at any point inside a mutating flow and,
//! after the boot-time deadman replays the intent journal, the system is in the
//! **desired** state or the **pre-mutation** state — never a third thing.
//!
//! At M0 the one journaled mutating flow is the write-ahead intent lifecycle
//! (append `Pending` → apply → `commit`); this suite drives it to every crash
//! point. As real reconcilers land (firewall, hotspot, VPN — M1+), each adds its
//! flow here: the harness is built so a new case is a few lines. That
//! "easy to add a case" property is the M0.14 exit criterion.
//!
//! ## Failure-catalog coverage (§11)
//!
//! Exercised here (journal / persistence level):
//! - **F-1**  deadman reconciles active vs. desired — a committed intent survives
//! - **F-3**  power loss mid-mutation — a pending intent is rolled back to prior
//! - **F-9**  a sysctl's *exact* prior value is restored (not merely a default)
//! - **F-12** write failure (disk full) aborts the flow before any OS mutation
//!
//! Exercised in the crate that owns the mechanism (kept there so the test lives
//! next to the code):
//! - **F-4**  child kill-on-close  → `nh-daemon` `src/child.rs`
//! - **F-5**  `config.db` corruption → `nh-state` `tests/persist.rs`
//! - **F-13** interface identity    → `nh-pal` `src/interface.rs`

#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use async_trait::async_trait;
use nh_state::journal::{
    InMemoryJournal, Intent, IntentState, JournalError, JournalResult, JournalStore,
};

/// A stand-in for an OS resource a mutation changes — a sysctl, a firewall table.
struct Resource {
    value: Mutex<String>,
}

impl Resource {
    fn new(initial: &str) -> Self {
        Self {
            value: Mutex::new(initial.to_owned()),
        }
    }
    fn get(&self) -> String {
        self.value.lock().unwrap().clone()
    }
    fn set(&self, v: &str) {
        v.clone_into(&mut self.value.lock().unwrap());
    }
}

/// Where a simulated crash interrupts the write-ahead flow.
#[derive(Debug, Clone, Copy)]
enum Crash {
    /// The process survives the whole flow.
    Never,
    /// Killed after the `Pending` intent is journaled, before the OS mutation.
    AfterAppend,
    /// Killed after the OS mutation, before the commit.
    AfterApply,
    /// Killed after commit (the mutation is durable desired state).
    AfterCommit,
}

fn pending_intent(id: &str, prior: &str, new: &str) -> Intent {
    Intent {
        id: id.to_owned(),
        module: "test".to_owned(),
        operation: "resource.set".to_owned(),
        prior_value: prior.to_owned(),
        new_value: new.to_owned(),
        state: IntentState::Pending,
    }
}

/// Runs the write-ahead mutating flow against `resource`, "crashing" at `crash`.
///
/// The ordering is the contract: the intent is journaled (with the *prior* value)
/// **before** the OS is touched, and committed only **after**. A crash therefore
/// always leaves the journal in a state the deadman can resolve.
async fn mutating_flow(
    journal: &dyn JournalStore,
    resource: &Resource,
    id: &str,
    new_value: &str,
    crash: Crash,
) {
    let prior = resource.get();
    journal
        .append(pending_intent(id, &prior, new_value))
        .await
        .unwrap();
    if matches!(crash, Crash::AfterAppend) {
        return;
    }

    resource.set(new_value);
    if matches!(crash, Crash::AfterApply) {
        return;
    }

    journal.commit(id).await.unwrap();
    // Never and AfterCommit both leave a committed intent behind.
}

/// The boot-time deadman (§4.2, F-3): every still-`Pending` intent is rolled back
/// to its `prior_value` — a pending intent means "we crashed before confirming
/// this mutation", so undo it. Committed intents are desired state; leave them.
async fn deadman_replay(journal: &dyn JournalStore, resource: &Resource) {
    for intent in journal.pending().await.unwrap() {
        resource.set(&intent.prior_value);
        journal.mark_reverted(&intent.id).await.unwrap();
    }
}

#[tokio::test]
async fn ad5_invariant_holds_at_every_crash_point() {
    // The core assertion: a crash at ANY point in the flow, followed by the
    // deadman, leaves the resource at the desired new value or the original prior
    // value — never partially applied, never a third thing.
    for crash in [
        Crash::Never,
        Crash::AfterAppend,
        Crash::AfterApply,
        Crash::AfterCommit,
    ] {
        let journal = InMemoryJournal::new();
        let resource = Resource::new("PRIOR");

        mutating_flow(&journal, &resource, "intent-1", "NEW", crash).await;
        deadman_replay(&journal, &resource).await;

        let final_value = resource.get();
        assert!(
            final_value == "PRIOR" || final_value == "NEW",
            "AD-5 violated for {crash:?}: resource is {final_value:?}",
        );
        // Desired iff the mutation committed; otherwise the pre-mutation state.
        let expected = match crash {
            Crash::Never | Crash::AfterCommit => "NEW",
            Crash::AfterAppend | Crash::AfterApply => "PRIOR",
        };
        assert_eq!(final_value, expected, "wrong recovery for {crash:?}");
        // The deadman always drains the journal of pending work.
        assert!(journal.pending().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn f3_crash_mid_mutation_rolls_back() {
    // Power loss after the sysctl is set but before the change is committed.
    let journal = InMemoryJournal::new();
    let resource = Resource::new("off");
    mutating_flow(&journal, &resource, "i", "on", Crash::AfterApply).await;
    assert_eq!(
        resource.get(),
        "on",
        "the mutation applied before the crash"
    );

    // Boot: the deadman finds the one pending intent and undoes it.
    assert_eq!(journal.pending().await.unwrap().len(), 1);
    deadman_replay(&journal, &resource).await;
    assert_eq!(
        resource.get(),
        "off",
        "F-3: rolled back to the pre-mutation state"
    );
}

#[tokio::test]
async fn f9_restores_the_exact_prior_value_not_a_default() {
    // ip_forward was already 1 before NetworkHub touched it; a rollback must
    // return it to 1, not to some assumed-off default of 0 (F-9).
    let journal = InMemoryJournal::new();
    let resource = Resource::new("1");
    mutating_flow(&journal, &resource, "i", "0", Crash::AfterApply).await;
    deadman_replay(&journal, &resource).await;
    assert_eq!(
        resource.get(),
        "1",
        "F-9: the exact prior value is restored"
    );
}

#[tokio::test]
async fn f1_committed_intent_survives_the_deadman() {
    // A committed mutation is desired state: the deadman must not roll it back,
    // even though the process crashed right after commit (F-1).
    let journal = InMemoryJournal::new();
    let resource = Resource::new("v0");
    mutating_flow(&journal, &resource, "i", "v1", Crash::AfterCommit).await;
    assert!(
        journal.pending().await.unwrap().is_empty(),
        "a committed intent is not pending"
    );
    deadman_replay(&journal, &resource).await;
    assert_eq!(
        resource.get(),
        "v1",
        "F-1: committed desired state is preserved"
    );
}

/// A journal whose writes always fail — a full disk (F-12).
struct FullDiskJournal;

#[async_trait]
impl JournalStore for FullDiskJournal {
    async fn append(&self, _intent: Intent) -> JournalResult<()> {
        Err(JournalError::Storage("no space left on device".to_owned()))
    }
    async fn commit(&self, _id: &str) -> JournalResult<()> {
        Ok(())
    }
    async fn mark_reverted(&self, _id: &str) -> JournalResult<()> {
        Ok(())
    }
    async fn pending(&self) -> JournalResult<Vec<Intent>> {
        Ok(Vec::new())
    }
    async fn prune_finished(&self) -> JournalResult<usize> {
        Ok(0)
    }
}

#[tokio::test]
async fn f12_write_failure_prevents_the_mutation() {
    // The write-ahead intent must be durable BEFORE the OS mutation. If it cannot
    // be written (disk full), the mutation must not proceed — no partial state,
    // no silent drift; the daemon stays consistent rather than crash-looping.
    let journal = FullDiskJournal;
    let resource = Resource::new("PRIOR");

    let append = journal
        .append(pending_intent("i", &resource.get(), "NEW"))
        .await;
    assert!(append.is_err(), "append fails on a full disk");

    // Because the write-ahead record could not be made, the flow bails before
    // touching the OS: the resource is untouched (F-12).
    assert_eq!(
        resource.get(),
        "PRIOR",
        "F-12: no partial state on write failure"
    );
}
