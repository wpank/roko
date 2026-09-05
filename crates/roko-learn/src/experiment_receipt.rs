//! `ExperimentReceipt` — crash-durable prompt-experiment receipt for all surfaces.
//!
//! The runner (event_loop.rs) already uses the three-phase protocol:
//!   1. `prepare_attempt_assignments` — idempotent assignment preparation
//!   2. `mark_attempt_dispatched` — record exact prompt hash before provider launch
//!   3. `settle_attempt` — record terminal outcome after durable terminal fact
//!
//! ACP and serve surfaces previously used `record_outcome_for_experiment` which
//! directly mutates variant statistics without the crash-durable bucket protocol.
//! This module provides `ExperimentReceipt`, a handle that wraps the three-phase
//! protocol and can be used from any dispatch surface.
//!
//! # Usage
//!
//! ```rust,ignore
//! // 1. Prepare a receipt from the experiment store.
//! let receipt = ExperimentReceipt::prepare(
//!     &store_path,
//!     &attempt_key,
//!     role,
//!     &["constraints", "style"],
//! )?;
//!
//! // 2. Apply variant content to your prompt sections.
//! for assignment in receipt.assignments() {
//!     // Replace section content...
//! }
//!
//! // 3. Mark dispatched immediately before provider launch.
//! receipt.mark_dispatched(&store_path, "prompt-hash", &included_ids)?;
//!
//! // 4. After terminal event, settle the receipt.
//! receipt.settle(&store_path, settlement)?;
//! ```

use std::path::Path;

use crate::prompt_experiment::{
    AssignmentSettlement, ExperimentStore, PromptAssignmentError, PromptAttemptKey,
    PromptExperimentAssignment,
};

/// A durable prompt-experiment receipt that tracks the three-phase lifecycle
/// (prepare -> dispatch -> settle) for a single attempt across any surface.
///
/// The receipt is constructed by [`Self::prepare`] which calls
/// `ExperimentStore::prepare_attempt_assignments`. It carries the resulting
/// assignments so the caller can apply variant content before dispatch.
///
/// After prompt composition, the caller marks the receipt dispatched
/// ([`Self::mark_dispatched`]) and, after a terminal event, settles it
/// ([`Self::settle`]).
///
/// If the receipt is dropped without being settled, the experiment store
/// retains the prepared bucket so reconciliation can discover and settle it
/// on restart.
#[derive(Debug, Clone)]
pub struct ExperimentReceipt {
    /// The attempt key used to prepare the assignments.
    pub attempt_key: PromptAttemptKey,
    /// The prepared assignments for this attempt.
    assignments: Vec<PromptExperimentAssignment>,
    /// Whether `mark_dispatched` has been called.
    dispatched: bool,
    /// Whether `settle` has been called.
    settled: bool,
}

impl ExperimentReceipt {
    /// Prepare experiment assignments for the given attempt.
    ///
    /// This is a thin wrapper around
    /// [`ExperimentStore::prepare_attempt_assignments`]. If the store does not
    /// exist or no running experiments match the eligible sections, the receipt
    /// will have an empty assignment list (which is valid -- no experiments to
    /// run).
    ///
    /// # Errors
    ///
    /// Returns [`PromptAssignmentError`] on I/O or conflict failures.
    pub fn prepare(
        store_path: &Path,
        attempt_key: &PromptAttemptKey,
        role: Option<&str>,
        eligible_sections: &[&str],
    ) -> Result<Self, PromptAssignmentError> {
        let assignments = if store_path.exists() {
            ExperimentStore::prepare_attempt_assignments(
                store_path,
                attempt_key,
                role,
                eligible_sections,
            )?
        } else {
            Vec::new()
        };

        Ok(Self {
            attempt_key: attempt_key.clone(),
            assignments,
            dispatched: false,
            settled: false,
        })
    }

    /// Construct a receipt without running the prepare phase.
    ///
    /// Useful when the caller has already prepared assignments through another
    /// path (e.g. the runner's existing `PromptAssembler` integration) and just
    /// needs the dispatch/settle handle.
    #[must_use]
    pub fn from_assignments(
        attempt_key: PromptAttemptKey,
        assignments: Vec<PromptExperimentAssignment>,
    ) -> Self {
        Self {
            attempt_key,
            assignments,
            dispatched: false,
            settled: false,
        }
    }

    /// The assignments prepared for this attempt.
    #[must_use]
    pub fn assignments(&self) -> &[PromptExperimentAssignment] {
        &self.assignments
    }

    /// Whether any assignments were prepared.
    #[must_use]
    pub fn has_assignments(&self) -> bool {
        !self.assignments.is_empty()
    }

    /// Assignment IDs that were prepared.
    #[must_use]
    pub fn assignment_ids(&self) -> Vec<String> {
        self.assignments
            .iter()
            .map(|a| a.assignment_id.clone())
            .collect()
    }

    /// Mark this receipt as dispatched immediately before the provider boundary.
    ///
    /// Records the exact prompt hash and which assignment IDs were included
    /// in the final composed prompt. This is the second phase of the
    /// three-phase protocol.
    ///
    /// # Errors
    ///
    /// Returns [`PromptAssignmentError`] on I/O or conflict failures.
    pub fn mark_dispatched(
        &mut self,
        store_path: &Path,
        prompt_hash: &str,
        included_assignment_ids: &[String],
    ) -> Result<(), PromptAssignmentError> {
        if self.assignments.is_empty() {
            return Ok(());
        }
        let included_refs: Vec<&str> = included_assignment_ids.iter().map(String::as_str).collect();
        ExperimentStore::mark_attempt_dispatched(
            store_path,
            &self.attempt_key,
            prompt_hash,
            &included_refs,
        )
        .map(|_| ())?;
        self.dispatched = true;
        Ok(())
    }

    /// Settle this receipt after a terminal event has been durably recorded.
    ///
    /// This is the third and final phase of the protocol. After settlement,
    /// the variant statistics in the experiment store are updated.
    ///
    /// Returns `Ok(true)` if settlement was recorded, `Ok(false)` if no
    /// applicable experiment was found (normal for non-experiment runs).
    ///
    /// # Errors
    ///
    /// Returns [`PromptAssignmentError`] on I/O failures.
    pub fn settle(
        &mut self,
        store_path: &Path,
        settlement: AssignmentSettlement,
    ) -> Result<bool, PromptAssignmentError> {
        if self.assignments.is_empty() {
            return Ok(false);
        }
        if !store_path.exists() {
            return Ok(false);
        }
        let result =
            match ExperimentStore::settle_attempt(store_path, &self.attempt_key, settlement) {
                Ok(_) => Ok(true),
                Err(PromptAssignmentError::AttemptNotFound(_)) => Ok(false),
                Err(err) => Err(err),
            };
        if result.as_ref().is_ok_and(|settled| *settled) {
            self.settled = true;
        }
        result
    }

    /// Abandon this receipt (e.g. after a prompt assembly error before the
    /// provider boundary was crossed).
    ///
    /// Equivalent to `settle(Abandoned)` but communicates intent.
    pub fn abandon(&mut self, store_path: &Path) -> Result<bool, PromptAssignmentError> {
        self.settle(store_path, AssignmentSettlement::Abandoned)
    }

    /// Whether the receipt has been settled.
    #[must_use]
    pub const fn is_settled(&self) -> bool {
        self.settled
    }

    /// Whether the receipt has been marked dispatched.
    #[must_use]
    pub const fn is_dispatched(&self) -> bool {
        self.dispatched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_experiment::{ExperimentStore, PromptExperiment, PromptVariant};

    fn make_variants(section: &str) -> Vec<PromptVariant> {
        vec![
            PromptVariant {
                id: "a".into(),
                name: "A".into(),
                section_name: section.into(),
                content: "Be concise.".into(),
                slug: None,
                active: true,
            },
            PromptVariant {
                id: "b".into(),
                name: "B".into(),
                section_name: section.into(),
                content: "Be thorough.".into(),
                slug: None,
                active: true,
            },
        ]
    }

    fn seed_store(path: &Path) {
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "receipt-exp",
            "constraints",
            make_variants("constraints"),
        ));
        store.save(path).unwrap();
    }

    #[test]
    fn receipt_prepare_with_no_store_produces_empty_assignments() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("nonexistent.json");
        let key = PromptAttemptKey::new("run-1", "plan-1", "task-1", 0);

        let receipt =
            ExperimentReceipt::prepare(&store_path, &key, None, &["constraints"]).unwrap();

        assert!(!receipt.has_assignments());
        assert_eq!(receipt.assignments().len(), 0);
        assert!(!receipt.is_dispatched());
        assert!(!receipt.is_settled());
    }

    #[test]
    fn receipt_full_lifecycle_prepare_dispatch_settle() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("experiments.json");
        seed_store(&store_path);

        let key = PromptAttemptKey::new("run-2", "plan-2", "task-2", 0);
        let mut receipt =
            ExperimentReceipt::prepare(&store_path, &key, None, &["constraints"]).unwrap();

        assert!(receipt.has_assignments());
        let ids = receipt.assignment_ids();
        assert!(!ids.is_empty());

        // Mark dispatched.
        receipt
            .mark_dispatched(&store_path, "prompt-hash-abc", &ids)
            .unwrap();
        assert!(receipt.is_dispatched());

        // Settle as observed success.
        let settled = receipt
            .settle(
                &store_path,
                AssignmentSettlement::Observed { success: true },
            )
            .unwrap();
        assert!(settled);
        assert!(receipt.is_settled());

        // Verify the experiment store was updated.
        let store = ExperimentStore::load_strict(&store_path).unwrap();
        let exp = store.get("receipt-exp").unwrap();
        let total_trials: u64 = exp.stats.values().map(|s| s.trials).sum();
        assert_eq!(total_trials, 1);
    }

    #[test]
    fn receipt_abandon_does_not_count_trial() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("experiments.json");
        seed_store(&store_path);

        let key = PromptAttemptKey::new("run-3", "plan-3", "task-3", 0);
        let mut receipt =
            ExperimentReceipt::prepare(&store_path, &key, None, &["constraints"]).unwrap();
        let ids = receipt.assignment_ids();
        receipt.mark_dispatched(&store_path, "hash", &ids).unwrap();

        // Abandon instead of settling with success/failure.
        let settled = receipt.abandon(&store_path).unwrap();
        assert!(settled);

        let store = ExperimentStore::load_strict(&store_path).unwrap();
        let exp = store.get("receipt-exp").unwrap();
        let total_trials: u64 = exp.stats.values().map(|s| s.trials).sum();
        assert_eq!(total_trials, 0, "abandoned receipts must not count a trial");
    }

    #[test]
    fn receipt_from_assignments_skips_prepare() {
        let key = PromptAttemptKey::new("run-4", "plan-4", "task-4", 0);
        let receipt = ExperimentReceipt::from_assignments(key.clone(), vec![]);

        assert!(!receipt.has_assignments());
        assert_eq!(receipt.attempt_key, key);
    }

    #[test]
    fn receipt_settle_with_missing_store_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("missing.json");

        // Prepare from a real store, then delete it to simulate the missing-store path.
        let real_store_path = dir.path().join("real.json");
        seed_store(&real_store_path);
        let key = PromptAttemptKey::new("run-5", "plan-5", "task-5", 0);
        let mut receipt =
            ExperimentReceipt::prepare(&real_store_path, &key, None, &["constraints"]).unwrap();
        assert!(receipt.has_assignments());

        // Now settle against the missing store path.
        let result = receipt
            .settle(
                &store_path,
                AssignmentSettlement::Observed { success: true },
            )
            .unwrap();
        assert!(!result, "settling against a missing store returns false");
    }
}
